use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};

use bytes::Bytes;
use configurable::configurable_component;
use framework::Extension;
use framework::config::{ExtensionConfig, ExtensionContext, Resource};
use framework::tls::MaybeTlsListener;
use http::{Request, Response, StatusCode};
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::service::service_fn;

static READINESS: AtomicBool = AtomicBool::new(false);

pub fn set_readiness(ready: bool) {
    READINESS.store(ready, Ordering::Relaxed)
}

fn default_endpoint() -> SocketAddr {
    SocketAddr::from(([0, 0, 0, 0], 13133))
}

#[configurable_component(extension, name = "healthcheck")]
#[serde(deny_unknown_fields)]
struct Config {
    /// Which address to listen for.
    #[serde(default = "default_endpoint")]
    #[configurable(required)]
    endpoint: SocketAddr,
}

#[async_trait::async_trait]
#[typetag::serde(name = "healthcheck")]
impl ExtensionConfig for Config {
    async fn build(&self, cx: ExtensionContext) -> crate::Result<Extension> {
        let shutdown = cx.shutdown;
        let listener = MaybeTlsListener::bind(&self.endpoint, None).await?;

        Ok(Box::pin(async move {
            info!(
                message = "start healthcheck server",
                listen = ?listener.local_addr().unwrap()
            );

            framework::http::serve(listener, service_fn(handle))
                .with_graceful_shutdown(shutdown)
                .await
                .map_err(|_err| ())
        }))
    }

    fn resources(&self) -> Vec<Resource> {
        vec![Resource::tcp(self.endpoint)]
    }
}

async fn handle(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, hyper::Error> {
    if req.method() != http::Method::GET {
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Full::default())
            .expect("should build not allowed response"));
    }

    let (status, body) = match req.uri().path() {
        "/healthy" => (StatusCode::OK, "Vertex is Healthy.\n"),
        "/ready" => {
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
        .body(Full::new(Bytes::from_static(body.as_bytes())))
        .expect("should build response"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }
}
