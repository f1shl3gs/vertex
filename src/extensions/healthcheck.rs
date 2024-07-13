use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};

use bytes::Bytes;
use configurable::configurable_component;
use framework::config::{ExtensionConfig, ExtensionContext};
use framework::Extension;
use http::{Request, Response, StatusCode};
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use tokio::net::TcpListener;

static READINESS: AtomicBool = AtomicBool::new(false);

pub fn set_readiness(ready: bool) {
    READINESS.store(ready, Ordering::Relaxed)
}

fn default_endpoint() -> SocketAddr {
    SocketAddr::from(([0, 0, 0, 0], 13133))
}

#[configurable_component(extension, name = "healthcheck")]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Which address to listen for.
    #[serde(default = "default_endpoint")]
    #[configurable(required)]
    pub endpoint: SocketAddr,
}

#[async_trait::async_trait]
#[typetag::serde(name = "healthcheck")]
impl ExtensionConfig for Config {
    async fn build(&self, cx: ExtensionContext) -> crate::Result<Extension> {
        let mut shutdown = cx.shutdown;
        let listener = TcpListener::bind(self.endpoint).await?;

        Ok(Box::pin(async move {
            info!(
                message = "start healthcheck server",
                listen = ?listener.local_addr().unwrap()
            );

            loop {
                let conn = tokio::select! {
                    _ = &mut shutdown => break,
                    result = listener.accept() => match result {
                        Ok((conn, peer)) => {
                            debug!(
                                message = "accept new connection",
                                ?peer
                            );

                            hyper_util::rt::TokioIo::new(conn)
                        },
                        Err(err) => {
                            error!(
                                message = "accept connection failed",
                                %err
                            );

                            continue
                        }
                    }
                };

                tokio::spawn(async move {
                    if let Err(err) = http1::Builder::new()
                        .serve_connection(conn, service_fn(handle))
                        .await
                    {
                        error!(message = "handle http connection failed", ?err);
                    }
                });
            }

            Ok(())
        }))
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
        .body(Full::new(Bytes::from_static(body.as_bytes())))
        .expect("should build response"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }
}
