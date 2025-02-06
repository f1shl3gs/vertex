use std::net::SocketAddr;

use bytes::{Bytes, BytesMut};
use codecs::decoding::{DeserializerConfig, FramingConfig};
use codecs::{Decoder, DecodingConfig};
use configurable::configurable_component;
use event::Events;
use framework::config::{Output, Resource, SourceConfig, SourceContext};
use framework::source::http::{ErrorMessage, HttpSource, HttpSourceAuthConfig};
use framework::tls::TlsConfig;
use framework::Source;
use glob::MatchOptions;
use http::{HeaderMap, HeaderValue, Method, StatusCode, Uri};
use tokio_util::codec::Decoder as _;
use url::form_urlencoded;
use value::{path, value};

use super::default_framing_message_based;
use crate::common::http::HttpParamKind;

#[configurable_component(source, name = "http")]
struct Config {
    /// The socket address to listen for connections on
    listen: SocketAddr,

    tls: Option<TlsConfig>,

    auth: Option<HttpSourceAuthConfig>,

    /// A list of HTTP headers to include in the log record
    #[serde(default)]
    headers: Vec<HttpParamKind>,

    /// A list of URL query parameters to include in the log record
    #[serde(default)]
    query_parameters: Vec<HttpParamKind>,

    #[serde(default = "default_framing_message_based")]
    framing: FramingConfig,

    #[serde(default)]
    decoding: DeserializerConfig,
}

#[async_trait::async_trait]
#[typetag::serde(name = "http")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let decoder = DecodingConfig::new(self.framing.clone(), self.decoding.clone()).build()?;

        let source = SimpleHttpSource {
            decoder,
            headers: self.headers.clone(),
            query_parameters: self.query_parameters.clone(),
        };

        source.run(
            self.listen,
            Method::POST,
            "/",
            false,
            self.tls.as_ref(),
            self.auth.as_ref(),
            cx,
        )
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::logs()]
    }

    fn resources(&self) -> Vec<Resource> {
        vec![Resource::tcp(self.listen)]
    }

    fn can_acknowledge(&self) -> bool {
        true
    }
}

#[derive(Clone)]
struct SimpleHttpSource {
    decoder: Decoder,

    headers: Vec<HttpParamKind>,
    query_parameters: Vec<HttpParamKind>,
}

impl HttpSource for SimpleHttpSource {
    fn build_events(
        &self,
        uri: &Uri,
        header_map: &HeaderMap,
        peer_addr: &SocketAddr,
        body: Bytes,
    ) -> Result<Events, ErrorMessage> {
        if body.is_empty() {
            return Err(ErrorMessage::new(
                StatusCode::BAD_REQUEST,
                "empty request body is not allowed".to_string(),
            ));
        }

        let mut decoder = self.decoder.clone();
        let mut bytes = BytesMut::new();
        bytes.extend_from_slice(&body);

        let mut events = Events::Logs(vec![]);

        loop {
            match decoder.decode_eof(&mut bytes) {
                Ok(Some((mut partial, _size))) => {
                    partial.for_each_log(|log| {
                        let client = peer_addr.ip().to_string();
                        let path = uri.path();

                        let value = value!({
                            "client": client,
                            "path": path,
                        });

                        // add headers
                        for h in &self.headers {
                            match h {
                                HttpParamKind::Exact(name) => {
                                    let value = header_map.get(name).map(|hv| {
                                        let v = HeaderValue::as_bytes(hv).to_vec();
                                        Bytes::from(v)
                                    });

                                    log.value_mut().insert(path!("headers", name), value);
                                }
                                HttpParamKind::Glob(pattern) => {
                                    header_map.iter().for_each(|(k, v)| {
                                        if pattern.matches_with(k.as_str(), MatchOptions::default())
                                        {
                                            log.value_mut()
                                                .insert(path!("headers", k.as_str()), v.as_bytes());
                                        }
                                    });
                                }
                            }
                        }

                        // add query parameters
                        if !self.query_parameters.is_empty() && uri.query().is_some() {
                            let parameters =
                                form_urlencoded::parse(uri.query().unwrap().as_bytes())
                                    .collect::<Vec<_>>();

                            for qp in &self.query_parameters {
                                match qp {
                                    HttpParamKind::Exact(name) => {
                                        let value = parameters.iter().find_map(|(k, v)| {
                                            if k == name {
                                                Some(Bytes::from(v.as_bytes().to_vec()))
                                            } else {
                                                None
                                            }
                                        });

                                        log.value_mut().insert(
                                            path!("query_parameters", name.as_str()),
                                            value,
                                        );
                                    }
                                    HttpParamKind::Glob(pattern) => {
                                        parameters.iter().for_each(|(k, v)| {
                                            if pattern
                                                .matches_with(k.as_ref(), MatchOptions::default())
                                            {
                                                log.value_mut().insert(
                                                    path!("query_parameters", k.as_ref()),
                                                    Bytes::from(v.as_bytes().to_vec()),
                                                );
                                            }
                                        });
                                    }
                                }
                            }
                        }

                        log.metadata_mut().value_mut().insert("http", value);
                    });

                    events.merge(partial);
                }
                Ok(None) => break,
                Err(err) => {
                    return Err(ErrorMessage::new(
                        StatusCode::BAD_REQUEST,
                        format!("failed decoding body, {}", err),
                    ))
                }
            }
        }

        Ok(events)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::io::Write;

    use codecs::decoding::format::JsonDeserializerConfig;
    use codecs::decoding::framing::NewlineDelimitedDecoderConfig;
    use event::{EventMetadata, EventStatus, LogRecord};
    use flate2::{
        write::{GzEncoder, ZlibEncoder},
        Compression,
    };
    use framework::config::ProxyConfig;
    use framework::http::HttpClient;
    use framework::Pipeline;
    use futures::Stream;
    use futures_util::StreamExt;
    use http::Request;
    use http_body_util::{BodyExt, Full};
    use testify::{btreemap, collect_ready, next_addr};
    use tokio::pin;
    use value::Value;

    use super::*;
    use crate::testing::{trace_init, wait_for_tcp};

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }

    async fn send(
        method: Method,
        path: String,
        header_map: BTreeMap<String, String>,
        body: impl AsRef<[u8]>,
        status_code: StatusCode,
    ) {
        let client = HttpClient::new(None, &ProxyConfig::default()).unwrap();

        let mut builder = Request::builder().method(method).uri(path);

        for (k, v) in header_map {
            builder = builder.header(k, v);
        }

        let req = builder
            .body(Full::new(Bytes::from(body.as_ref().to_vec())))
            .unwrap();

        let resp = client.send(req).await.unwrap();
        let (parts, incoming) = resp.into_parts();

        let msg = incoming.collect().await.unwrap().to_bytes();

        assert_eq!(
            parts.status,
            status_code,
            "resp: {:?}",
            String::from_utf8_lossy(&msg)
        )
    }

    async fn spawn_source(
        headers: &[&str],
        queries: &[&str],
        framing: FramingConfig,
        decoding: DeserializerConfig,
    ) -> (SocketAddr, impl Stream<Item = LogRecord>) {
        let in_addr = next_addr();
        let (output, recv) = Pipeline::new_test_finalize(EventStatus::Delivered);

        let headers = headers
            .iter()
            .map(|h| {
                if h.contains("*") {
                    HttpParamKind::Glob(glob::Pattern::new(h).unwrap())
                } else {
                    HttpParamKind::Exact(h.to_string())
                }
            })
            .collect::<Vec<_>>();

        let query_parameters = queries
            .iter()
            .map(|q| {
                if q.contains("*") {
                    HttpParamKind::Glob(glob::Pattern::new(q).unwrap())
                } else {
                    HttpParamKind::Exact(q.to_string())
                }
            })
            .collect::<Vec<_>>();

        tokio::spawn(async move {
            let config = Config {
                listen: in_addr,
                tls: None,
                auth: None,
                headers,
                query_parameters,
                framing,
                decoding,
            };

            config
                .build(SourceContext::new_test(output))
                .await
                .unwrap()
                .await
                .unwrap();
        });

        wait_for_tcp(in_addr).await;

        let n = recv
            .filter_map(|events| async move { events.into_logs() })
            .flat_map(futures::stream::iter);

        (in_addr, n)
    }

    fn build_log(msg: impl Into<Value>) -> LogRecord {
        LogRecord::from_parts(
            EventMetadata::default_with_value(value!({
                "http": {
                    "client": "127.0.0.1",
                    "path": "/"
                }
            })),
            msg.into(),
        )
    }

    #[tokio::test]
    async fn empty() {
        trace_init();

        let (addr, receiver) = spawn_source(
            &[],
            &[],
            FramingConfig::NewlineDelimited(NewlineDelimitedDecoderConfig::default()),
            DeserializerConfig::Bytes,
        )
        .await;

        send(
            Method::POST,
            format!("http://{}/", addr),
            btreemap! {},
            "".to_string(),
            StatusCode::BAD_REQUEST,
        )
        .await;

        pin!(receiver);
        let logs = collect_ready(receiver).await;

        assert!(logs.is_empty());
    }

    #[tokio::test]
    async fn multiline_text() {
        trace_init();

        let (addr, receiver) = spawn_source(
            &[],
            &[],
            FramingConfig::NewlineDelimited(NewlineDelimitedDecoderConfig::default()),
            DeserializerConfig::Bytes,
        )
        .await;

        send(
            Method::POST,
            format!("http://{}/", addr),
            btreemap! {},
            "foo\nbar\n".to_string(),
            StatusCode::OK,
        )
        .await;

        pin!(receiver);
        let got = collect_ready(receiver).await;

        assert_eq!(
            vec![
                build_log(value!({"message": "foo"})),
                build_log(value!({"message": "bar"}))
            ],
            got
        )
    }

    #[tokio::test]
    async fn multiline_preserves_newlines() {
        trace_init();

        let (addr, receiver) = spawn_source(
            &[],
            &[],
            FramingConfig::NewlineDelimited(NewlineDelimitedDecoderConfig::default()),
            DeserializerConfig::Bytes,
        )
        .await;

        send(
            Method::POST,
            format!("http://{}/", addr),
            btreemap! {},
            "foo\nbar".to_string(),
            StatusCode::OK,
        )
        .await;

        pin!(receiver);
        let got = collect_ready(receiver).await;
        assert_eq!(
            vec![
                build_log(value!({"message": "foo"})),
                build_log(value!({"message": "bar"}))
            ],
            got
        )
    }

    #[tokio::test]
    async fn multiline_json() {
        trace_init();

        let (addr, receiver) = spawn_source(
            &[],
            &[],
            FramingConfig::NewlineDelimited(NewlineDelimitedDecoderConfig::default()),
            DeserializerConfig::Json(JsonDeserializerConfig::default()),
        )
        .await;

        send(
            Method::POST,
            format!("http://{}/", addr),
            btreemap! {},
            "{}\n{\"foo\":\"bar\"}\n".to_string(),
            StatusCode::OK,
        )
        .await;

        pin!(receiver);
        let got = collect_ready(receiver).await;
        assert_eq!(
            vec![build_log(value!({})), build_log(value!({"foo": "bar"})),],
            got
        )
    }

    #[tokio::test]
    async fn request_headers() {
        trace_init();

        let (addr, receiver) = spawn_source(
            &[
                "User-Agent",
                "Upgrade-Insecure-Requests",
                "X-*",
                "AbsentHeader",
            ],
            &[],
            FramingConfig::NewlineDelimited(NewlineDelimitedDecoderConfig::default()),
            DeserializerConfig::Json(JsonDeserializerConfig::default()),
        )
        .await;

        send(
            Method::POST,
            format!("http://{}/", addr),
            btreemap! {
                "User-Agent" => "test_client",
                "Upgrade-Insecure-Requests" => "false",
                "X-Test-Header" => "true"
            },
            "{\"key\":\"value1\"}\n{\"key\":\"value2\"}\n".to_string(),
            StatusCode::OK,
        )
        .await;

        pin!(receiver);
        let got = collect_ready(receiver).await;

        assert_eq!(
            vec![
                build_log(value!({
                    "key": "value1",
                    "headers": {
                        "User-Agent": "test_client",
                        "Upgrade-Insecure-Requests": "false",
                        "x-test-header": "true", // hyper will lower case the header name
                        "AbsentHeader": null,
                    }
                })),
                build_log(value!({
                    "key": "value2",
                    "headers": {
                        "User-Agent": "test_client",
                        "Upgrade-Insecure-Requests": "false",
                        "x-test-header": "true", // hyper will lower case the header name
                        "AbsentHeader": null,
                    }
                }))
            ],
            got
        )
    }

    #[tokio::test]
    async fn request_headers_wildcard() {
        trace_init();

        let (addr, receiver) = spawn_source(
            &["*"],
            &[],
            FramingConfig::NewlineDelimited(NewlineDelimitedDecoderConfig::default()),
            DeserializerConfig::Json(JsonDeserializerConfig::default()),
        )
        .await;

        send(
            Method::POST,
            format!("http://{}/", addr),
            btreemap! {
                "User-Agent" => "test_client",
                "Upgrade-Insecure-Requests" => "false",
                "X-Test-Header" => "true",
                "X-Case-Sensitive-Value" => "CaseSensitive"
            },
            "{\"key\":\"value1\"}\n{\"key\":\"value2\"}\n".to_string(),
            StatusCode::OK,
        )
        .await;

        pin!(receiver);
        let mut got = collect_ready(receiver).await;
        assert_eq!(got.len(), 2);

        let log = got.remove(0);
        assert_eq!(log["key"], "value1".into());
        assert_eq!(log["headers.\"user-agent\""], "test_client".into());
        assert_eq!(
            log["headers.\"x-case-sensitive-value\""],
            "CaseSensitive".into()
        );
    }

    #[tokio::test]
    async fn request_parameter() {
        trace_init();

        let (addr, receiver) = spawn_source(
            &[],
            &["source", "reg*"],
            FramingConfig::NewlineDelimited(NewlineDelimitedDecoderConfig::default()),
            DeserializerConfig::Json(JsonDeserializerConfig::default()),
        )
        .await;

        send(
            Method::POST,
            format!("http://{}?source=staging&region=gb", addr),
            btreemap! {
                "User-Agent" => "test_client",
                "Upgrade-Insecure-Requests" => "false",
                "X-Test-Header" => "true",
                "X-Case-Sensitive-Value" => "CaseSensitive"
            },
            "{\"key\":\"value1\"}\n{\"key\":\"value2\"}\n".to_string(),
            StatusCode::OK,
        )
        .await;

        pin!(receiver);
        let mut got = collect_ready(receiver).await;
        assert_eq!(got.len(), 2);

        let log = got.remove(0);
        assert_eq!(log["key"], "value1".into());

        assert_eq!(log["query_parameters.source"], "staging".into());
        assert_eq!(log["query_parameters.region"], "gb".into());
    }

    #[tokio::test]
    async fn http_gzip_deflate() {
        trace_init();

        let body = "test body";

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(body.as_bytes()).unwrap();
        let body = encoder.finish().unwrap();

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(body.as_slice()).unwrap();
        let body = encoder.finish().unwrap();

        let (addr, receiver) = spawn_source(
            &[],
            &["source", "reg*"],
            FramingConfig::NewlineDelimited(NewlineDelimitedDecoderConfig::default()),
            DeserializerConfig::Bytes,
        )
        .await;

        send(
            Method::POST,
            format!("http://{}?source=staging&region=gb", addr),
            btreemap! {
                "User-Agent" => "test_client",
                "Upgrade-Insecure-Requests" => "false",
                "X-Test-Header" => "true",
                "X-Case-Sensitive-Value" => "CaseSensitive",
                "Content-Encoding" => "gzip, deflate",
            },
            body,
            StatusCode::OK,
        )
        .await;

        pin!(receiver);
        let mut got = collect_ready(receiver).await;
        assert_eq!(got.len(), 1);

        let log = got.remove(0);

        assert_eq!(log["query_parameters.source"], "staging".into());
        assert_eq!(log["query_parameters.region"], "gb".into());
    }

    #[tokio::test]
    async fn http_path() {
        trace_init();

        let (addr, receiver) = spawn_source(
            &[],
            &[],
            FramingConfig::NewlineDelimited(NewlineDelimitedDecoderConfig::default()),
            DeserializerConfig::Bytes,
        )
        .await;

        send(
            Method::POST,
            format!("http://{}/some/path", addr),
            btreemap! {},
            "foo\n".to_string(),
            StatusCode::OK,
        )
        .await;

        pin!(receiver);
        let mut got = collect_ready(receiver).await;

        assert_eq!(got.len(), 1);
        let log = got.remove(0);

        println!("{:#?}", log);

        assert_eq!(
            log.metadata().value().get("http.path").unwrap(),
            &Value::from("/some/path")
        );
    }
}
