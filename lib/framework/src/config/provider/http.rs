mod chunk;

use std::time::Duration;

use async_stream::stream;
use backoff::ExponentialBackoff;
use bytes::Buf;
use chunk::ChunkedDecoder;
use configurable::{configurable_component, Configurable};
use futures::{Stream, StreamExt, TryStreamExt};
use http::{header, Request, Response};
use hyper::Body;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use tokio::time::timeout;
use tokio_util::codec::FramedRead;
use tokio_util::io::StreamReader;
use url::Url;

use crate::config::{provider::ProviderConfig, Builder, ProxyConfig};
use crate::http::HttpClient;
use crate::tls::TlsConfig;
use crate::SignalHandler;
use crate::{config, signal};

const fn default_interval() -> Duration {
    Duration::from_secs(60)
}

#[derive(Configurable, Clone, Debug, Default, Deserialize, Serialize)]
pub struct RequestConfig {
    #[serde(default)]
    pub headers: IndexMap<String, String>,
}

#[configurable_component(provider, name = "http")]
#[derive(Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct HttpConfig {
    /// The URL to download config
    #[configurable(required, format = "uri", example = "https://exampel.com/config")]
    url: Url,

    /// The interval between fetch config.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    tls: Option<TlsConfig>,
    /// Configures an HTTP/HTTPS proxy for Vertex to use. By default, the globally
    /// configured proxy is used.
    #[serde(default)]
    proxy: ProxyConfig,
}

#[async_trait::async_trait]
#[typetag::serde(name = "http")]
impl ProviderConfig for HttpConfig {
    async fn build(&mut self, signal_handler: &mut SignalHandler) -> Result<Builder, Vec<String>> {
        let url = self.url.clone();

        let tls_options = self.tls.take();
        let poll_interval = self.interval;
        // let request = self.request.clone();
        let proxy = ProxyConfig::from_env().merge(&self.proxy);

        let mut cfs = Box::pin(poll_http(poll_interval, url, tls_options, proxy));

        let builder = match timeout(Duration::from_secs(20), cfs.next()).await {
            Ok(b) => b.expect("first build should not be empty"),
            Err(_err) => {
                return Err(vec![format!("timeout for the first config")]);
            }
        };

        signal_handler.add(cfs.map(signal::SignalTo::ReloadFromConfigBuilder));

        Ok(builder)
    }
}

async fn http_request(
    url: &Url,
    tls_config: &Option<TlsConfig>,
    proxy: &ProxyConfig,
) -> Result<Response<Body>, crate::Error> {
    let client = HttpClient::new(tls_config, proxy)?;

    let req = Request::get(url.as_str())
        .header(header::ACCEPT, "application/yaml")
        .body(Body::empty())?;

    client.send(req).await.map_err(Into::into)
}

fn watchable_response(resp: &Response<Body>) -> bool {
    const CHUNKED: &str = "chunked";

    match resp.headers().get("Transfer-Encoding") {
        Some(hv) => hv
            .as_bytes()
            .windows(CHUNKED.len())
            .any(|w| w == CHUNKED.as_bytes()),

        None => false,
    }
}

/// Polls the HTTP endpoint after/every `interval`, returning a stream of `ConfigBuilder`.
fn poll_http(
    interval: Duration,
    url: Url,
    tls_options: Option<TlsConfig>,
    proxy: ProxyConfig,
) -> impl Stream<Item = Builder> {
    let mut b = ExponentialBackoff::from_secs(3).max_delay(Duration::from_secs(60));
    let mut backoff = move || {
        let to_sleep = b.next().expect("backoff should always return a duration");

        tokio::time::sleep(to_sleep)
    };

    stream! {
        loop {
            let resp = match http_request(&url, &tls_options, &proxy).await {
                Ok(resp) => resp,
                Err(err) => {
                    warn!(message = "request failed", ?err);
                    backoff().await;
                    continue;
                }
            };

            if resp.status() != 200 {
                warn!(
                    message = "fetch config failed, unexpected status code",
                    ?url,
                    code = ?resp.status(),
                );

                backoff().await;
                continue;
            }

            if !watchable_response(&resp) {
                let result = hyper::body::to_bytes(resp.into_body())
                    .await
                    .map_err(|err| {
                        let message = "Error interpreting response";
                        let cause = err.into_cause();

                        error!(
                            message,
                            err = ?cause
                        );

                        message
                    });

                match result {
                    Ok(data) => {
                        let builder = match config::load(data.chunk(), None) {
                            Ok((builder, warnings)) => {
                                for warning in warnings.into_iter() {
                                    warn!(message = warning)
                                }

                                builder
                            }
                            Err(errs) => {
                                for err in errs {
                                    error!(message = "load config builder failed", err)
                                }

                                backoff().await;
                                continue;
                            }
                        };

                        yield builder;

                        tokio::time::sleep(interval).await;
                    },

                    Err(err) => {
                        warn!(
                            message = "load config failed",
                            ?err,
                            ?url
                        );

                        backoff().await;

                        continue
                    }
                }

                continue;
            }

            let mut frames = FramedRead::new(
                StreamReader::new(resp.into_body().map_err(|err| {
                    // Client timeout. This will be ignored.
                    if err.is_timeout() {
                        return std::io::Error::new(std::io::ErrorKind::TimedOut, err);
                    }

                    // Unexpected EOF from chunked decoder.
                    // Tends to happen when watching for 300+s. This will be ignored.
                    if err.to_string().contains("unexpected EOF during chunk") {
                        return std::io::Error::new(std::io::ErrorKind::UnexpectedEof, err);
                    }

                    std::io::Error::new(std::io::ErrorKind::Other, err)
                })),
                ChunkedDecoder::default(),
            );

            while let Some(result) = frames.next().await {
                match result {
                    Ok(data) => {
                        let builder = match config::load(data.chunk(), None) {
                            Ok((builder, warnings)) => {
                                for warning in warnings.into_iter() {
                                    warn!(message = warning)
                                }

                                builder
                            }
                            Err(errs) => {
                                for err in errs {
                                    error!(message = err)
                                }

                                continue;
                            }
                        };

                        yield builder
                    }
                    Err(err) => {
                        error!(message = "read new frame failed", ?err);

                        break;
                    }
                }
            }

            debug!(
                message = "HTTP provider is waiting",
                ?interval,
                url = ?url.as_str()
            );

            tokio::time::sleep(interval).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_config() {
        let cfg = configurable::generate_config::<HttpConfig>();
        serde_yaml::from_str::<HttpConfig>(&cfg).expect("Invalid config generated");
    }
}
