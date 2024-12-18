mod statsz;

use std::net::SocketAddr;
use std::str::FromStr;

use async_trait::async_trait;
use bytes::Bytes;
use configurable::configurable_component;
use framework::config::{ExtensionConfig, ExtensionContext};
use framework::tls::MaybeTlsListener;
use framework::Extension;
use http::header::CONTENT_TYPE;
use http::{Method, Request, Response, StatusCode};
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::service::service_fn;
use statsz::Statsz;

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
    endpoint: SocketAddr,
}

#[async_trait]
#[typetag::serde(name = "zpages")]
impl ExtensionConfig for Config {
    async fn build(&self, cx: ExtensionContext) -> framework::Result<Extension> {
        let shutdown = cx.shutdown;
        let listener = MaybeTlsListener::bind(&self.endpoint, &None).await?;

        Ok(Box::pin(async move {
            framework::http::serve(listener, service_fn(http_handle))
                .with_graceful_shutdown(shutdown)
                .await
                .map_err(|_err| ())
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
