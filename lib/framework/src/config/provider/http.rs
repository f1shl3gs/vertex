use std::path::PathBuf;
use std::time::Duration;

use async_stream::stream;
use bytes::Buf;
use futures::Stream;
use hyper::Body;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use sysinfo::unix::{kernel_version, machine_id, os_version};
use url::Url;

use crate::config::{
    default_interval, provider::ProviderConfig, Builder, GenerateConfig, ProviderDescription,
    ProxyConfig,
};
use crate::http::HttpClient;
use crate::signal;
use crate::tls::{TlsConfig, TlsSettings};
use crate::SignalHandler;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RequestConfig {
    #[serde(default)]
    pub headers: IndexMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct HttpConfig {
    url: Option<Url>,
    request: RequestConfig,
    #[serde(with = "humanize::duration::serde")]
    interval: Duration,
    tls: Option<TlsConfig>,
    proxy: ProxyConfig,
    #[serde(default)]
    persist: Option<PathBuf>,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            url: None,
            request: RequestConfig::default(),
            interval: Duration::from_secs(30),
            tls: None,
            proxy: Default::default(),
            persist: None,
        }
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "http")]
impl ProviderConfig for HttpConfig {
    async fn build(&mut self, signal_handler: &mut SignalHandler) -> Result<Builder, Vec<String>> {
        let url = self
            .url
            .take()
            .ok_or_else(|| vec!["URL is required for http provider".to_owned()])?;

        let tls_options = self.tls.take();
        let poll_interval = self.interval;
        let request = self.request.clone();
        let proxy = ProxyConfig::from_env().merge(&self.proxy);
        let attrs = build_attributes();
        let builder =
            http_request_to_config_builder(&url, &tls_options, &request.headers, &proxy, attrs)
                .await?;

        // Poll for changes to remote configuration
        signal_handler.add(poll_http(
            poll_interval,
            url,
            tls_options,
            request.headers,
            proxy.clone(),
        ));

        Ok(builder)
    }

    fn provider_type(&self) -> &'static str {
        "http"
    }
}

inventory::submit! {
    ProviderDescription::new::<HttpConfig>("http")
}

impl GenerateConfig for HttpConfig {
    fn generate_config() -> String {
        format!(
            r#"
# The URL to download config
#
url: http://config.example.com/config

# The interval between fetch config.
#
# interval: {}

# Configures the TLS options for outgoing connections.
#
# tls:
{}

# Configures an HTTP/HTTPS proxy for Vertex to use. By default, the globally
# configured proxy is used.
#
# proxy:
{}

#

        "#,
            humanize::duration::duration(&default_interval()),
            TlsConfig::generate_commented_with_indent(2),
            ProxyConfig::generate_commented_with_indent(2)
        )
    }
}

/// Calls `http_request`, serializing the result to a `ConfigBuilder`.
async fn http_request_to_config_builder(
    url: &Url,
    tls_options: &Option<TlsConfig>,
    headers: &IndexMap<String, String>,
    proxy: &ProxyConfig,
    attrs: IndexMap<String, String>,
) -> Result<crate::config::Builder, Vec<String>> {
    let config_str = http_request(url, tls_options, headers, proxy, attrs)
        .await
        .map_err(|err| vec![err.to_owned()])?;

    let (builder, warnings) = crate::config::load(config_str.chunk(), None)?;
    for warning in warnings.into_iter() {
        warn!("{}", warning);
    }

    Ok(builder)
}

/// Makes an HTTP request to the provided endpoint, returning the String body.
async fn http_request(
    url: &Url,
    tls: &Option<TlsConfig>,
    headers: &IndexMap<String, String>,
    proxy: &ProxyConfig,
    attrs: IndexMap<String, String>,
) -> std::result::Result<bytes::Bytes, &'static str> {
    let tls_settings = TlsSettings::from_options(tls).map_err(|_| "Invalid TLS options")?;
    let client =
        HttpClient::<Body>::new(tls_settings, proxy).map_err(|_| "Invalid TLS settings")?;

    let url = Url::parse_with_params(url.as_ref(), attrs.iter().map(|(k, v)| (k, v)))
        .map_err(|_| "Invalid URL Params")?;

    // Build HTTP request
    let mut builder = http::request::Builder::new().uri(url.to_string());

    // Augment with headers. These may be required e.g. for authentication to
    // private endpoints.
    for (header, value) in headers.iter() {
        builder = builder.header(header.as_str(), value.as_str());
    }

    let request = builder
        .body(Body::empty())
        .map_err(|_| "Couldn't create HTTP request")?;

    debug!(
        message = "Attempting to retrieve configuration",
        url = ?url.as_str()
    );

    let resp = client.send(request).await.map_err(|err| {
        let message = "HTTP error";
        error!(
            message,
            ?err,
            url = ?url.as_str()
        );

        message
    })?;

    debug!(
        message = "Response received",
        url = ?url.as_str()
    );

    hyper::body::to_bytes(resp.into_body())
        .await
        .map_err(|err| {
            let message = "Error interpreting response";
            let cause = err.into_cause();

            error!(
                message,
                err = ?cause
            );

            message
        })
}

fn build_attributes() -> IndexMap<String, String> {
    let mut attrs = IndexMap::new();

    match machine_id() {
        Ok(uid) => {
            attrs.insert("uid".to_string(), uid.trim().to_string());
        }
        Err(err) => {
            let uid = uuid::Uuid::new_v4().to_string();
            warn!(
                message = "Read uid from /etc/machine-id failed, using ephemeral uuid",
                ?err,
                ?uid,
            );

            attrs.insert("uid".to_string(), uid);
            attrs.insert("epheral_uid".to_string(), "true".to_string());
        }
    };

    let kernel = kernel_version().unwrap_or_else(|| "unknown".to_string());
    attrs.insert("kernel".to_string(), kernel);

    let hostname = crate::hostname().unwrap_or_default();
    attrs.insert("hostname".to_string(), hostname);

    attrs.insert("os".to_string(), os_version().unwrap_or_default());

    attrs
}

/// Polls the HTTP endpoint after/every `interval`, returning a stream of `ConfigBuilder`.
fn poll_http(
    interval: std::time::Duration,
    url: Url,
    tls_options: Option<TlsConfig>,
    headers: IndexMap<String, String>,
    proxy: ProxyConfig,
) -> impl Stream<Item = crate::signal::SignalTo> {
    let mut interval = tokio::time::interval_at(tokio::time::Instant::now() + interval, interval);

    stream! {
        loop {
            interval.tick().await;
            let attrs = build_attributes();
            match http_request_to_config_builder(&url, &tls_options, &headers, &proxy, attrs).await {
                Ok(builder) => {
                    yield signal::SignalTo::ReloadFromConfigBuilder(builder)
                },
                Err(_) => return,
            };

            debug!(
                message = "HTTP provider is waiting",
                ?interval,
                url = ?url.as_str()
            );
        }
    }
}
