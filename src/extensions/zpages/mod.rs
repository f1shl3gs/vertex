mod statsz;

use std::net::{Ipv4Addr, SocketAddr};

use bytes::Bytes;
use configurable::configurable_component;
use framework::Extension;
use framework::config::{ExtensionConfig, ExtensionContext, Resource};
use framework::observe::current_endpoints;
use framework::tls::MaybeTlsListener;
use http::header::CONTENT_TYPE;
use http::{Method, Request, Response, StatusCode};
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::service::service_fn;
use parking_lot::RwLock;
use statsz::Statsz;

static CURRENT_CONFIG: RwLock<String> = RwLock::new(String::new());

pub fn update_config(config: &framework::config::Config) {
    let content = serde_yaml::to_string(config).unwrap();
    *CURRENT_CONFIG.write() = content;
}

fn default_endpoint() -> SocketAddr {
    SocketAddr::from((Ipv4Addr::UNSPECIFIED, 56888))
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

#[async_trait::async_trait]
#[typetag::serde(name = "zpages")]
impl ExtensionConfig for Config {
    async fn build(&self, cx: ExtensionContext) -> framework::Result<Extension> {
        let shutdown = cx.shutdown;
        let listener = MaybeTlsListener::bind(&self.endpoint, None).await?;

        Ok(Box::pin(async move {
            framework::http::serve(listener, service_fn(http_handle))
                .with_graceful_shutdown(shutdown)
                .await
                .map_err(|_err| ())
        }))
    }

    fn resources(&self) -> Vec<Resource> {
        vec![Resource::tcp(self.endpoint)]
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

    let resp = match req.uri().path() {
        "/statsz" => {
            let stats = Statsz::snapshot();
            let data = serde_json::to_vec(&stats.metrics)?;

            Response::builder()
                .header(CONTENT_TYPE, "application/json")
                .status(StatusCode::OK)
                .body(Full::new(data.into()))?
        }
        "/config" => {
            let text = CURRENT_CONFIG.read().to_string();

            Response::builder()
                .header(CONTENT_TYPE, "text/yaml")
                .status(StatusCode::OK)
                .body(Full::new(text.into()))?
        }
        "/observe" => {
            let endpoints = current_endpoints();

            let body = serde_json::to_vec(&endpoints)?;

            Response::builder()
                .header(CONTENT_TYPE, "application/json")
                .status(StatusCode::OK)
                .body(Full::new(body))?
        }
        "/" => {
            let text = r##"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Zpages</title>
</head>
<body>
<p><a href="/config">config</a></p>
<p><a href="/statsz">statsz</a></p>
</body>
</html>
"##;
            Response::builder()
                .header(CONTENT_TYPE, "text/html")
                .status(StatusCode::OK)
                .body(Full::new(text.into()))?
        }
        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::default())
            .expect("build ok"),
    };

    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }
}
