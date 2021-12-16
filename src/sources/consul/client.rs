use bytes::Buf;
use http::{Request, StatusCode, Uri};
use hyper::Body;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use std::collections::{BTreeMap, HashMap};
use url::Url;

use crate::config::{deserialize_std_duration, serialize_std_duration};
use crate::http::HttpClient;

#[derive(Debug, Snafu)]
pub enum ConsulError {
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
pub struct ConsulNode {
    pub address: String,
}

// Not all field included, only the field we need
#[derive(Debug, Deserialize, Serialize)]
pub struct AgentMember {
    pub name: String,
    pub status: f64,
}

// Not all field included, only the field we need
#[derive(Debug, Deserialize, Serialize)]
pub struct HealthCheck {
    status: String,
    service_id: String,
    check_id: String,
    node: String,
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
    fn apply<B>(&self, req: &mut Request<B>) {
        let mut params = vec![];

        if self.namespace != "" {
            params.push(("ns", self.namespace.as_str()));
        }
        if self.datacenter != "" {
            params.push(("dc", self.datacenter.as_str()));
        }
        if self.allow_stale {
            params.push(("stale", ""));
        }
        if self.require_consistent {
            params.push(("consistent", ""));
        }
        if self.wait_index != 0 {
            params.push(("index", self.wait_index.to_string().as_str()));
        }
        if !self.wait_time.is_zero() {
            let ms = self.wait_time.as_millis().to_string() + "ms";
            params.push(("wait", ms.as_str()));
        }
        if self.token != "" {
            req.headers_mut()
                .insert("X-Consul-Token", self.token.parse().unwrap());
        }
        if self.near != "" {
            params.push(("near", self.near.as_str()));
        }
        if self.filter != "" {
            params.push(("filter", self.filter.as_str()));
        }
        if !self.node_meta.is_empty() {
            for (key, value) in self.node_meta {
                params.push(("node-meta", format!("{}:{}", key, value).as_str()));
            }
        }
        if self.relay_factor != 0 {
            params.push(("relay-factor", self.relay_factor.to_string().as_str()));
        }
        if self.local_only {
            params.push(("local-only", "true"))
        }
        if self.connect {
            params.push(("connect", "true"));
        }

        if self.use_cache && !self.require_consistent {
            params.push(("cached", ""));

            let mut cc = vec![];
            if !self.max_age.is_zero() {
                cc.push(format!("max-age={}", self.max_age.as_secs_f64()))
            }

            if !self.stale_if_error.is_zero() {
                cc.push(format!("stale-if-error={}", self.stale_if_error.as_secs_f64()))
            }

            if cc.len() > 0 {
                req.headers_mut()
                    .insert("Cache-Control", cc.join(",").as_str().parse().unwrap())
            }
        }

        let n = Url::parse_with_params(uri, params);
    }
}

pub struct Client {
    client: HttpClient,
    endpoint: String,
}

impl Client {
    pub fn new(endpoint: String, client: HttpClient) -> Self {
        Self { endpoint, client }
    }

    pub async fn peers(&self) -> Result<Vec<String>, ConsulError> {
        self.fetch("/v1/status/peers").await
    }

    pub async fn leader(&self) -> Result<String, ConsulError> {
        self.fetch("/v1/status/leader").await
    }

    pub async fn nodes(&self, opts: &QueryOptions) -> Result<Vec<ConsulNode>, ConsulError> {
        self.fetch("/v1/catalog/nodes").await
    }

    pub async fn members(&self, wan: bool) -> Result<Vec<AgentMember>, ConsulError> {
        self.fetch("/v1/agent/members").await
    }

    // TODO: collect health summary!?
    pub async fn services(
        &self,
        opts: &QueryOptions,
    ) -> Result<BTreeMap<String, Vec<String>>, ConsulError> {
        self.fetch("/v1/catalog/services").await
    }

    pub async fn health_state(&self, opts: &QueryOptions) -> Result<Vec<HealthCheck>, ConsulError> {
        self.fetch("/v1/health/state/any").await
    }

    async fn fetch<T>(&self, path: &str) -> Result<T, ConsulError>
    where
        T: serde::de::DeserializeOwned,
    {
        let uri = format!("{}{}", self.endpoint, path);
        let req = http::Request::get(uri)
            .body(Body::empty())
            .context(BuildRequest)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProxyConfig;
    use crate::tls::MaybeTlsSettings;

    #[tokio::test]
    async fn test_fetch() {
        let tls = MaybeTlsSettings::client_config(&None).unwrap();
        let client = HttpClient::new(tls, &ProxyConfig::default()).unwrap();
        let endpoint = "http://127.0.0.1:8500".to_string();

        let client = Client::new(endpoint, client);

        let peers = client.peers().await.unwrap();
        println!("peers: {:?}", peers);
    }
}

#[cfg(all(test, feature = "integration-tests-consul"))]
mod integration_tests {
    use testcontainers::images::generic::{GenericImage, WaitFor};
    use testcontainers::Docker;

    #[test]
    #[ignore]
    fn start_local_service() {
        let docker = testcontainers::clients::Cli::default();
        let image = GenericImage::new("consul:1.11.1").with_wait_for(WaitFor::LogMessage {
            message: "Synced node info".to_string(),
            stream: testcontainers::Stream::StdOut,
        });
        let service = docker.run(image);
        let host_port = service.get_host_port(8500).unwrap();
    }
}
