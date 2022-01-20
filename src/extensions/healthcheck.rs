use futures_util::FutureExt;
use std::convert::Infallible;
use std::net::SocketAddr;

use http::{Request, Response, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Server};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};

use crate::config::{ExtensionConfig, ExtensionContext, ExtensionDescription, GenerateConfig};
use crate::extensions::Extension;

static READINESS: OnceCell<bool> = OnceCell::new();

pub fn set_readiness(ready: bool) {
    READINESS.set(ready).expect("Set READINESS success");
}

fn default_endpoint() -> SocketAddr {
    "0.0.0.0:13133".parse().unwrap()
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HealthcheckConfig {
    #[serde(default = "default_endpoint")]
    pub endpoint: SocketAddr,
}

impl GenerateConfig for HealthcheckConfig {
    fn generate_config() -> serde_yaml::Value {
        serde_yaml::to_value(Self {
            endpoint: default_endpoint(),
        })
        .unwrap()
    }
}

inventory::submit! {
    ExtensionDescription::new::<HealthcheckConfig>("healthcheck")
}

#[async_trait::async_trait]
#[typetag::serde(name = "healthcheck")]
impl ExtensionConfig for HealthcheckConfig {
    async fn build(&self, ctx: ExtensionContext) -> crate::Result<Extension> {
        info!(
            message = "start healthcheck server",
            endpoint = ?self.endpoint
        );

        let shutdown = ctx.shutdown;
        let service = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });
        let server = Server::bind(&self.endpoint)
            .serve(service)
            .with_graceful_shutdown(shutdown.map(|_| ()));

        Ok(Box::pin(async move {
            if let Err(err) = server.await {
                error!(message = "Error serving", ?err);
                return Err(());
            }

            Ok(())
        }))
    }

    fn extension_type(&self) -> &'static str {
        "healthcheck"
    }
}

async fn handle(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    if req.method() != http::Method::GET {
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Body::empty())
            .unwrap());
    }

    Ok(match req.uri().path() {
        "/-/healthy" => Response::new(Body::from("Vertex is Healthy.\n")),
        "/-/ready" => {
            let readiness = READINESS.get_or_init(|| false);

            if *readiness {
                Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from("Vertex is Ready.\n"))
                    .unwrap()
            } else {
                Response::builder()
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .body(Body::from("Vertex is not ready.\n"))
                    .unwrap()
            }
        }
        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap(),
    })
}
