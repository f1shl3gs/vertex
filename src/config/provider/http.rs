use std::path::PathBuf;

use async_stream::stream;
use bytes::Buf;
use futures::Stream;
use hyper::Body;
use indexmap::IndexMap;
use md5::Digest;
use url::Url;
use serde::{Deserialize, Serialize};
use sysinfo::unix::{kernel_version, os_version, machine_id};

use crate::http::HttpClient;
use crate::SignalHandler;
use crate::signal;
use crate::tls::{TlsOptions, TlsSettings};
use crate::config::{
    ProviderDescription, Builder, ProxyConfig, provider::ProviderConfig,
    deserialize_duration, serialize_duration, GenerateConfig, default_interval,
};


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
    #[serde(deserialize_with = "deserialize_duration", serialize_with = "serialize_duration")]
    interval: chrono::Duration,
    tls: Option<TlsOptions>,
    proxy: ProxyConfig,
    #[serde(default)]
    persist: Option<PathBuf>,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            url: None,
            request: RequestConfig::default(),
            interval: chrono::Duration::seconds(30),
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
        let url = self.url.take()
            .ok_or_else(|| vec!["URL is required for http provider".to_owned()])?;

        let tls_options = self.tls.take();
        let poll_interval = self.interval.to_std().unwrap();
        let request = self.request.clone();
        let proxy = ProxyConfig::from_env().merge(&self.proxy);
        let attrs = build_attributes();
        let (builder, _) = http_request_to_config_builder(&url, &tls_options, &request.headers, &proxy, attrs)
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
    fn generate_config() -> serde_yaml::Value {
        let url = "https://example.config.com/config".parse().unwrap();

        serde_yaml::to_value(Self {
            url: Some(url),
            request: Default::default(),
            interval: default_interval(),
            tls: None,
            proxy: Default::default(),
            persist: None,
        }).unwrap()
    }
}

struct Watcher {
    digest: Digest,
}

impl Watcher {
    // Polls the HTTP endpoint after/every `interval`, returning a stream of `ConfigBuilder`.
    // fn poll_http(&self) -> impl Stream<Item=crate::signal::SignalTo> {
    //     todo!()
    // }
}

/// Calls `http_request`, serializing the result to a `ConfigBuilder`.
async fn http_request_to_config_builder(
    url: &Url,
    tls_options: &Option<TlsOptions>,
    headers: &IndexMap<String, String>,
    proxy: &ProxyConfig,
    attrs: IndexMap<String, String>,
) -> Result<(crate::config::Builder, String), Vec<String>> {
    let config_str = http_request(url, tls_options, headers, proxy, attrs)
        .await
        .map_err(|err| vec![err.to_owned()])?;

    let digest = md5::compute(&config_str);

    let (builder, warnings) = crate::config::load(config_str.chunk(), None)?;
    for warning in warnings.into_iter() {
        warn!("{}", warning);
    }

    Ok((builder, format!("{:?}", digest)))
}

/// Makes an HTTP request to the provided endpoint, returning the String body.
async fn http_request(
    url: &Url,
    tls: &Option<TlsOptions>,
    headers: &IndexMap<String, String>,
    proxy: &ProxyConfig,
    attrs: IndexMap<String, String>,
) -> std::result::Result<bytes::Bytes, &'static str> {
    let tls_settings = TlsSettings::from_options(tls)
        .map_err(|_| "Invalid TLS options")?;
    let client = HttpClient::<Body>::new(tls_settings, proxy)
        .map_err(|_| "Invalid TLS settings")?;

    let url = Url::parse_with_params(&url.to_string(), attrs.iter().map(|(k, v)| (k, v)))
        .map_err(|_| "Invalid URL Params")?;

    // Build HTTP request
    let mut builder = http::request::Builder::new()
        .uri(url.to_string());

    // Augment with headers. These may be required e.g. for authentication to
    // private endpoints.
    for (header, value) in headers.iter() {
        builder = builder.header(header.as_str(), value.as_str());
    }

    let request = builder.body(Body::empty())
        .map_err(|_| "Couldn't create HTTP request")?;


    debug!(
        message = "Attempting to retrieve configuration",
        url = ?url.as_str()
    );

    let resp = client.send(request)
        .await
        .map_err(|err| {
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

    let kernel = kernel_version().unwrap_or("unknown".to_string());
    attrs.insert("kernel".to_string(), kernel);

    let hostname = crate::hostname().unwrap_or("unknown".to_string());
    attrs.insert("hostname".to_string(), hostname);

    attrs.insert("os".to_string(), os_version().unwrap_or("".to_string()));

    attrs
}

/// Polls the HTTP endpoint after/every `interval`, returning a stream of `ConfigBuilder`.
fn poll_http(
    interval: std::time::Duration,
    url: Url,
    tls_options: Option<TlsOptions>,
    headers: IndexMap<String, String>,
    proxy: ProxyConfig,
) -> impl Stream<Item=crate::signal::SignalTo> {
    let mut interval = tokio::time::interval_at(
        tokio::time::Instant::now() + interval,
        interval,
    );

    stream! {
        let mut digest = String::new();

        loop {
            interval.tick().await;
            let attrs = build_attributes();
            match http_request_to_config_builder(&url, &tls_options, &headers, &proxy, attrs).await {
                Ok((builder, _digest)) => {
                    digest = _digest;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_os_version() {
        let v = os_version().unwrap();
        println!("{}", v);
        let k = kernel_version().unwrap();
        println!("{}", k);
    }

    #[test]
    fn test_md5() {
        let digest = md5::compute("abc");

        println!("digest: {:?}", digest);
    }
}