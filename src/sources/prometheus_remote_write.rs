use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::net::SocketAddr;

use bytes::Bytes;
use chrono::{DateTime, TimeZone, Utc};
use event::{tags, Bucket, Event, Metric, MetricValue, Quantile};
use http::{HeaderMap, Method, StatusCode, Uri};
use prometheus::{proto, GroupKind, MetricGroup, METRIC_NAME_LABEL};
use prost::Message;
use serde::{Deserialize, Serialize};

use crate::config::{
    DataType, GenerateConfig, Output, Resource, SourceConfig, SourceContext, SourceDescription,
};
use crate::sources::utils::http::{decode, ErrorMessage};
use crate::sources::{
    utils::http::{HttpSource, HttpSourceAuthConfig},
    Source,
};
use crate::tls::TlsConfig;

const SOURCE_NAME: &str = "prometheus_remote_write";

fn default_address() -> SocketAddr {
    "0.0.0.0:9090".parse().unwrap()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PrometheusRemoteWriteConfig {
    address: SocketAddr,
    tls: Option<TlsConfig>,
    auth: Option<HttpSourceAuthConfig>,

    acknowledgements: bool,
}

impl GenerateConfig for PrometheusRemoteWriteConfig {
    fn generate_config() -> serde_yaml::Value {
        serde_yaml::to_value(Self {
            address: default_address(),
            tls: None,
            auth: None,
            acknowledgements: false,
        })
        .unwrap()
    }
}

inventory::submit! {
    SourceDescription::new::<PrometheusRemoteWriteConfig>(SOURCE_NAME)
}

#[async_trait::async_trait]
#[typetag::serde(name = "prometheus_remote_write")]
impl SourceConfig for PrometheusRemoteWriteConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        let source = RemoteWriteSource;

        source
            .run(
                self.address,
                Method::POST,
                "/write",
                &self.tls,
                &self.auth,
                ctx,
                self.acknowledgements,
            )
            .await
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }

    fn source_type(&self) -> &'static str {
        SOURCE_NAME
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
            .get("Content-Encoding")
            .map(|header| header.as_ref())
            != None
        {
            body = decode(Some("snappy"), body)?;
        }

        self.decode_body(body)
    }
}

fn utc_timestamp(timestamp: Option<i64>, default: DateTime<Utc>) -> Option<DateTime<Utc>> {
    match timestamp {
        None => Some(default),
        Some(timestamp) => Utc
            .timestamp_opt(timestamp / 1000, (timestamp % 1000) as u32 * 1000000)
            .latest(),
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
                    .with_tags(key.labels)
                    .with_timestamp(utc_timestamp(key.timestamp, start));

                    result.push(summary.into())
                }
            }
        }
    }

    result
}

use indexmap::IndexMap;

type Labels = Vec<proto::Label>;

pub struct TimeSeries {
    buffer: IndexMap<Labels, Vec<proto::Sample>>,
    metadata: IndexMap<String, proto::MetricMetadata>,
    timestamp: Option<i64>,
}

impl TimeSeries {
    fn new() -> Self {
        Self {
            buffer: Default::default(),
            metadata: Default::default(),
            timestamp: None,
        }
    }

    fn default_timestamp(&mut self) -> i64 {
        *self
            .timestamp
            .get_or_insert_with(|| Utc::now().timestamp_millis())
    }

    fn emit_metadata(&mut self, name: &str, fullname: &str, value: &MetricValue) {
        if !self.metadata.contains_key(name) {
            let r#type = prometheus_metric_type(value);
            let metadata = proto::MetricMetadata {
                r#type: r#type as i32,
                metric_family_name: fullname.into(),
                help: name.into(),
                unit: String::new(),
            };

            self.metadata.insert(name.into(), metadata);
        }
    }

    fn emit_value(
        &mut self,
        timestamp: Option<i64>,
        name: &str,
        suffix: Option<&str>,
        value: f64,
        tags: &BTreeMap<String, String>,
        extra: Option<(&str, String)>,
    ) {
        let timestamp = timestamp.unwrap_or_else(|| self.default_timestamp());
        self.buffer
            .entry(Self::make_labels(tags, name, suffix, extra))
            .or_default()
            .push(proto::Sample { value, timestamp });
    }

    fn finish(self) -> proto::WriteRequest {
        let timeseries = self
            .buffer
            .into_iter()
            .map(|(labels, samples)| proto::TimeSeries { labels, samples })
            .collect::<Vec<_>>();

        let metadata = self
            .metadata
            .into_iter()
            .map(|(_, metadata)| metadata)
            .collect();

        proto::WriteRequest {
            timeseries,
            metadata,
        }
    }

    fn encode_metric(&mut self, buckets: &[f64], quantiles: &[f64], metric: &Metric) {
        let name = metric.name();
        let timestamp = metric.timestamp().map(|ts| ts.timestamp_millis());
        let tags = &metric.tags;
        self.emit_metadata(name, name, &metric.value);

        match &metric.value {
            MetricValue::Sum(value) | MetricValue::Gauge(value) => {
                self.emit_value(timestamp, name, None, *value, tags, None)
            }
            MetricValue::Histogram {
                count,
                sum,
                buckets,
            } => {
                for bucket in buckets {
                    self.emit_value(
                        timestamp,
                        name,
                        Some("_bucket"),
                        bucket.count as f64,
                        tags,
                        Some(("le", bucket.upper.to_string())),
                    );
                }

                self.emit_value(timestamp, name, Some("_sum"), *sum, tags, None);
                self.emit_value(timestamp, name, Some("_count"), *count as f64, tags, None);
            }
            MetricValue::Summary {
                count,
                sum,
                quantiles,
            } => {
                for quantile in quantiles {
                    self.emit_value(
                        timestamp,
                        name,
                        None,
                        quantile.value,
                        tags,
                        Some(("quantile", quantile.quantile.to_string())),
                    )
                }

                self.emit_value(timestamp, name, Some("_sum"), *sum, tags, None);
                self.emit_value(timestamp, name, Some("_count"), *count as f64, tags, None);
            }
        }
    }

    fn make_labels(
        tags: &BTreeMap<String, String>,
        name: &str,
        suffix: Option<&str>,
        extra: Option<(&str, String)>,
    ) -> Labels {
        // Each Prometheus metric is grouped by its labels, which contains all the labels
        // from the source metric, plus the name label for the actual metric name. For
        // convenience below, an optional extra tag is added.
        let mut labels = tags.clone();
        let name = match suffix {
            Some(suffix) => [name, suffix].join(""),
            None => name.to_string(),
        };

        labels.insert(METRIC_NAME_LABEL.into(), name);

        if let Some((name, value)) = extra {
            labels.insert(name.to_string(), value);
        }

        // Extract the labels into a vec and sort to produce a consistent key for the
        // buffer
        let mut labels = labels
            .into_iter()
            .map(|(name, value)| proto::Label { name, value })
            .collect::<Labels>();

        labels.sort();
        labels
    }
}

const fn prometheus_metric_type(value: &MetricValue) -> proto::MetricType {
    match value {
        MetricValue::Sum(_) => proto::MetricType::Counter,
        MetricValue::Gauge(_) => proto::MetricType::Gauge,
        MetricValue::Histogram { .. } => proto::MetricType::Histogram,
        MetricValue::Summary { .. } => proto::MetricType::Summary,
    }
}

fn default_histogram_buckets() -> Vec<f64> {
    vec![
        0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
    ]
}

fn default_summary_quantiles() -> Vec<f64> {
    vec![0.5, 0.75, 0.9, 0.95, 0.99]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProxyConfig;
    use crate::http::HttpClient;
    use crate::pipeline::Pipeline;
    use crate::testing::components;
    use crate::tls::MaybeTlsSettings;
    use bytes::BytesMut;
    use chrono::{SubsecRound, Utc};
    use event::encoding::Encoder;
    use event::{assert_event_data_eq, buckets, quantiles, EventStatus, Metric};
    use hyper::Body;
    use testify::collect_ready;

    #[test]
    fn generate_config() {
        crate::config::test_generate_config::<PrometheusRemoteWriteConfig>();
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

        let source = PrometheusRemoteWriteConfig {
            address,
            auth: None,
            tls: tls.clone(),
            acknowledgements: false,
        };

        let source = source.build(SourceContext::new_test(tx)).await.unwrap();
        tokio::spawn(source);

        let tls_settings = MaybeTlsSettings::from_config(&tls, false).unwrap();
        let client = HttpClient::new(tls_settings, &ProxyConfig::default()).unwrap();
        let url = format!(
            "{}://localhost:{}/write",
            if tls.is_some() { "https" } else { "http" },
            address.port()
        );

        let events = make_events();
        let mut timeseries = TimeSeries::new();
        let buckets = default_histogram_buckets();
        let quantiles = default_summary_quantiles();

        for event in events.clone() {
            let metric = event.as_metric();
            timeseries.encode_metric(&buckets, &quantiles, metric);
        }

        let wr = timeseries.finish();
        let mut out = BytesMut::with_capacity(wr.encoded_len());
        wr.encode(&mut out).expect("Out of memory");

        let body = out.freeze();

        let req = http::Request::post(&url).body(Body::from(body)).unwrap();

        let resp = client.send(req).await.unwrap();

        let output = collect_ready(rx).await;

        assert_event_data_eq!(events, output);
    }

    #[tokio::test]
    async fn receives_metrics_over_http() {
        receives_metrics(None).await;
    }

    #[tokio::test]
    async fn receives_metrics_over_https() {
        receives_metrics(Some(TlsConfig::test_config())).await;
    }
}

#[cfg(test)]
mod integration_tests {}
