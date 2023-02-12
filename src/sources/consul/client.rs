use std::collections::{BTreeMap, HashMap};

use bytes::Buf;
use configurable::Configurable;
use framework::config::default_true;
use framework::http::HttpClient;
use http::StatusCode;
use hyper::Body;
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConsulError {
    #[error("Parse url failed, err: {0}")]
    ParseUrl(#[from] url::ParseError),
    #[error("Build request failed, {0}")]
    BuildRequest(#[from] http::Error),
    #[error("Read response body failed, {0}")]
    ReadBody(hyper::Error),
    #[error("Do http request failed, {0}")]
    HttpErr(framework::http::HttpError),
    #[error("Decode response failed, {0}")]
    Decode(#[from] serde_json::Error),
    #[error("Unexpected status {0}")]
    UnexpectedStatusCode(u16),
    #[error("Redirection to {0}")]
    NeedRedirection(String),
    #[error("Redirection failed")]
    RedirectionFailed,
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

#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QueryOptions {
    /// Namespace overrides the `default` namespace.
    ///
    /// Note: Namespaces are available only in Consul Enterprise
    pub namespace: String,

    /// Providing a datacenter overwrites the DC provided
    /// by the Config
    pub datacenter: String,

    /// AllowStale allows any Consul server (non-leader) to service
    /// a read. This allows for lower latency and higher throughput
    #[serde(default = "default_true")]
    pub allow_stale: bool,

    /// RequireConsistent forces the read to be fully consistent.
    /// This is more expensive but prevents ever performing a stale
    /// read.
    #[serde(default)]
    pub require_consistent: bool,

    /// UseCache requests that the agent cache results locally. See
    /// https://www.consul.io/api/features/caching.html for more details on the
    /// semantics.
    pub use_cache: bool,

    /// MaxAge limits how old a cached value will be returned if UseCache is true.
    /// If there is a cached response that is older than the MaxAge, it is treated
    /// as a cache miss and a new fetch invoked. If the fetch fails, the error is
    /// returned. Clients that wish to allow for stale results on error can set
    /// StaleIfError to a longer duration to change this behavior. It is ignored
    /// if the endpoint supports background refresh caching. See
    /// https://www.consul.io/api/features/caching.html for more details.
    #[serde(with = "humanize::duration::serde")]
    pub max_age: std::time::Duration,

    /// StaleIfError specifies how stale the client will accept a cached response
    /// if the servers are unavailable to fetch a fresh one. Only makes sense when
    /// UseCache is true and MaxAge is set to a lower, non-zero value. It is
    /// ignored if the endpoint supports background refresh caching. See
    /// https://www.consul.io/api/features/caching.html for more details.
    #[serde(with = "humanize::duration::serde")]
    pub stale_if_error: std::time::Duration,

    /// WaitIndex is used to enable a blocking query. Waits
    /// until the timeout or the next index is reached
    pub wait_index: u64,

    /// WaitHash is used by some endpoints instead of WaitIndex to perform blocking
    /// on state based on a hash of the response rather than a monotonic index.
    /// This is required when the state being blocked on is not stored in Raft, for
    /// example agent-local proxy configuration.
    pub wait_hash: String,

    /// WaitTime is used to bound the duration of a wait.
    /// Defaults to that of the Config, but can be overridden.
    #[serde(with = "humanize::duration::serde")]
    pub wait_time: std::time::Duration,

    /// Token is used to provide a per-request ACL token
    /// which overrides the agent's default token.
    pub token: String,

    /// Near is used to provide a node name that will sort the results
    /// in ascending order based on the estimated round trip time from
    /// that node. Setting this to "_agent" will use the agent's node
    /// for the sort.
    pub near: String,

    /// NodeMeta is used to filter results by nodes with the given
    /// metadata key/value pairs. Currently, only one key/value pair can
    /// be provided for filtering.
    pub node_meta: HashMap<String, String>,

    /// RelayFactor is used in keyring operations to cause responses to be
    /// relayed back to the sender through N other random nodes. Must be
    /// a value from 0 to 5 (inclusive).
    pub relay_factor: u8,

    /// LocalOnly is used in keyring list operation to force the keyring
    /// query to only hit local servers (no WAN traffic).
    pub local_only: bool,

    /// Connect filters prepared query execution to only include Connect-capable
    /// services. This currently affects prepared query execution.
    pub connect: bool,

    /// Filter requests filtering data prior to it being returned. The string
    /// is a go-bexpr compatible expression.
    pub filter: String,
}

impl QueryOptions {
    // TODO: less to_string() and to_owned()
    fn builder(&self, path: &str) -> Result<http::request::Builder, ConsulError> {
        let mut builder = http::request::Builder::new();
        let mut params = Vec::with_capacity(16);

        if !self.namespace.is_empty() {
            params.push(("ns", self.namespace.to_owned()));
        }
        if !self.datacenter.is_empty() {
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
        if !self.token.is_empty() {
            builder = builder.header("X-Consul-Token", self.token.to_owned());
        }
        if !self.near.is_empty() {
            params.push(("near", self.near.to_owned()));
        }
        if !self.filter.is_empty() {
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

            if !cc.is_empty() {
                let value = cc.join(",");
                builder = builder.header("Cache-Control", value);
            }
        }

        let uri = url::Url::parse_with_params(path, params)?;

        Ok(builder.uri(uri.as_str()))
    }
}

pub struct Client {
    client: HttpClient,
    pub endpoint: String,
}

impl Client {
    pub const fn new(endpoint: String, client: HttpClient) -> Self {
        Self { endpoint, client }
    }

    pub async fn peers(&self) -> Result<Vec<String>, ConsulError> {
        self.fetch("/v1/status/peers", &None).await
    }

    pub async fn leader(&self) -> Result<String, ConsulError> {
        self.fetch("/v1/status/leader", &None).await
    }

    pub async fn nodes(&self, opts: &Option<QueryOptions>) -> Result<Vec<Node>, ConsulError> {
        self.fetch("/v1/catalog/nodes", opts).await
    }

    pub async fn members(&self) -> Result<Vec<AgentMember>, ConsulError> {
        self.fetch("/v1/agent/members", &None).await
    }

    pub async fn services(
        &self,
        opts: &Option<QueryOptions>,
    ) -> Result<BTreeMap<String, Vec<String>>, ConsulError> {
        self.fetch("/v1/catalog/services", opts).await
    }

    // `service` is used to query health information along with service info for a given service.
    // It can optionally do server-side filtering on a tag or nodes with passing health checks only.
    pub async fn service(
        &self,
        name: &str,
        opts: &Option<QueryOptions>,
    ) -> Result<Vec<ServiceEntry>, ConsulError> {
        let name = percent_encode(name.as_bytes(), NON_ALPHANUMERIC).to_string();
        let uri = format!("/v1/health/service/{}", name);
        match self.fetch(uri.as_str(), opts).await {
            Ok(entries) => Ok(entries),
            Err(err) => match err {
                ConsulError::NeedRedirection(to) => self.fetch(&to, opts).await,
                _ => Err(err),
            },
        }
    }

    pub async fn health_state(
        &self,
        opts: &Option<QueryOptions>,
    ) -> Result<Vec<HealthCheck>, ConsulError> {
        self.fetch("/v1/health/state/any", opts).await
    }

    async fn fetch<T>(&self, path: &str, opts: &Option<QueryOptions>) -> Result<T, ConsulError>
    where
        T: serde::de::DeserializeOwned,
    {
        let path = format!("{}{}", self.endpoint, path);
        let builder = match opts {
            Some(opts) => opts.builder(&path)?,
            None => http::Request::get(path),
        };

        let req = builder.body(Body::empty())?;

        return match self.client.send(req).await {
            Ok(resp) => {
                let (parts, body) = resp.into_parts();
                match parts.status {
                    StatusCode::OK => {
                        let body = hyper::body::to_bytes(body)
                            .await
                            .map_err(ConsulError::ReadBody)?;

                        let body = serde_json::from_slice::<T>(body.chunk())?;

                        Ok(body)
                    }
                    StatusCode::MOVED_PERMANENTLY => {
                        return match parts.headers.get("Location") {
                            Some(redirect) => Err(ConsulError::NeedRedirection(
                                redirect.to_str().unwrap().to_string(),
                            )),
                            None => Err(ConsulError::RedirectionFailed),
                        };
                    }
                    status => Err(ConsulError::UnexpectedStatusCode(status.as_u16())),
                }
            }
            Err(err) => Err(ConsulError::HttpErr(err)),
        };
    }
}
