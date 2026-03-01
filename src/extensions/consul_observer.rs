use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use configurable::configurable_component;
use framework::config::{ExtensionConfig, ExtensionContext};
use framework::http::{Auth, HttpClient, HttpError};
use framework::observe::{Endpoint, Observer};
use framework::tls::TlsConfig;
use framework::{Extension, ShutdownSignal};
use http::response::Parts;
use http::{Method, Request, Uri};
use http_body_util::{BodyExt, Full};
use serde::Deserialize;
use tripwire::Tripwire;
use value::value;

const WAIT_TIMEOUT: Duration = Duration::from_secs(2 * 60);

const fn default_refresh_interval() -> Duration {
    Duration::from_secs(30)
}

const fn default_allow_stale() -> bool {
    true
}

/// Consul service discovery allows retrieving services from Catalog API
///
/// ```yaml
/// id: 1234
/// target: 127.0.0.1:8080
/// details:
///   node:
///     addr: 127.0.0.1
///     port: 8080
///   service:
///     address: 127.0.0.1
///     port: 9090
///     id: blah
///     tags:
///     - foo
///     - bar
/// ```
#[configurable_component(extension, name = "consul_observer")]
struct Config {
    /// The Endpoint to access to the Consul API
    #[serde(with = "framework::config::http::uri")]
    endpoint: Uri,

    tls: Option<TlsConfig>,

    auth: Option<Auth>,

    /// Namespaces are only supported in Consul Enterprise
    namespace: Option<String>,
    /// Admin partitions are only supported in Consul Enterprise.
    partition: Option<String>,

    datacenter: Option<String>,

    /// A list of services for which targets are retrieved. If omitted, all
    /// services are watched.
    #[serde(default)]
    services: Vec<String>,

    /// A Consul Filter expression used to filter the catalog results
    /// See https://www.consul.io/api-docs/catalog#list-services to known
    /// more about the filter expressions that can be used
    filter: Option<String>,

    /// Allow stale Consul results, which reduce load on Consul
    ///
    /// See https://www.consul.io/api/features/consistency.html
    #[serde(default = "default_allow_stale")]
    allow_stale: bool,

    /// The time after which the provided names are refreshed.
    /// On large setup it might be a good idea to increase this value because
    /// the catalog will change all the time.
    #[serde(
        with = "humanize::duration::serde",
        default = "default_refresh_interval"
    )]
    refresh_interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "consul_observer")]
impl ExtensionConfig for Config {
    async fn build(&self, cx: ExtensionContext) -> crate::Result<Extension> {
        let http_client = HttpClient::new(self.tls.as_ref(), &cx.proxy)?;
        let observer = Observer::register(cx.key);
        let auth = self.auth.clone();
        let endpoint = match (self.endpoint.scheme(), self.endpoint.authority()) {
            (Some(scheme), Some(authority)) => {
                format!("{}://{}", scheme.as_ref(), authority.as_str())
            }
            (None, Some(authority)) => format!("http://{}", authority.as_str()),
            _ => unreachable!(),
        };

        let client = Client {
            http_client,
            endpoint,
            auth,
            filter: self.filter.clone(),
            datacenter: self.datacenter.clone(),
            namespace: self.namespace.clone(),
            partition: self.partition.clone(),
            allow_stale: self.allow_stale,
            last_index: Arc::new(Default::default()),
        };

        Ok(Box::pin(watch(
            client,
            self.refresh_interval,
            self.services.clone(),
            observer,
            cx.shutdown,
        )))
    }
}

async fn watch(
    client: Client,
    interval: Duration,
    services: Vec<String>,
    observer: Observer,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let mut ticker = tokio::time::interval(interval);
    let mut initial_delay = Some(Duration::from_secs(5));
    let mut existing = BTreeMap::new();
    let grouped_endpoints = Arc::new(Mutex::new(BTreeMap::<String, Vec<Endpoint>>::new()));
    let start_watch = std::sync::Once::new();

    loop {
        tokio::select! {
            _ = ticker.tick() => {}
            _ = &mut shutdown => break,
        }

        if services.is_empty() {
            match client.services().await {
                Ok(services) => {
                    for service in &services {
                        if existing.contains_key(service) {
                            // already running
                            continue;
                        }

                        let (trigger, tripwire) = Tripwire::new();
                        existing.insert(service.clone(), trigger);

                        tokio::spawn(watch_service(
                            client.clone(),
                            service.clone(),
                            interval,
                            tripwire,
                            Arc::clone(&grouped_endpoints),
                        ));
                    }

                    // remove services, which is not exists anymore
                    existing.retain(|name, _trigger| {
                        // NOTE: we don't need to call trigger.cancel, cause it will
                        // be dropped after this closure.
                        services.contains(name)
                    });
                }
                Err(err) => {
                    warn!(message = "list services failed", ?err);
                    continue;
                }
            };
        } else {
            start_watch.call_once(|| {
                for service in &services {
                    let (trigger, tripwire) = Tripwire::new();
                    existing.insert(service.to_string(), trigger);

                    tokio::spawn(watch_service(
                        client.clone(),
                        service.clone(),
                        interval,
                        tripwire,
                        Arc::clone(&grouped_endpoints),
                    ));
                }
            });
        }

        if let Some(delay) = initial_delay.take() {
            tokio::select! {
                _ = tokio::time::sleep(delay) => {},
                _ = &mut shutdown => break,
            }
        }

        let endpoints = grouped_endpoints
            .lock()
            .unwrap()
            .values()
            .flat_map(|endpoints| endpoints.clone())
            .collect::<Vec<_>>();

        if let Err(_err) = observer.publish(endpoints) {
            break;
        }
    }

    for (_service, trigger) in existing {
        trigger.cancel();
    }

    Ok(())
}

async fn watch_service(
    client: Client,
    service: String,
    interval: Duration,
    mut tripwire: Tripwire,
    cache: Arc<Mutex<BTreeMap<String, Vec<Endpoint>>>>,
) -> Result<(), ()> {
    debug!(message = "start watching service", service);

    let mut ticker = tokio::time::interval(interval);

    // prepare the service entry, so later insert will not need to clone
    // the service key.
    cache.lock().unwrap().insert(service.clone(), Vec::new());

    loop {
        tokio::select! {
            _ = ticker.tick() => {}
            _ = &mut tripwire => break
        }

        tokio::select! {
            _ = &mut tripwire => break,
            result = client.service_entries(&service) => match result {
                Ok(entries) => {
                    let endpoints = entries.into_iter().map(build_endpoint).collect::<Vec<_>>();

                    if let Some(dst) = cache.lock().unwrap().get_mut(&service) {
                        *dst = endpoints;
                    }
                },
                Err(err) => {
                    warn!(message = "list service entries failed", ?err, service);
                    continue;
                }
            }
        }
    }

    debug!(message = "service watch routine finished", service);

    Ok(())
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    Http(#[from] HttpError),
    #[error("deserialize response failed, {0}")]
    Deserialize(#[from] serde_json::Error),
    #[error("unexpected status code {0}")]
    UnexpectedStatus(http::StatusCode),
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Node {
    #[serde(rename = "ID")]
    id: String,
    node: String,
    address: String,
    #[serde(rename = "Datacenter")]
    data_center: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct AgentService {
    #[serde(rename = "ID")]
    id: String,
    tags: Vec<String>,
    port: u16,
    address: String,
    namespace: Option<String>,
    partition: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct HealthChecks {
    // node: String,
    #[serde(rename = "CheckID")]
    check_id: String,
    status: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ServiceEntry {
    node: Node,
    service: AgentService,
    checks: Vec<HealthChecks>,
}

/// AggregatedStatus returns the "best" status for the list of health checks.
/// Because a given entry may have many service and node-level health checks
/// attached, this function determines the best representative of the status
/// as single string using the following heuristic:
///
/// maintenance > critical > warning > passing
fn aggregated_status(checks: &[HealthChecks]) -> String {
    let mut passing = false;
    let mut warning = false;
    let mut critical = false;
    let mut maintenance = false;

    for check in checks {
        // `_node_maintenance` is the special key set by a node in maintenance mode
        // `_service_maintenance` is the prefix for a service in maintenance mode
        if check.check_id == "_node_maintenance"
            || check.check_id.starts_with("_service_maintenance")
        {
            maintenance = true;
            continue;
        }

        if check.status == "passing" {
            passing = true;
        } else if check.status == "warning" {
            warning = true;
        } else if check.status == "critical" {
            critical = true;
        } else {
            return "".to_owned();
        }
    }

    if maintenance {
        "maintenance".to_owned()
    } else if critical {
        "critical".to_owned()
    } else if warning {
        "warning".to_owned()
    } else if passing {
        "passing".to_owned()
    } else {
        "passing".to_string()
    }
}

#[derive(Clone)]
struct Client {
    http_client: HttpClient,

    endpoint: String,
    auth: Option<Auth>,
    filter: Option<String>,
    datacenter: Option<String>,
    namespace: Option<String>,
    partition: Option<String>,
    allow_stale: bool,
    last_index: Arc<AtomicU64>,
}

impl Client {
    async fn services(&self) -> Result<Vec<String>, Error> {
        let mut req = Request::builder()
            .method(Method::GET)
            .uri(format!(
                "{}/v1/catalog/services?{}",
                self.endpoint,
                self.build_query()
            ))
            .body(Full::default())
            .unwrap();

        if let Some(auth) = self.auth.as_ref() {
            auth.apply(&mut req);
        }

        let resp = self.http_client.send(req).await?;
        let (parts, incoming) = resp.into_parts();
        if !parts.status.is_success() {
            return Err(Error::UnexpectedStatus(parts.status));
        }

        let body = incoming
            .collect()
            .await
            .map_err(HttpError::from)?
            .to_bytes();

        let services = serde_json::from_slice::<BTreeMap<String, Vec<String>>>(&body)?;

        Ok(services.keys().cloned().collect())
    }

    async fn service_entries(&self, name: &str) -> Result<Vec<ServiceEntry>, Error> {
        let mut req = Request::builder()
            .method(Method::GET)
            .uri(format!(
                "{}/v1/health/service/{}?{}",
                self.endpoint,
                name,
                self.build_query()
            ))
            .body(Full::default())
            .unwrap();

        if let Some(auth) = self.auth.as_ref() {
            auth.apply(&mut req);
        }

        let resp = self.http_client.send(req).await?;
        let (parts, incoming) = resp.into_parts();
        if !parts.status.is_success() {
            return Err(Error::UnexpectedStatus(parts.status));
        }

        self.set_last_index(parts);

        let body = incoming
            .collect()
            .await
            .map_err(HttpError::from)?
            .to_bytes();

        serde_json::from_slice::<Vec<ServiceEntry>>(&body).map_err(Into::into)
    }

    fn set_last_index(&self, parts: Parts) {
        if let Some(value) = parts.headers.get("X-Consul-Index")
            && let Ok(value) = value.to_str()
        {
            match value.parse::<u64>() {
                Ok(last_index) => {
                    self.last_index.store(last_index, Ordering::Relaxed);
                }
                Err(err) => {
                    warn!(message = "parse last index failed", ?err)
                }
            }
        }
    }

    fn build_query(&self) -> String {
        let mut builder = url::form_urlencoded::Serializer::new(String::new());

        if let Some(datacenter) = self.datacenter.as_ref() {
            builder.append_pair("dc", datacenter.as_str());
        }
        if let Some(namespace) = self.namespace.as_ref() {
            builder.append_pair("ns", namespace);
        }
        if let Some(partition) = self.partition.as_ref() {
            builder.append_pair("partition", partition);
        }
        if let Some(filter) = self.filter.as_ref() {
            builder.append_pair("filter", filter);
        }

        let last_index = self.last_index.load(Ordering::Relaxed);
        if last_index != 0 {
            builder.append_pair("index", last_index.to_string().as_ref());
        }
        if self.allow_stale {
            builder.append_pair("stale", "");
        }

        builder.append_pair("wait", format!("{}ms", WAIT_TIMEOUT.as_millis()).as_str());

        builder.finish()
    }
}

fn build_endpoint(entry: ServiceEntry) -> Endpoint {
    // node id might be empty
    let id = if entry.node.id.is_empty() {
        format!("{}_{}", entry.node.node, entry.node.address)
    } else {
        entry.node.id
    };

    // if the service address is not empty it should be used instead of the node
    // address since the service may be registered remotely through a different node.
    let target = if entry.service.address.is_empty() {
        format!("{}:{}", entry.node.address, entry.service.port)
    } else {
        format!("{}:{}", entry.service.address, entry.service.port)
    };

    let mut node = BTreeMap::new();
    node.insert("address".to_string(), entry.node.address.into());
    node.insert("node".to_string(), entry.node.node.into());
    node.insert("data_center".to_string(), entry.node.data_center.into());

    let mut service = BTreeMap::new();
    service.insert("id".to_string(), entry.service.id.into());
    service.insert("port".to_string(), entry.service.port.into());
    service.insert(
        "namespace".to_string(),
        entry.service.namespace.unwrap_or_default().into(),
    );
    service.insert(
        "partition".to_string(),
        entry.service.partition.unwrap_or_default().into(),
    );
    service.insert("tags".to_string(), entry.service.tags.into());

    let health = aggregated_status(&entry.checks);

    Endpoint {
        id,
        typ: "node".into(),
        target,
        details: value!({
            "node": node,
            "service": service,
            "health": health,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }

    #[test]
    fn deserialize() {
        let input = r##"[
    {
        "Node": {
            "ID": "7ffbe9eb-af71-d28e-d4e0-d70a2679b174",
            "Node": "4e361f566be5",
            "Address": "127.0.0.1",
            "Datacenter": "dc1",
            "TaggedAddresses": {
                "lan": "127.0.0.1",
                "lan_ipv4": "127.0.0.1",
                "wan": "127.0.0.1",
                "wan_ipv4": "127.0.0.1"
            },
            "Meta": {
                "consul-network-segment": ""
            },
            "CreateIndex": 13,
            "ModifyIndex": 15
        },
        "Service": {
            "ID": "consul",
            "Service": "consul",
            "Tags": [],
            "Address": "",
            "Meta": {
                "grpc_port": "8502",
                "grpc_tls_port": "8503",
                "non_voter": "false",
                "raft_version": "3",
                "read_replica": "false",
                "serf_protocol_current": "2",
                "serf_protocol_max": "5",
                "serf_protocol_min": "1",
                "version": "1.15.4"
            },
            "Port": 8300,
            "Weights": {
                "Passing": 1,
                "Warning": 1
            },
            "EnableTagOverride": false,
            "Proxy": {
                "Mode": "",
                "MeshGateway": {},
                "Expose": {}
            },
            "Connect": {},
            "PeerName": "",
            "CreateIndex": 13,
            "ModifyIndex": 13
        },
        "Checks": [
            {
                "Node": "4e361f566be5",
                "CheckID": "serfHealth",
                "Name": "Serf Health Status",
                "Status": "passing",
                "Notes": "",
                "Output": "Agent alive and reachable",
                "ServiceID": "",
                "ServiceName": "",
                "ServiceTags": [],
                "Type": "",
                "Interval": "",
                "Timeout": "",
                "ExposedPort": 0,
                "Definition": {},
                "CreateIndex": 13,
                "ModifyIndex": 13
            }
        ]
    }
]
"##;
        let _entries = serde_json::from_str::<Vec<ServiceEntry>>(input).unwrap();
    }
}
