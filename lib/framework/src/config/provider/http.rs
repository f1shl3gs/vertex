use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::Duration;

use async_stream::stream;
use backoff::ExponentialBackoff;
use bytes::{Buf, Bytes, BytesMut};
use configurable::configurable_component;
use futures::{Stream, StreamExt, TryStreamExt};
use http::header::{ACCEPT, TRANSFER_ENCODING};
use http::{Request, Response, Uri};
use http_body_util::{BodyExt, BodyStream, Empty};
use hyper::body::Incoming;
use indexmap::IndexMap;
use tokio::time::timeout;
use tokio_util::codec::{Decoder, FramedRead};
use tokio_util::io::StreamReader;

use crate::SignalHandler;
use crate::config::{Builder, ProxyConfig, provider::ProviderConfig};
use crate::http::{Auth, HttpClient};
use crate::tls::TlsConfig;
use crate::{config, signal};

const fn default_interval() -> Duration {
    Duration::from_secs(60)
}

#[configurable_component(provider, name = "http")]
#[derive(Clone)]
#[serde(deny_unknown_fields)]
struct Config {
    /// The URL to download config
    #[configurable(format = "uri", example = "https://exampel.com/config")]
    #[serde(with = "crate::config::serde_uri")]
    endpoint: Uri,

    /// The interval between fetch config.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    /// Extra HTTP headers to add to request.
    #[serde(default)]
    headers: IndexMap<String, String>,

    auth: Option<Auth>,

    tls: Option<TlsConfig>,

    /// Configures an HTTP/HTTPS proxy for Vertex to use. By default, the globally
    /// configured proxy is used.
    #[serde(default)]
    proxy: ProxyConfig,
}

#[async_trait::async_trait]
#[typetag::serde(name = "http")]
impl ProviderConfig for Config {
    async fn build(&mut self, signal_handler: &mut SignalHandler) -> Result<Builder, Vec<String>> {
        let tls_config = self.tls.take();
        let proxy = ProxyConfig::from_env().merge(&self.proxy);

        let mut cfs = Box::pin(poll_http(
            self.endpoint.clone(),
            self.auth.clone(),
            self.headers.clone(),
            tls_config,
            proxy,
            self.interval,
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
    uri: &Uri,
    auth: Option<&Auth>,
    headers: &IndexMap<String, String>,
    tls_config: Option<&TlsConfig>,
    proxy: &ProxyConfig,
) -> Result<Response<Incoming>, crate::Error> {
    let client = HttpClient::new(tls_config, proxy)?;
    let mut builder = Request::get(uri).header(ACCEPT, "application/yaml");
    for (key, value) in headers {
        builder = builder.header(key, value);
    }
    let mut req = builder.body(Empty::<Bytes>::default())?;
    if let Some(auth) = auth {
        auth.apply(&mut req);
    }

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

/// Hash value of the content is checked, and if the current_hash is same as the last_hash
/// then nothing will be yield, so vertex will not reload config. Note that, the comment of
/// the config(only in yaml) will be calculated too, so it will trigger the reload routine.
#[inline]
fn config_hash(data: &Bytes) -> u64 {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

/// Polls the HTTP endpoint after/every `interval`, returning a stream of `ConfigBuilder`.
fn poll_http(
    endpoint: Uri,
    auth: Option<Auth>,
    headers: IndexMap<String, String>,
    tls_config: Option<TlsConfig>,
    proxy: ProxyConfig,
    interval: Duration,
) -> impl Stream<Item = Builder> {
    stream! {
        let mut last_hash = 0u64;

        loop {
            // Retry loop to fetch config
            let mut backoff = ExponentialBackoff::from_secs(10).max_delay(5 * interval);
            let (parts, incoming) = loop {
                let resp = match http_request(&endpoint, auth.as_ref(), &headers, tls_config.as_ref(), &proxy).await {
                    Ok(resp) => resp,
                    Err(err) => {
                        warn!(
                            message = "fetch request failed",
                            %err,
                        );

                        backoff.wait().await;
                        continue;
                    }
                };

                if resp.status() != 200 {
                    warn!(
                        message = "fetch config failed, unexpected status code",
                        ?endpoint,
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
                        let data = data.to_bytes();
                        let hash = config_hash(&data);
                        if hash == last_hash && last_hash != 0 {
                            debug!(
                                message = "config is not changed yet",
                            );

                            tokio::time::sleep(interval).await;
                            continue;
                        }

                        let builder = match config::load(data.as_ref(), None) {
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

                        // save the last hash
                        last_hash = hash;
                        backoff.reset();

                        yield builder;

                        tokio::time::sleep(interval).await;
                    }

                    Err(err) => {
                        warn!(
                            message = "load config failed",
                            %err,
                            %endpoint,
                        );

                        backoff.wait().await;
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

                        std::io::Error::other(err)
                    })
                ),
                ChunkedDecoder::default(),
            );

            while let Some(result) = frames.next().await {
                match result {
                    Ok(data) => {
                        let hash = config_hash(&data);
                        if hash == last_hash && last_hash != 0 {
                            debug!(
                                message = "config is not changed yet",
                            );

                            continue;
                        }

                        let builder = match config::load(data.as_ref(), None) {
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

                                continue;
                            }
                        };

                        // save the last hash
                        last_hash = hash;
                        backoff.reset();

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

#[derive(Default)]
struct ChunkedDecoder {
    state: Option<usize>,
}

/// https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Transfer-Encoding
impl Decoder for ChunkedDecoder {
    type Item = Bytes;
    type Error = std::io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        loop {
            match self.state {
                Some(size) => {
                    if buf.len() < size + 2 {
                        return Ok(None);
                    }

                    let data = buf.split_to(size).freeze();
                    if !buf.ends_with(b"\r\n") {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "invalid data",
                        ));
                    }

                    buf.advance(2);
                    self.state = None;

                    return Ok(Some(data));
                }
                None => {
                    let end = 18.min(buf.len());
                    let Some(len) = buf[..end].windows(2).position(|buf| buf == b"\r\n") else {
                        return Ok(None);
                    };

                    if len == 0 || len > 16 {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "invalid data",
                        ));
                    }

                    match parse_length(&buf[..len]) {
                        Ok(size) => {
                            self.state = Some(size);
                            buf.advance(len + 2);
                        }
                        Err(_pos) => return Err(std::io::Error::other("invalid length")),
                    }
                }
            }
        }
    }
}

fn parse_length(buf: &[u8]) -> Result<usize, usize> {
    let mut len = 0;

    for (index, ch) in buf.iter().enumerate() {
        let b = match ch {
            b'0'..=b'9' => *ch - b'0',
            b'a'..=b'f' => *ch - b'a' + 10,
            b'A'..=b'F' => *ch - b'A' + 10,
            _ => return Err(index),
        };

        len <<= 4;
        len |= b as usize
    }

    Ok(len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        let cfg = configurable::generate_config::<Config>();
        serde_yaml::from_str::<Config>(&cfg).expect("Invalid config generated");
    }

    #[test]
    fn hex() {
        let tests = [("0", 0), ("a", 10), ("F", 15), ("10", 16), ("233", 563)];

        for (input, want) in tests {
            let got = parse_length(input.as_bytes()).unwrap();
            assert_eq!(want, got, "{input}")
        }
    }

    #[tokio::test]
    async fn good() {
        let input = "7\r\nMozilla\r\n11\r\nDeveloper Network\r\n0\r\n\r\n";
        let want = ["Mozilla", "Developer Network", ""];

        let frames = FramedRead::new(std::io::Cursor::new(input), ChunkedDecoder::default());
        let got = frames.try_collect::<Vec<Bytes>>().await.unwrap();

        assert_eq!(3, got.len());
        for i in 0..3 {
            assert_eq!(want[i], String::from_utf8_lossy(got[i].as_ref()));
        }
    }
}
