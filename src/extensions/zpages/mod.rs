mod statsz;

use std::net::SocketAddr;
use std::str::FromStr;

use async_trait::async_trait;
use bytes::Bytes;
use configurable::configurable_component;
use framework::config::{ExtensionConfig, ExtensionContext};
use framework::Extension;
use http::header::CONTENT_TYPE;
use http::{Method, Request, Response, StatusCode};
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper_util::rt::{TokioExecutor, TokioIo};
pub use statsz::{Metric, Point, Statsz};
use tokio::net::TcpListener;

fn default_endpoint() -> SocketAddr {
    SocketAddr::from_str("127.0.0.1:56888").expect("default endpoint parse ok")
}

/// Enables an extension that serves zPages, an HTTP endpoint that provides
/// live data for debugging different components that were properly instrumented for such.
///
/// https://opencensus.io/zpages/
#[configurable_component(extension, name = "zpages")]
#[serde(deny_unknown_fields)]
struct Config {
    #[serde(default = "default_endpoint")]
    #[configurable(required)]
    endpoint: SocketAddr,
}

#[async_trait]
#[typetag::serde(name = "zpages")]
impl ExtensionConfig for Config {
    async fn build(&self, cx: ExtensionContext) -> framework::Result<Extension> {
        let mut shutdown = cx.shutdown;
        let listener = TcpListener::bind(self.endpoint).await?;

        Ok(Box::pin(async move {
            loop {
                let conn = tokio::select! {
                    _ = &mut shutdown => break,
                    result = listener.accept() => match result {
                        Ok((conn, _peer)) => TokioIo::new(conn),
                        Err(err) => {
                            error!(
                                message = "accept new connection failed",
                                %err
                            );

                            continue
                        }
                    }
                };

                let mut shutdown = shutdown.clone();
                tokio::spawn(async move {
                    let builder =
                        hyper_util::server::conn::auto::Builder::new(TokioExecutor::new());
                    let conn =
                        builder.serve_connection_with_upgrades(conn, service_fn(http_handle));
                    tokio::pin!(conn);

                    loop {
                        tokio::select! {
                            result = conn.as_mut() => {
                                if let Err(err) = result {
                                    error!(message = "handle http connection failed", %err);
                                }

                                break
                            },
                            _ = &mut shutdown => {
                                conn.as_mut().graceful_shutdown();
                            }
                        }
                    }
                });
            }

            Ok(())
        }))
    }
}

async fn http_handle(req: Request<Incoming>) -> framework::Result<Response<Full<Bytes>>> {
    if req.method() != Method::GET {
        let resp = Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Full::default())
            .expect("build ok");

        return Ok(resp);
    }

    match req.uri().path() {
        "/statsz" => {
            let stats = Statsz::snapshot();
            let data = serde_json::to_vec(&stats.metrics)?;

            let resp = Response::builder()
                .header(CONTENT_TYPE, "application/json")
                .status(StatusCode::OK)
                .body(Full::new(data.into()))?;

            Ok(resp)
        }
        _ => {
            let resp = Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::default())
                .expect("build ok");

            Ok(resp)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }
}
