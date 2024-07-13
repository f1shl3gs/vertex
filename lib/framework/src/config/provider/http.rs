mod chunk;

use std::time::Duration;

use async_stream::stream;
use backoff::ExponentialBackoff;
use bytes::{Buf, Bytes};
use chunk::ChunkedDecoder;
use configurable::{configurable_component, Configurable};
use futures::{Stream, StreamExt};
use futures_util::TryStreamExt;
use http::header::{ACCEPT, TRANSFER_ENCODING};
use http::{Request, Response};
use http_body_util::{BodyExt, BodyStream, Empty};
use hyper::body::Incoming;
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
#[derive(Clone)]
#[serde(deny_unknown_fields)]
struct Config {
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

    /// HTTP headers to add to the request.
    #[serde(default)]
    headers: IndexMap<String, String>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "http")]
impl ProviderConfig for Config {
    async fn build(&mut self, signal_handler: &mut SignalHandler) -> Result<Builder, Vec<String>> {
        let url = self.url.clone();
        let tls_config = self.tls.take();
        let proxy = ProxyConfig::from_env().merge(&self.proxy);

        let mut cfs = Box::pin(poll_http(
            self.interval,
            url,
            self.headers.clone(),
            tls_config,
            proxy,
        ));

        let builder = match timeout(Duration::from_secs(20), cfs.next()).await {
            Ok(b) => b.expect("first build should not be empty"),
            Err(_err) => {
                return Err(vec!["timeout for the first config".to_string()]);
            }
        };

        signal_handler.add(cfs.map(signal::SignalTo::ReloadFromConfigBuilder));

        Ok(builder)
    }
}

/// Makes an HTTP request to the provided endpoint, returning the Body.
async fn http_request(
    url: &Url,
    headers: &IndexMap<String, String>,
    tls_config: &Option<TlsConfig>,
    proxy: &ProxyConfig,
) -> Result<Response<Incoming>, crate::Error> {
    let client = HttpClient::new(tls_config, proxy)?;
    let mut builder = Request::get(url.as_str()).header(ACCEPT, "application/yaml");
    for (key, value) in headers {
        builder = builder.header(key, value);
    }
    let req = builder.body(Empty::<Bytes>::default())?;

    client.send(req).await.map_err(Into::into)
}

fn watchable_response(headers: &http::header::HeaderMap) -> bool {
    const CHUNKED: &str = "chunked";

    match headers.get(TRANSFER_ENCODING) {
        Some(value) => match value.to_str() {
            Ok(value) => value.contains(CHUNKED),
            Err(_err) => false,
        },
        None => false,
    }
}

/// Polls the HTTP endpoint after/every `interval`, returning a stream of `ConfigBuilder`.
fn poll_http(
    interval: Duration,
    url: Url,
    headers: IndexMap<String, String>,
    tls_config: Option<TlsConfig>,
    proxy: ProxyConfig,
) -> impl Stream<Item = Builder> {
    stream! {
        loop {
            // Before this loop starting, config is loaded already.
            tokio::time::sleep(interval).await;

            // Retry loop to fetch config
            let mut backoff = ExponentialBackoff::from_secs(10).max_delay(5 * interval);
            let (parts, incoming) = loop {
                let resp = match http_request(&url, &headers, &tls_config, &proxy).await {
                    Ok(resp) => resp,
                    Err(err) => {
                        warn!(message = "fetch request failed", %err);
                        backoff.wait().await;
                        continue;
                    }
                };

                if resp.status() != 200 {
                    warn!(
                        message = "fetch config failed, unexpected status code",
                        ?url,
                        code = ?resp.status(),
                    );

                    backoff.wait().await;
                    continue;
                }

                break resp.into_parts();
            };

            if !watchable_response(&parts.headers) {
                match incoming.collect().await {
                    Ok(data) => {
                        let builder = match config::load(data.to_bytes().chunk(), None) {
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

                                backoff.wait().await;
                                continue;
                            }
                        };

                        yield builder;

                        tokio::time::sleep(interval).await;
                    }

                    Err(err) => {
                        warn!(message = "load config failed", %err, %url);

                        backoff.wait().await;

                        continue;
                    }
                }

                continue;
            }

            let mut frames = FramedRead::new(
                StreamReader::new(Box::pin(BodyStream::new(incoming).try_filter_map(|frame| async { Ok(frame.into_data().ok()) }))
                    .map_err(|err| {
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
                    })
                ),
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
                        error!(message = "read new frame failed", %err);

                        break;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_config() {
        let cfg = configurable::generate_config::<Config>();
        serde_yaml::from_str::<Config>(&cfg).expect("Invalid config generated");
    }
}
