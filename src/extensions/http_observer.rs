use std::collections::BTreeMap;
use std::time::Duration;

use bytes::Bytes;
use configurable::configurable_component;
use framework::Extension;
use framework::config::default_interval;
use framework::config::{ExtensionConfig, ExtensionContext};
use framework::http::{Auth, HttpClient};
use framework::observe::{Observer, run};
use framework::tls::TlsConfig;
use http::{Method, Request, Uri};
use http_body_util::{BodyExt, Full};

/// HTTP-based service discovery provides a more generic way to configure endpoints
/// as an interface to plug in custom service discovery mechanisms.
///
/// This extension fetches endpoints from an HTTP endpoint containing a list of zero or
/// more endpoints. The endpoint must reply with an HTTP 200 response. The HTTP header
/// json, and the body must be valid JSON.
///
/// ## TODO
///
/// Something like this kind of HTTP endpoint probably provided already, but the response
/// content is not what we want. So we can transform the response to the Endpoints, that's
/// what we want.
#[configurable_component(extension, name = "http_observer")]
struct Config {
    #[serde(with = "framework::config::http::uri")]
    endpoint: Uri,

    tls: Option<TlsConfig>,

    auth: Option<Auth>,

    #[serde(default)]
    headers: BTreeMap<String, String>,

    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "http_observer")]
impl ExtensionConfig for Config {
    async fn build(&self, cx: ExtensionContext) -> crate::Result<Extension> {
        let client = HttpClient::new(self.tls.as_ref(), &cx.proxy)?;
        let observer = Observer::register(cx.key);
        let endpoint = self.endpoint.clone();
        let auth = self.auth.clone();
        let headers = self.headers.clone();

        Ok(Box::pin(run(
            observer,
            self.interval,
            cx.shutdown,
            async move || {
                let mut builder = Request::builder().method(Method::GET).uri(&endpoint);
                for (key, value) in &headers {
                    builder = builder.header(key.as_str(), value.as_str());
                }

                let mut req = builder.body(Full::<Bytes>::default()).unwrap();
                if let Some(auth) = &auth {
                    auth.apply(&mut req);
                }

                let resp = client.send(req).await?;
                let (parts, incoming) = resp.into_parts();
                if !parts.status.is_success() {
                    return Err(format!("unexpected status code {}", parts.status).into());
                }

                let body = incoming.collect().await?.to_bytes();

                serde_json::from_slice(&body).map_err(Into::into)
            },
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }
}
