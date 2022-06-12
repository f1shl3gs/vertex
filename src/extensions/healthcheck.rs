use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};

use framework::config::{ExtensionConfig, ExtensionContext, ExtensionDescription, GenerateConfig};
use framework::Extension;
use futures_util::FutureExt;
use http::{Request, Response, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Server};
use serde::{Deserialize, Serialize};

static READINESS: AtomicBool = AtomicBool::new(false);

pub fn set_readiness(ready: bool) {
    READINESS.store(ready, Ordering::Relaxed)
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
    fn generate_config() -> String {
        "endpoint: 0.0.0.0:13133".into()
    }
}

inventory::submit! {
    ExtensionDescription::new::<HealthcheckConfig>("healthcheck")
}

#[async_trait::async_trait]
#[typetag::serde(name = "healthcheck")]
impl ExtensionConfig for HealthcheckConfig {
    async fn build(&self, cx: ExtensionContext) -> crate::Result<Extension> {
        info!(
            message = "start healthcheck server",
            endpoint = ?self.endpoint
        );

        let shutdown = cx.shutdown;
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

    let (status, body) = match req.uri().path() {
        "/-/healthy" => (StatusCode::OK, "Vertex is Healthy.\n"),
        "/-/ready" => {
            if READINESS.load(Ordering::Relaxed) {
                (StatusCode::OK, "Vertex is Ready.\n")
            } else {
                (StatusCode::SERVICE_UNAVAILABLE, "Vertex is not ready.\n")
            }
        }
        _ => (
            StatusCode::NOT_FOUND,
            "Only \"/-/healthy\" and \"/-/ready\" allowed",
        ),
    };

    Ok(Response::builder()
        .status(status)
        .body(Body::from(body))
        .unwrap())
}
