mod grpc;
mod http;

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};

use configurable::{Configurable, configurable_component};
use framework::Extension;
use framework::config::{ExtensionConfig, ExtensionContext, Resource};
use framework::http::Auth;
use framework::tls::TlsConfig;
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;

static READINESS: AtomicBool = AtomicBool::new(false);

pub fn set_readiness(ready: bool) {
    READINESS.store(ready, Ordering::Relaxed)
}

fn default_http_endpoint() -> SocketAddr {
    SocketAddr::from(([0, 0, 0, 0], 13133))
}

fn default_grpc_endpoint() -> SocketAddr {
    SocketAddr::from(([0, 0, 0, 0], 13132))
}

/// The HTTP service provide two endpoint
///
/// - `/healthy` for `liveness` check
/// - `/ready` for `readiness` check
#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
struct HttpConfig {
    /// Which address the HTTP server listen to
    #[serde(default = "default_http_endpoint")]
    listen: SocketAddr,

    tls: Option<TlsConfig>,

    auth: Option<Auth>,
}

/// The GRPC service expose `Check` and `Watch` to retrieve or watch the
/// serving status of Vertex
#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
struct GrpcConfig {
    /// Which address the GRPC server listen to
    #[serde(default = "default_grpc_endpoint")]
    listen: SocketAddr,
}

#[configurable_component(extension, name = "healthcheck")]
struct Config {
    http: Option<HttpConfig>,
    grpc: Option<GrpcConfig>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "healthcheck")]
impl ExtensionConfig for Config {
    async fn build(&self, cx: ExtensionContext) -> crate::Result<Extension> {
        if self.http.is_none() && self.grpc.is_none() {
            return Err("`http` and `grpc` must be set at least".into());
        }

        let http = self.http.clone();
        let grpc = self.grpc.clone();
        let shutdown = cx.shutdown;

        Ok(Box::pin(async move {
            let mut tasks = JoinSet::new();

            if let Some(http) = http {
                let shutdown = shutdown.clone();
                tasks.spawn(async move {
                    if let Err(err) =
                        http::serve(http.listen, http.tls.as_ref(), http.auth, shutdown).await
                    {
                        warn!(message = "HTTP server error", ?err);
                    }
                });
            }

            if let Some(grpc) = grpc {
                tasks.spawn(async move {
                    if let Err(err) = grpc::serve(grpc.listen, shutdown).await {
                        warn!(message = "GRPC server error", ?err);
                    }
                });
            }

            while (tasks.join_next().await).is_some() {}

            Ok(())
        }))
    }

    fn resources(&self) -> Vec<Resource> {
        let mut resources = Vec::with_capacity(2);

        if let Some(http) = &self.http {
            resources.push(Resource::tcp(http.listen));
        }

        if let Some(grpc) = &self.grpc {
            resources.push(Resource::tcp(grpc.listen));
        }

        resources
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }
}
