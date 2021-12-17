use std::collections::{BTreeMap, HashMap};

use bytes::Buf;
use http::StatusCode;
use hyper::Body;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};

use crate::config::{deserialize_std_duration, serialize_std_duration};
use crate::http::HttpClient;

#[derive(Debug, Snafu)]
pub enum ConsulError {
    #[snafu(display("Parse url failed, {}", source))]
    ParseUrl { source: url::ParseError },
    #[snafu(display("Build request failed, {}", source))]
    BuildRequest { source: http::Error },
    #[snafu(display("Read response body failed, {}", source))]
    ReadBody { source: hyper::Error },
    #[snafu(display("Do http request failed, {}", source))]
    HttpErr { source: crate::http::HttpError },
    #[snafu(display("Decode response failed, {}", source))]
    DecodeError { source: serde_json::Error },
    #[snafu(display("Unexpected status {}", code))]
    UnexpectedStatusCode { code: u16 },
}

// Not all field included, only the field we need
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Node {
    #[serde(rename = "ID")]
    pub id: String,
    pub address: String,
    pub node: String,
}

// Not all field included, only the field we need
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct AgentMember {
    pub name: String,
    pub status: f64,
    pub addr: String,
}

// Not all field included, only the field we need
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct HealthCheck {
    pub name: String,
    pub service_name: String,
    pub status: String,
    #[serde(rename = "ServiceID")]
    pub service_id: String,
    #[serde(rename = "CheckID")]
    pub check_id: String,
    pub node: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Service {
    #[serde(rename = "ID")]
    pub id: String,
    pub tags: Vec<String>,
    pub service: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ServiceEntry {
    pub node: Node,
    pub service: Service,
    pub checks: Vec<HealthCheck>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QueryOptions {
    // Namespace overrides the `default` namespace
    // Note: Namespaces are available only in Consul Enterprise
    pub namespace: String,

    // Providing a datacenter overwrites the DC provided
    // by the Config
    pub datacenter: String,

    // AllowStale allows any Consul server (non-leader) to service
    // a read. This allows for lower latency and higher throughput
    pub allow_stale: bool,

    // RequireConsistent forces the read to be fully consistent.
    // This is more expensive but prevents ever performing a stale
    // read.
    pub require_consistent: bool,

    // UseCache requests that the agent cache results locally. See
    // https://www.consul.io/api/features/caching.html for more details on the
    // semantics.
    pub use_cache: bool,

    // MaxAge limits how old a cached value will be returned if UseCache is true.
    // If there is a cached response that is older than the MaxAge, it is treated
    // as a cache miss and a new fetch invoked. If the fetch fails, the error is
    // returned. Clients that wish to allow for stale results on error can set
    // StaleIfError to a longer duration to change this behavior. It is ignored
    // if the endpoint supports background refresh caching. See
    // https://www.consul.io/api/features/caching.html for more details.
    #[serde(
        deserialize_with = "deserialize_std_duration",
        serialize_with = "serialize_std_duration"
    )]
    pub max_age: std::time::Duration,

    // StaleIfError specifies how stale the client will accept a cached response
    // if the servers are unavailable to fetch a fresh one. Only makes sense when
    // UseCache is true and MaxAge is set to a lower, non-zero value. It is
    // ignored if the endpoint supports background refresh caching. See
    // https://www.consul.io/api/features/caching.html for more details.
    #[serde(
        deserialize_with = "deserialize_std_duration",
        serialize_with = "serialize_std_duration"
    )]
    pub stale_if_error: std::time::Duration,

    // WaitIndex is used to enable a blocking query. Waits
    // until the timeout or the next index is reached
    pub wait_index: u64,

    // WaitHash is used by some endpoints instead of WaitIndex to perform blocking
    // on state based on a hash of the response rather than a monotonic index.
    // This is required when the state being blocked on is not stored in Raft, for
    // example agent-local proxy configuration.
    pub wait_hash: String,

    // WaitTime is used to bound the duration of a wait.
    // Defaults to that of the Config, but can be overridden.
    #[serde(
        deserialize_with = "deserialize_std_duration",
        serialize_with = "serialize_std_duration"
    )]
    pub wait_time: std::time::Duration,

    // Token is used to provide a per-request ACL token
    // which overrides the agent's default token.
    pub token: String,

    // Near is used to provide a node name that will sort the results
    // in ascending order based on the estimated round trip time from
    // that node. Setting this to "_agent" will use the agent's node
    // for the sort.
    pub near: String,

    // NodeMeta is used to filter results by nodes with the given
    // metadata key/value pairs. Currently, only one key/value pair can
    // be provided for filtering.
    pub node_meta: HashMap<String, String>,

    // RelayFactor is used in keyring operations to cause responses to be
    // relayed back to the sender through N other random nodes. Must be
    // a value from 0 to 5 (inclusive).
    pub relay_factor: u8,

    // LocalOnly is used in keyring list operation to force the keyring
    // query to only hit local servers (no WAN traffic).
    pub local_only: bool,

    // Connect filters prepared query execution to only include Connect-capable
    // services. This currently affects prepared query execution.
    pub connect: bool,

    // Filter requests filtering data prior to it being returned. The string
    // is a go-bexpr compatible expression.
    pub filter: String,
}

impl QueryOptions {
    // TODO: less to_string() and to_owned()
    fn builder(&self, path: &str) -> Result<http::request::Builder, ConsulError> {
        let mut builder = http::request::Builder::new();
        let mut headers = builder.headers_mut();
        let mut params = Vec::with_capacity(16);

        if self.namespace != "" {
            params.push(("ns", self.namespace.to_owned()));
        }
        if self.datacenter != "" {
            params.push(("dc", self.datacenter.to_owned()));
        }
        if self.allow_stale {
            params.push(("stale", "".to_owned()));
        }
        if self.require_consistent {
            params.push(("consistent", "".to_owned()));
        }
        if self.wait_index != 0 {
            let n = self.wait_index.to_string();
            params.push(("index", n));
        }
        if !self.wait_time.is_zero() {
            let ms = self.wait_time.as_millis().to_string() + "ms";
            params.push(("wait", ms));
        }
        if self.token != "" {
            builder = builder.header("X-Consul-Token", self.token.to_owned());
        }
        if self.near != "" {
            params.push(("near", self.near.to_owned()));
        }
        if self.filter != "" {
            params.push(("filter", self.filter.to_owned()));
        }
        if !self.node_meta.is_empty() {
            for (key, value) in &self.node_meta {
                params.push(("node-meta", format!("{}:{}", key, value)));
            }
        }
        if self.relay_factor != 0 {
            params.push(("relay-factor", self.relay_factor.to_string()));
        }
        if self.local_only {
            params.push(("local-only", "true".to_string()))
        }
        if self.connect {
            params.push(("connect", "true".to_string()));
        }

        if self.use_cache && !self.require_consistent {
            params.push(("cached", "".to_string()));

            let mut cc = vec![];
            if !self.max_age.is_zero() {
                cc.push(format!("max-age={}", self.max_age.as_secs_f64()))
            }

            if !self.stale_if_error.is_zero() {
                cc.push(format!(
                    "stale-if-error={}",
                    self.stale_if_error.as_secs_f64()
                ))
            }

            if cc.len() > 0 {
                let value = cc.join(",");
                builder = builder.header("Cache-Control", value);
            }
        }

        let uri = url::Url::parse_with_params(path, params).context(ParseUrl)?;

        Ok(builder.uri(uri.as_str()))
    }
}

pub struct Client {
    client: HttpClient,
    pub endpoint: String,
}

impl Client {
    pub fn new(endpoint: String, client: HttpClient) -> Self {
        Self { endpoint, client }
    }

    pub async fn peers(&self) -> Result<Vec<String>, ConsulError> {
        self.fetch("/v1/status/peers", None).await
    }

    pub async fn leader(&self) -> Result<String, ConsulError> {
        self.fetch("/v1/status/leader", None).await
    }

    pub async fn nodes(&self, opts: Option<QueryOptions>) -> Result<Vec<Node>, ConsulError> {
        self.fetch("/v1/catalog/nodes", None).await
    }

    pub async fn members(&self, wan: bool) -> Result<Vec<AgentMember>, ConsulError> {
        self.fetch("/v1/agent/members", None).await
    }

    pub async fn services(
        &self,
        opts: Option<QueryOptions>,
    ) -> Result<BTreeMap<String, Vec<String>>, ConsulError> {
        self.fetch("/v1/catalog/services", opts).await
    }

    // `service` is used to query health information along with service info for a given service.
    // It can optionally do server-side filtering on a tag or nodes with passing health checks only.
    pub async fn service(
        &self,
        name: &str,
        tag: &str,
        opts: Option<QueryOptions>,
    ) -> Result<Vec<ServiceEntry>, ConsulError> {
        let uri = format!("/v1/health/service/{}", name);
        self.fetch(uri.as_str(), opts).await
    }

    pub async fn health_state(
        &self,
        opts: Option<QueryOptions>,
    ) -> Result<Vec<HealthCheck>, ConsulError> {
        self.fetch("/v1/health/state/any", opts).await
    }

    async fn fetch<T>(&self, path: &str, opts: Option<QueryOptions>) -> Result<T, ConsulError>
    where
        T: serde::de::DeserializeOwned,
    {
        let path = format!("{}{}", self.endpoint, path);
        let builder = match opts {
            Some(opts) => opts.builder(&path)?,
            None => http::Request::get(path),
        };

        let req = builder.body(Body::empty()).context(BuildRequest)?;

        return match self.client.send(req).await {
            Ok(resp) => {
                let (parts, body) = resp.into_parts();
                match parts.status {
                    StatusCode::OK => {
                        let body = hyper::body::to_bytes(body).await.context(ReadBody)?;

                        let body =
                            serde_json::from_slice::<T>(body.chunk()).context(DecodeError)?;

                        Ok(body)
                    }
                    status => Err(ConsulError::UnexpectedStatusCode {
                        code: status.as_u16(),
                    }),
                }
            }
            Err(err) => Err(ConsulError::HttpErr { source: err }),
        };
    }
}

#[cfg(all(test, feature = "integration-tests-consul"))]
mod integration_tests {
    use super::*;
    use crate::config::ProxyConfig;
    use crate::http::HttpClient;
    use crate::tls::MaybeTlsSettings;
    use testcontainers::images::generic::{GenericImage, WaitFor};
    use testcontainers::Docker;

    #[tokio::test]
    async fn start_local_service() {
        let docker = testcontainers::clients::Cli::default();
        let image = GenericImage::new("consul:1.11.1").with_wait_for(WaitFor::LogMessage {
            message: "Synced node info".to_string(),
            stream: testcontainers::images::generic::Stream::StdOut,
        });
        let service = docker.run(image);
        let host_port = service.get_host_port(8500).unwrap();

        let tls = MaybeTlsSettings::client_config(&None).unwrap();
        let client = HttpClient::new(tls, &ProxyConfig::default()).unwrap();
        let endpoint = format!("http://127.0.0.1:{}", host_port);
        let client = Client::new(endpoint, client);

        let peers = client.peers().await.unwrap();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0], "127.0.0.1:8300".to_string());

        let leader = client.leader().await.unwrap();
        assert_eq!(leader, "127.0.0.1:8300".to_string());

        let nodes = client.nodes(None).await.unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].address, "127.0.0.1");

        let members = client.members(false).await.unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].status, 1.0);
        assert_eq!(members[0].addr, "127.0.0.1".to_string());

        let services = client.services(None).await.unwrap();
        assert_eq!(services.len(), 1);
        assert_eq!(services.get("consul").unwrap().len(), 0);

        let entries = client.service("consul", "", None).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].node.address, "127.0.0.1".to_string());
        assert_eq!(entries[0].service.service, "consul".to_string());

        let health_states = client.health_state(None).await.unwrap();
        assert_eq!(health_states.len(), 1);
        assert_eq!(health_states[0].name, "Serf Health Status".to_string());
        assert_eq!(health_states[0].status, "passing".to_string());
    }
}
