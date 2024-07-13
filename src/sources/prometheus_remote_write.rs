use std::cmp::Ordering;
use std::net::SocketAddr;

use bytes::Bytes;
use chrono::{DateTime, Utc};
use configurable::configurable_component;
use event::{Bucket, Event, Metric, Quantile};
use framework::config::{Output, Resource, SourceConfig, SourceContext};
use framework::source::util::http::{decode, ErrorMessage};
use framework::source::util::http::{HttpSource, HttpSourceAuthConfig};
use framework::{tls::TlsConfig, Source};
use http::header::CONTENT_ENCODING;
use http::{HeaderMap, Method, StatusCode, Uri};
use prometheus::{proto, GroupKind, MetricGroup};
use prost::Message;

/// Start a HTTP server and receive Protobuf encoded metrics.
#[configurable_component(source, name = "prometheus_remote_write")]
#[serde(deny_unknown_fields)]
struct Config {
    /// The address to accept connections on. The address must include a port
    #[configurable(required)]
    address: SocketAddr,

    /// HTTP Server TLS config
    tls: Option<TlsConfig>,

    auth: Option<HttpSourceAuthConfig>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "prometheus_remote_write")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let source = RemoteWriteSource;

        source
            .run(
                self.address,
                Method::POST,
                "/write",
                &self.tls,
                &self.auth,
                cx,
            )
            .await
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::metrics()]
    }

    fn resources(&self) -> Vec<Resource> {
        vec![Resource::tcp(self.address)]
    }
}

#[derive(Clone)]
struct RemoteWriteSource;

impl RemoteWriteSource {
    fn decode_body(&self, body: Bytes) -> Result<Vec<Event>, ErrorMessage> {
        let req = proto::WriteRequest::decode(body).map_err(|err| {
            ErrorMessage::new(
                StatusCode::BAD_REQUEST,
                format!("Could not decode write request: {}", err),
            )
        })?;

        let metrics = prometheus::parse_request(req).map_err(|err| {
            ErrorMessage::new(
                StatusCode::BAD_REQUEST,
                format!("Could not decode write request, err: {:?}", err),
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
        mut body: Bytes,
    ) -> Result<Vec<Event>, ErrorMessage> {
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

fn reparse_groups(groups: Vec<MetricGroup>) -> Vec<Event> {
    let mut result = Vec::new();
    let start = Utc::now();

    for group in groups {
        match group.metrics {
            GroupKind::Counter(metrics) => {
                for (key, metric) in metrics {
                    let counter =
                        Metric::sum_with_tags(group.name.clone(), "", metric.value, key.labels)
                            .with_timestamp(utc_timestamp(key.timestamp, start));

                    result.push(counter.into())
                }
            }

            GroupKind::Gauge(metrics) | GroupKind::Untyped(metrics) => {
                for (key, metric) in metrics {
                    let gauge =
                        Metric::gauge_with_tags(group.name.clone(), "", metric.value, key.labels)
                            .with_timestamp(utc_timestamp(key.timestamp, start));

                    result.push(gauge.into())
                }
            }

            GroupKind::Histogram(metrics) => {
                for (key, metric) in metrics {
                    let mut buckets = metric.buckets;
                    buckets.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));

                    let histogram = Metric::histogram_with_tags(
                        group.name.clone(),
                        "",
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

                    result.push(histogram.into());
                }
            }

            GroupKind::Summary(metrics) => {
                for (key, metric) in metrics {
                    let summary = Metric::summary(
                        group.name.clone(),
                        "",
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

                    result.push(summary.into())
                }
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;
    use chrono::{SubsecRound, Utc};
    use event::{buckets, quantiles, tags, EventStatus, Metric};
    use framework::config::ProxyConfig;
    use framework::http::HttpClient;
    use framework::pipeline::Pipeline;
    use framework::tls::TlsConfig;
    use http_body_util::Full;
    use testify::{assert_event_data_eq, collect_ready};

    use super::*;
    use crate::common::prometheus::TimeSeries;
    use crate::testing::components;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>();
    }

    fn make_events() -> Vec<Event> {
        let ts = || Utc::now().trunc_subsecs(3);

        let metrics = vec![
            Metric::sum_with_tags("counter_1", "", 42, tags!("type" => "counter")),
            Metric::gauge_with_tags("gauge_2", "", 42, tags!("type" => "gauge")),
            Metric::histogram_with_tags(
                "histogram_3",
                "",
                tags!("type" => "histogram"),
                96_u64,
                156.2,
                buckets!(
                    2.3 => 11,
                    4.2 => 85
                ),
            ),
            Metric::summary(
                "summary_4",
                "",
                23_u64,
                8.6,
                quantiles!(
                    0.1 => 1.2,
                    0.5 => 3.6,
                    0.9 => 5.2
                ),
            )
            .with_tags(tags!("type" => "summary")),
        ];

        metrics
            .into_iter()
            .map(|mut m| {
                m.timestamp = Some(ts());
                Event::from(m)
            })
            .collect::<Vec<_>>()
    }

    async fn receives_metrics(tls: Option<TlsConfig>) {
        components::init_test();
        let address = testify::next_addr();
        let (tx, rx) = Pipeline::new_test_finalize(EventStatus::Delivered);

        let source = Config {
            address,
            auth: None,
            tls: tls.clone(),
        };

        let source = source.build(SourceContext::new_test(tx)).await.unwrap();
        tokio::spawn(source);

        let client = HttpClient::new(&tls, &ProxyConfig::default()).unwrap();
        let url = format!(
            "{}://localhost:{}/write",
            if tls.is_some() { "https" } else { "http" },
            address.port()
        );

        let events = make_events();
        let mut timeseries = TimeSeries::new();
        for event in events.clone() {
            let metric = event.as_metric();
            timeseries.encode_metric(metric);
        }

        let wr = timeseries.finish();
        let mut out = BytesMut::with_capacity(wr.encoded_len());
        wr.encode(&mut out).expect("Out of memory");

        let body = out.freeze();

        let req = http::Request::post(&url).body(Full::new(body)).unwrap();

        let resp = client.send(req).await.unwrap();
        assert!(resp.status().is_success());

        let output = collect_ready(rx).await;

        assert_event_data_eq!(events, output);
    }

    #[tokio::test]
    async fn receives_metrics_over_http() {
        receives_metrics(None).await;
    }
}
