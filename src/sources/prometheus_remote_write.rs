use std::borrow::Cow;
use std::cmp::Ordering;
use std::net::SocketAddr;

use bytes::Bytes;
use chrono::{DateTime, Utc};
use configurable::configurable_component;
use event::{Bucket, Events, Metric, Quantile};
use framework::config::{OutputType, Resource, SourceConfig, SourceContext};
use framework::source::http::{ErrorMessage, decode};
use framework::source::http::{HttpSource, HttpSourceAuthConfig};
use framework::{Source, tls::TlsConfig};
use http::header::CONTENT_ENCODING;
use http::{HeaderMap, Method, StatusCode, Uri};
use prometheus::{GroupKind, MetricGroup, proto};
use prost::Message;

/// Start an HTTP server and receive Protobuf encoded metrics.
#[configurable_component(source, name = "prometheus_remote_write")]
#[serde(deny_unknown_fields)]
struct Config {
    /// The address to accept connections on. The address must include a port
    listen: SocketAddr,

    /// HTTP Server TLS config
    tls: Option<TlsConfig>,

    auth: Option<HttpSourceAuthConfig>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "prometheus_remote_write")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let source = RemoteWriteSource;

        source.run(
            self.listen,
            Method::POST,
            "/write",
            true,
            self.tls.as_ref(),
            self.auth.as_ref(),
            cx,
        )
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::metric()]
    }

    fn resources(&self) -> Vec<Resource> {
        vec![Resource::tcp(self.listen)]
    }

    fn can_acknowledge(&self) -> bool {
        true
    }
}

#[derive(Clone)]
struct RemoteWriteSource;

impl RemoteWriteSource {
    fn decode_body(&self, body: Bytes) -> Result<Events, ErrorMessage> {
        let req = proto::WriteRequest::decode(body).map_err(|err| {
            ErrorMessage::new(
                StatusCode::BAD_REQUEST,
                format!("Could not decode write request: {err}"),
            )
        })?;

        let metrics = prometheus::parse_request(req).map_err(|err| {
            ErrorMessage::new(
                StatusCode::BAD_REQUEST,
                format!("Could not decode write request, {err:?}"),
            )
        })?;

        Ok(reparse_groups(metrics))
    }
}

impl HttpSource for RemoteWriteSource {
    fn build_events(
        &self,
        _uri: &Uri,
        headers: &HeaderMap,
        _peer_addr: &SocketAddr,
        mut body: Bytes,
    ) -> Result<Events, ErrorMessage> {
        if headers
            .get(CONTENT_ENCODING)
            .map(|header| header.as_ref())
            .is_some()
        {
            body = decode(Some("snappy"), body)?;
        }

        self.decode_body(body)
    }
}

fn utc_timestamp(timestamp: Option<i64>, default: DateTime<Utc>) -> Option<DateTime<Utc>> {
    match timestamp {
        None => Some(default),
        Some(timestamp) => {
            DateTime::<Utc>::from_timestamp(timestamp / 1000, (timestamp % 1000) as u32 * 1000000)
        }
    }
}

fn reparse_groups(groups: Vec<MetricGroup>) -> Events {
    let mut metrics = Vec::new();
    let start = Utc::now();

    for MetricGroup {
        name,
        description,
        metrics: group,
    } in groups
    {
        let name = Cow::<'static, str>::Owned(name);
        let description = Cow::<'static, str>::Owned(description);

        match group {
            GroupKind::Counter(map) => {
                for (key, metric) in map {
                    let counter = Metric::sum_with_tags(
                        name.clone(),
                        description.clone(),
                        metric.value,
                        key.labels,
                    )
                    .with_timestamp(utc_timestamp(key.timestamp, start));

                    metrics.push(counter)
                }
            }

            GroupKind::Gauge(map) | GroupKind::Untyped(map) => {
                for (key, metric) in map {
                    let gauge = Metric::gauge_with_tags(
                        name.clone(),
                        description.clone(),
                        metric.value,
                        key.labels,
                    )
                    .with_timestamp(utc_timestamp(key.timestamp, start));

                    metrics.push(gauge)
                }
            }

            GroupKind::Histogram(map) => {
                for (key, metric) in map {
                    let mut buckets = metric.buckets;
                    buckets.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));

                    let histogram = Metric::histogram_with_tags(
                        name.clone(),
                        description.clone(),
                        key.labels,
                        metric.count,
                        metric.sum,
                        buckets
                            .into_iter()
                            .map(|b| Bucket {
                                upper: b.bucket,
                                count: b.count,
                            })
                            .collect(),
                    )
                    .with_timestamp(utc_timestamp(key.timestamp, start));

                    metrics.push(histogram);
                }
            }

            GroupKind::Summary(map) => {
                for (key, metric) in map {
                    let summary = Metric::summary(
                        name.clone(),
                        description.clone(),
                        metric.count,
                        metric.sum,
                        metric
                            .quantiles
                            .into_iter()
                            .map(|q| Quantile {
                                quantile: q.quantile,
                                value: q.value,
                            })
                            .collect(),
                    )
                    .with_tags(key.labels.into())
                    .with_timestamp(utc_timestamp(key.timestamp, start));

                    metrics.push(summary)
                }
            }
        }
    }

    metrics.into()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bytes::BytesMut;
    use chrono::{SubsecRound, Utc};
    use event::{EventStatus, Metric, tags};
    use framework::config::ProxyConfig;
    use framework::http::HttpClient;
    use framework::pipeline::Pipeline;
    use framework::tls::TlsConfig;
    use http_body_util::Full;
    use testify::collect_ready;

    use super::*;
    use crate::common::prometheus::TimeSeries;
    use crate::testing::components;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }

    fn make_events() -> Events {
        let ts = || Utc::now().trunc_subsecs(3);

        let mut metrics = vec![
            Metric::sum_with_tags("counter_1", "", 42, tags!("type" => "counter")),
            Metric::gauge_with_tags("gauge_2", "", 42, tags!("type" => "gauge")),
            Metric::histogram_with_tags(
                "histogram_3",
                "",
                tags!("type" => "histogram"),
                96_u64,
                156.2,
                vec![
                    Bucket {
                        upper: 2.3,
                        count: 11,
                    },
                    Bucket {
                        upper: 4.2,
                        count: 85,
                    },
                ],
            ),
            Metric::summary(
                "summary_4",
                "",
                23_u64,
                8.6,
                vec![
                    Quantile {
                        quantile: 0.1,
                        value: 1.2,
                    },
                    Quantile {
                        quantile: 0.5,
                        value: 3.6,
                    },
                    Quantile {
                        quantile: 0.9,
                        value: 5.2,
                    },
                ],
            )
            .with_tags(tags!("type" => "summary")),
        ];

        metrics.iter_mut().for_each(|m| m.timestamp = Some(ts()));

        metrics.into()
    }

    async fn run_and_receive(tls: Option<TlsConfig>) {
        components::init_test();

        let listen = testify::next_addr();
        let (tx, rx) = Pipeline::new_test_finalize(EventStatus::Delivered);

        let source = Config {
            listen,
            auth: None,
            tls: tls.clone(),
        };

        let source = source.build(SourceContext::new_test(tx)).await.unwrap();
        tokio::spawn(source);

        // wait for source start
        tokio::time::sleep(Duration::from_secs(1)).await;

        let client = HttpClient::new(
            Some(&TlsConfig::test_client_config()),
            &ProxyConfig::default(),
        )
        .unwrap();
        let url = format!(
            "{}://localhost:{}/write",
            if tls.is_some() { "https" } else { "http" },
            listen.port()
        );

        let mut timeseries = TimeSeries::new();
        let mut events = make_events();
        events.for_each_metric(|metric| {
            timeseries.encode_metric(metric);
        });

        let wr = timeseries.finish();
        let mut out = BytesMut::with_capacity(wr.encoded_len());
        wr.encode(&mut out).expect("Out of memory");

        let body = out.freeze();

        let req = http::Request::post(&url).body(Full::new(body)).unwrap();

        let resp = client.send(req).await.unwrap();
        assert!(resp.status().is_success());

        let output = collect_ready(rx).await.remove(0);

        assert_eq!(events, output);
    }

    #[tokio::test]
    async fn receive_over_http() {
        run_and_receive(None).await;
    }

    #[tokio::test]
    async fn receive_over_https() {
        run_and_receive(Some(TlsConfig::test_server_config())).await;
    }
}
