mod statsz;

use std::convert::Infallible;
use std::net::SocketAddr;
use std::str::FromStr;

use async_trait::async_trait;
use configurable::configurable_component;
use framework::config::{ExtensionConfig, ExtensionContext};
use framework::Extension;
use futures_util::FutureExt;
use http::{Method, Request, Response, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Server};

pub use statsz::{Metric, Point, Statsz};

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
        let shutdown = cx.shutdown;
        let addr = self.endpoint;

        Ok(Box::pin(async move {
            let service =
                make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(http_handle)) });

            if let Err(err) = Server::bind(&addr)
                .serve(service)
                .with_graceful_shutdown(shutdown.map(|_token| {
                    info!("zpages done");
                }))
                .await
            {
                warn!(message = "http server error", ?err);
            }

            Ok(())
        }))
    }
}

async fn http_handle(req: Request<Body>) -> framework::Result<Response<Body>> {
    if req.method() != Method::GET {
        let resp = Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Body::empty())
            .expect("build ok");

        return Ok(resp);
    }

    match req.uri().path() {
        "/statsz" => {
            let stats = Statsz::snapshot();
            let data = serde_json::to_vec(&stats.metrics)?;

            let resp = Response::builder()
                .header("content-type", "application/json")
                .status(StatusCode::OK)
                .body(Body::from(data))?;

            Ok(resp)
        }
        _ => {
            let resp = Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
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
