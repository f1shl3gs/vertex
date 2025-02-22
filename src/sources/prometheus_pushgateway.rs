use std::net::SocketAddr;

use base64::Engine;
use base64::prelude::BASE64_URL_SAFE;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use configurable::configurable_component;
use event::{Bucket, Events, Metric, Quantile};
use framework::Source;
use framework::config::{Output, Resource, SourceConfig, SourceContext};
use framework::source::http::{ErrorMessage, HttpSource, HttpSourceAuthConfig};
use framework::tls::TlsConfig;
use http::{HeaderMap, Method, StatusCode, Uri};
use prometheus::{GroupKind, MetricGroup};

/// Configuration for the `prometheus_pushgateway` source.
#[configurable_component(source, name = "prometheus_pushgateway")]
struct Config {
    /// The address to accept connections on.
    address: SocketAddr,

    tls: Option<TlsConfig>,

    auth: Option<HttpSourceAuthConfig>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "prometheus_pushgateway")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let source = PushgatewaySource;

        source.run(
            self.address,
            Method::POST,
            "/metrics/job",
            false,
            self.tls.as_ref(),
            self.auth.as_ref(),
            cx,
        )
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::metrics()]
    }

    fn resources(&self) -> Vec<Resource> {
        vec![Resource::tcp(self.address)]
    }

    fn can_acknowledge(&self) -> bool {
        true
    }
}

#[derive(Clone)]
struct PushgatewaySource;

impl HttpSource for PushgatewaySource {
    fn build_events(
        &self,
        uri: &Uri,
        _headers: &HeaderMap,
        peer_addr: &SocketAddr,
        body: Bytes,
    ) -> Result<Events, ErrorMessage> {
        let mut extra_labels = parse_path_labels(uri.path())?;
        if !extra_labels.iter().any(|(key, _value)| key == "instance") {
            extra_labels.push(("instance".to_string(), peer_addr.ip().to_string()))
        }

        let data = String::from_utf8_lossy(&body);
        let metric_group = prometheus::parse_text(data.as_ref())
            .map_err(|err| ErrorMessage::new(StatusCode::UNPROCESSABLE_ENTITY, err.to_string()))?;

        Ok(convert_metrics(metric_group, &extra_labels))
    }
}

fn convert_metrics(groups: Vec<MetricGroup>, extra_labels: &[(String, String)]) -> Events {
    let mut metrics = Vec::with_capacity(groups.len());
    let start = Utc::now();

    for MetricGroup {
        name,
        description,
        metrics: group,
    } in groups
    {
        match group {
            GroupKind::Counter(map) => {
                for (key, metric) in map {
                    let mut counter = Metric::sum(&name, &description, metric.value)
                        .with_timestamp(utc_timestamp(key.timestamp, start))
                        .with_tags(key.labels.into());

                    for (key, value) in extra_labels {
                        counter.tags_mut().insert(key.clone(), value.clone());
                    }

                    metrics.push(counter);
                }
            }
            GroupKind::Gauge(map) | GroupKind::Untyped(map) => {
                for (key, metric) in map {
                    let mut gauge = Metric::gauge(&name, &description, metric.value)
                        .with_timestamp(utc_timestamp(key.timestamp, start))
                        .with_tags(key.labels.into());

                    for (key, value) in extra_labels {
                        gauge.tags_mut().insert(key.clone(), value.clone());
                    }

                    metrics.push(gauge);
                }
            }
            GroupKind::Summary(map) => {
                for (key, metric) in map {
                    let mut summary = Metric::summary(
                        &name,
                        &description,
                        metric.count,
                        metric.sum,
                        metric
                            .quantiles
                            .iter()
                            .map(|q| Quantile {
                                quantile: q.quantile,
                                value: q.value,
                            })
                            .collect::<Vec<_>>(),
                    )
                    .with_timestamp(utc_timestamp(key.timestamp, start))
                    .with_tags(key.labels.into());

                    for (key, value) in extra_labels {
                        summary.tags_mut().insert(key.clone(), value.clone());
                    }

                    metrics.push(summary);
                }
            }
            GroupKind::Histogram(map) => {
                for (key, metric) in map {
                    let mut histogram = Metric::histogram(
                        &name,
                        &description,
                        metric.count,
                        metric.sum,
                        metric
                            .buckets
                            .iter()
                            .map(|b| Bucket {
                                upper: b.bucket,
                                count: b.count,
                            })
                            .collect::<Vec<_>>(),
                    )
                    .with_timestamp(utc_timestamp(key.timestamp, start))
                    .with_tags(key.labels.into());

                    for (key, value) in extra_labels {
                        histogram.tags_mut().insert(key.clone(), value.clone());
                    }

                    metrics.push(histogram);
                }
            }
        }
    }

    metrics.into()
}

fn utc_timestamp(timestamp: Option<i64>, default: DateTime<Utc>) -> Option<DateTime<Utc>> {
    match timestamp {
        None => Some(default),
        Some(timestamp) => {
            DateTime::<Utc>::from_timestamp(timestamp / 1000, (timestamp % 1000) as u32 * 1000000)
        }
    }
}

fn parse_path_labels(path: &str) -> Result<Vec<(String, String)>, ErrorMessage> {
    if !path.starts_with("/metrics/job") {
        return Err(ErrorMessage::new(
            StatusCode::BAD_REQUEST,
            "Path must begin with '/metrics/job'",
        ));
    }

    let mut segments = path.split('/').skip(2);
    let mut labels = vec![];

    while let Some(key) = segments.next() {
        let value = match segments.next() {
            Some(value) => value,
            None => {
                return Err(ErrorMessage::new(
                    StatusCode::BAD_REQUEST,
                    "Request path must have an even number of segments to form grouping key",
                ));
            }
        };

        labels.push(decode_label_pair(key, value)?);
    }

    Ok(labels)
}

fn decode_label_pair(key: &str, value: &str) -> Result<(String, String), ErrorMessage> {
    // Return early if we're not dealing with a base64-encoded label
    let Some(stripped_key) = key.strip_suffix("@base64") else {
        return Ok((key.to_owned(), value.to_owned()));
    };

    // The Prometheus Pushgateway spec explicitly uses one or more `=` characters
    // (the padding character in base64) to represent an empty string in a path
    // segment:
    //
    // https://github.com/prometheus/pushgateway/blob/ec7afda4eef288bd9b9c43d063e4df54c8961272/README.md#url
    //
    // Unfortunately, the Rust base64 crate doesn't treat an encoded string that
    // only contains padding characters as valid and returns an error.
    //
    // Let's handle this case manually, before handing over to the base64 decoder.
    if value.chars().all(|c| c == '=') {
        // An empty job label isn't valid, so return an error if that's the key
        if stripped_key == "job" {
            return Err(ErrorMessage::new(
                StatusCode::BAD_REQUEST,
                "Job must not have an empty value",
            ));
        }

        return Ok((stripped_key.to_owned(), "".to_owned()));
    }

    // The Prometheus Pushgateway has a fairly permissive base64 implementation
    // that allows padding to be missing. We need to fake that by adding in any
    // missing padding before we pass the value to the base64 decoder.
    //
    // This is documented, as example in their README don't use padding:
    //
    // https://github.com/prometheus/pushgateway/blob/ec7afda4eef288bd9b9c43d063e4df54c8961272/README.md#url
    let missing_padding = value.len() % 4;
    let padded_value = if missing_padding == 0 {
        value.to_owned()
    } else {
        let padding = "=".repeat(missing_padding);
        value.to_owned() + &padding
    };

    let decoded_bytes = BASE64_URL_SAFE.decode(padded_value).map_err(|_err| {
        ErrorMessage::new(
            StatusCode::BAD_REQUEST,
            format!(
                "Grouping key invalid - invalid base64 value for key {}: {}",
                key, value
            ),
        )
    })?;

    let decoded = String::from_utf8(decoded_bytes).map_err(|_err| {
        ErrorMessage::new(
            StatusCode::BAD_REQUEST,
            format!(
                "Grouping key invalid - invalid UTF-8 in decoded base64 value for key {}",
                key
            ),
        )
    })?;

    Ok((stripped_key.to_owned(), decoded))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::{TimeZone, Timelike};
    use event::{EventStatus, tags};
    use framework::Pipeline;
    use framework::config::ProxyConfig;
    use framework::http::HttpClient;
    use http::Request;
    use http_body_util::{BodyExt, Full};
    use testify::collect_ready;

    use super::*;

    #[test]
    fn config() {
        crate::testing::generate_config::<Config>();
    }

    #[test]
    fn parse_simple_path() {
        let path = "/metrics/job/foo/instance/bar";
        let expected: Vec<_> = vec![("job", "foo"), ("instance", "bar")]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        let actual = parse_path_labels(path);

        assert!(actual.is_ok());
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn parse_path_wrong_number_of_segments() {
        let path = "/metrics/job/foo/instance";
        let result = parse_path_labels(path);

        assert!(result.is_err());
        assert!(result.unwrap_err().message().contains("number of segments"));
    }

    #[test]
    fn parse_path_with_base64_segment() {
        let path = "/metrics/job/foo/instance@base64/YmFyL2Jheg==";
        let expected: Vec<_> = vec![("job", "foo"), ("instance", "bar/baz")]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        let actual = parse_path_labels(path);

        assert!(actual.is_ok());
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn parse_path_with_base64_segment_missing_padding() {
        let path = "/metrics/job/foo/instance@base64/YmFyL2Jheg";
        let expected: Vec<_> = vec![("job", "foo"), ("instance", "bar/baz")]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        let actual = parse_path_labels(path);

        assert!(actual.is_ok());
        assert_eq!(actual.unwrap(), expected);
    }

    #[test]
    fn parse_path_empty_job_name_invalid() {
        let path = "/metrics/job@base64/=";
        let result = parse_path_labels(path);

        assert!(result.is_err());
        assert!(result.unwrap_err().message().contains("Job must not"));
    }

    #[test]
    fn parse_path_empty_path_invalid() {
        let path = "/";
        let result = parse_path_labels(path);

        assert!(result.is_err());
        assert!(result.unwrap_err().message().contains("Path must begin"));
    }

    // This is to ensure that the last value for a given key is the one used when we
    // pass the grouping key into the Prometheus text parser to override label values
    // on individual metrics
    #[test]
    fn parse_path_duplicate_labels_preserves_order() {
        let path = "/metrics/job/foo/instance/bar/instance/baz";
        let expected: Vec<_> = vec![("job", "foo"), ("instance", "bar"), ("instance", "baz")]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        let actual = parse_path_labels(path);

        assert!(actual.is_ok());
        assert_eq!(actual.unwrap(), expected);
    }

    async fn assert_compliance(tls: Option<TlsConfig>) {
        let push_body = r#"# HELP jobs_total Total number of jobs
# TYPE jobs_total counter
jobs_total{type="a"} 1.0 1612411506789
# HELP jobs_current Current number of jobs
# TYPE jobs_current gauge
jobs_current{type="a"} 5.0 1612411506789
# HELP jobs_distribution Distribution of jobs
# TYPE jobs_distribution histogram
jobs_distribution_bucket{type="a",le="1"} 0.0 1612411506789
jobs_distribution_bucket{type="a",le="2.5"} 0.0 1612411506789
jobs_distribution_bucket{type="a",le="5"} 0.0 1612411506789
jobs_distribution_bucket{type="a",le="10"} 1.0 1612411506789
jobs_distribution_bucket{type="a",le="+Inf"} 1.0 1612411506789
jobs_distribution_sum{type="a"} 8.0 1612411506789
jobs_distribution_count{type="a"} 1.0 1612411506789
# HELP jobs_summary Summary of jobs
# TYPE jobs_summary summary
jobs_summary_bucket{type="a",quantile="1.0"} 1 1612411506789
jobs_summary_sum{type="a"} 8.0 1612411506789
jobs_summary_count{type="a"} 1.0 1612411506789
"#;

        // start server
        let address = testify::next_addr();
        let (tx, rx) = Pipeline::new_test_finalize(EventStatus::Delivered);
        let source = Config {
            address,
            tls: tls.clone(),
            auth: None,
        };

        let source = source.build(SourceContext::new_test(tx)).await.unwrap();
        tokio::spawn(source);

        // wait for source start http server
        tokio::time::sleep(Duration::from_secs(1)).await;

        // post metrics
        let client = HttpClient::new(
            Some(&TlsConfig::test_client_config()),
            &ProxyConfig::default(),
        )
        .unwrap();
        let url = format!(
            "{}://localhost:{}/metrics/job/foo",
            if tls.is_some() { "https" } else { "http" },
            address.port()
        );
        let req = Request::builder()
            .uri(url)
            .method(Method::POST)
            .body(Full::new(Bytes::from(push_body)))
            .unwrap();

        let resp = client.send(req).await.unwrap();
        let (parts, incoming) = resp.into_parts();
        let body = incoming.collect().await.unwrap().to_bytes();
        let body = std::str::from_utf8(&body).unwrap();
        println!("{}", body);
        assert_eq!(parts.status, StatusCode::OK);

        let got = collect_ready(rx)
            .await
            .into_iter()
            .flat_map(|events| events.into_metrics())
            .flatten()
            .collect::<Vec<_>>();

        let timestamp = Utc
            .with_ymd_and_hms(2021, 2, 4, 4, 5, 6)
            .single()
            .and_then(|t| t.with_nanosecond(789 * 1_000_000))
            .expect("invalid timestamp");

        assert_eq!(
            got,
            vec![
                Metric::sum("jobs_total", "Total number of jobs", 1.0)
                    .with_timestamp(Some(timestamp))
                    .with_tags(tags!("instance" => "127.0.0.1", "type" => "a", "job" => "foo")),
                Metric::gauge("jobs_current", "Current number of jobs", 5.0)
                    .with_timestamp(Some(timestamp))
                    .with_tags(tags!("instance" => "127.0.0.1", "type" => "a", "job" => "foo")),
                Metric::histogram(
                    "jobs_distribution",
                    "Distribution of jobs",
                    1u64,
                    8.0,
                    vec![
                        Bucket {
                            upper: 1.0,
                            count: 0,
                        },
                        Bucket {
                            upper: 2.5,
                            count: 0,
                        },
                        Bucket {
                            upper: 5.0,
                            count: 0,
                        },
                        Bucket {
                            upper: 10.0,
                            count: 1
                        },
                        Bucket {
                            upper: f64::INFINITY,
                            count: 1,
                        }
                    ]
                )
                .with_timestamp(Some(timestamp))
                .with_tags(tags!("instance" => "127.0.0.1", "job" => "foo", "type" => "a")),
                Metric::summary("jobs_summary", "Summary of jobs", 1u64, 8.0, vec![])
                    .with_timestamp(Some(timestamp))
                    .with_tags(tags!("instance" => "127.0.0.1", "job" => "foo", "type" => "a"))
            ]
        )
    }

    #[tokio::test]
    async fn receive_over_http() {
        assert_compliance(None).await;
    }

    #[tokio::test]
    async fn receive_over_https() {
        assert_compliance(Some(TlsConfig::test_server_config())).await;
    }
}
