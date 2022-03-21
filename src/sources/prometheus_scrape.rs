use std::borrow::Cow;
use std::hash::{Hash, Hasher};
use std::ops::Sub;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
use event::{Bucket, Metric, Quantile, EXPORTED_INSTANCE_KEY, INSTANCE_KEY};
use framework::config::{
    default_false, default_interval, deserialize_duration, serialize_duration, DataType,
    GenerateConfig, Output, ProxyConfig, SourceConfig, SourceContext, SourceDescription,
};
use framework::http::{Auth, HttpClient};
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::tls::{MaybeTlsSettings, TlsConfig};
use framework::Source;
use futures::{FutureExt, StreamExt};
use http::{StatusCode, Uri};
use prometheus::{GroupKind, MetricGroup};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use tokio_stream::wrappers::IntervalStream;

// pulled up, and split over multiple lines, because the long lines trip up rustfmt such that it
// gave up trying to format, but reported no error
static PARSE_ERROR_NO_PATH: &str = "No path is set on the endpoint and we got a parse error,\
                                    did you mean to use /metrics? This behavior changed in version 0.11.";
static NOT_FOUND_NO_PATH: &str = "No path is set on the endpoint and we got a 404,\
                                  did you mean to use /metrics?\
                                  This behavior changed in version 0.11.";

#[derive(Debug, Deserialize, Serialize)]
struct PrometheusScrapeConfig {
    endpoints: Vec<String>,
    #[serde(default = "default_interval")]
    #[serde(
        serialize_with = "serialize_duration",
        deserialize_with = "deserialize_duration"
    )]
    interval: std::time::Duration,
    #[serde(default = "default_false")]
    honor_labels: bool,
    tls: Option<TlsConfig>,
    auth: Option<Auth>,

    /// Global jitterSeed seed is used to spread scrape workload across HA setup.
    jitter_seed: Option<u64>,
}

impl GenerateConfig for PrometheusScrapeConfig {
    fn generate_config() -> String {
        format!(
            r#"
# Endpoints to scrape metrics from.
#
endpoints:
- http://localhost:9090/metrics

# The interval between scrapes.
#
# interval: 15s

# Controls how tag conflicts are handled if the scraped source has tags
# that Vertex would add. If true Vertex will not add the new tag if the
# scraped metric has the tag already. If false, Vertex will rename the
# conflicting tag by adding "exported_" to it. This matches Prometheus's
# "honor_labels" configuration.
#
# honor_labels: false

# Configures the TLS options for outgoing connections.
#
# tls:
{}

# Configures the authentication strategy.
#
# auth:
{}

"#,
            TlsConfig::generate_commented_with_indent(2),
            Auth::generate_commented_with_indent(2)
        )
    }
}

inventory::submit! {
    SourceDescription::new::<PrometheusScrapeConfig>("prometheus_scrape")
}

#[async_trait::async_trait]
#[typetag::serde(name = "prometheus_scrape")]
impl SourceConfig for PrometheusScrapeConfig {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let urls = self
            .endpoints
            .iter()
            .map(|s| {
                s.parse::<http::Uri>()
                    .context(crate::sources::UriParseSnafu)
            })
            .collect::<Result<Vec<http::Uri>, crate::sources::BuildError>>()?;
        let tls = MaybeTlsSettings::from_config(&self.tls, true)?;

        Ok(scrape(
            urls,
            tls,
            self.auth.clone(),
            cx.proxy,
            self.honor_labels,
            self.interval,
            self.jitter_seed.unwrap_or_default(),
            cx.shutdown,
            cx.output,
        ))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }

    fn source_type(&self) -> &'static str {
        "prometheus_scrape"
    }
}

fn calculate_hash<T: Hash>(t: &T) -> u64 {
    // Default hasher is not the fastest, but it's totally fine here, cause
    // this func is not in the hot path.
    let mut s = std::collections::hash_map::DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

// offset returns the time until the next scrape cycle for the target.
fn offset<H: Hash>(h: &H, now: i64, interval: Duration, jitter_seed: u64) -> Duration {
    let hv = calculate_hash(h);
    let base = interval.as_nanos() as i64 - now % interval.as_nanos() as i64;
    let offset = (hv ^ jitter_seed) % interval.as_nanos() as u64;
    let mut next = base + offset as i64;
    if next > interval.as_nanos() as i64 {
        next -= interval.as_nanos() as i64
    }

    Duration::from_nanos(next as u64)
}

fn scrape(
    urls: Vec<http::Uri>,
    tls: MaybeTlsSettings,
    auth: Option<Auth>,
    proxy: ProxyConfig,
    honor_labels: bool,
    interval: std::time::Duration,
    jitter_seed: u64,
    shutdown: ShutdownSignal,
    output: Pipeline,
) -> Source {
    let shutdown = shutdown.shared();
    let client = HttpClient::new(tls.clone(), &proxy).expect("Building HTTP client failed");

    Box::pin(async move {
        let auth = Arc::new(auth);

        let handles = urls
            .into_iter()
            .map(|url| {
                let client = client.clone();
                let shutdown = shutdown.clone();
                let mut output = output.clone();
                let now = chrono::Utc::now().timestamp_nanos();
                let interval = tokio::time::interval_at(
                    tokio::time::Instant::now() + offset(&url, now, interval, jitter_seed),
                    interval,
                );
                let auth = Arc::clone(&auth);
                let instance = Cow::from(format!(
                    "{}:{}",
                    url.host().unwrap_or_default(),
                    url.port_u16().unwrap_or_else(|| match url.scheme() {
                        Some(scheme) if scheme == &http::uri::Scheme::HTTP => 80,
                        Some(scheme) if scheme == &http::uri::Scheme::HTTPS => 443,
                        _ => 0,
                    })
                ));

                tokio::spawn(async move {
                    let mut ticker = IntervalStream::new(interval).take_until(shutdown);

                    while ticker.next().await.is_some() {
                        let start = Utc::now();
                        let result = scrape_one(&client, auth.as_ref(), &url).await;
                        let elapsed = Utc::now()
                            .sub(start)
                            .num_nanoseconds()
                            .expect("Nano seconds should not overflow");

                        let success = result.is_ok();
                        let mut metrics = result.unwrap_or_default();
                        metrics.extend_from_slice(&[
                            Metric::gauge("up", "", success),
                            Metric::gauge(
                                "scrape_duration_seconds",
                                "",
                                elapsed as f64 / 1000.0 / 1000.0 / 1000.0,
                            ),
                        ]);

                        metrics.iter_mut().for_each(|metric| {
                            // Handle "instance" overwrite
                            if let Some(value) = metric.remote_tag(&INSTANCE_KEY) {
                                if honor_labels {
                                    metric.insert_tag(EXPORTED_INSTANCE_KEY, value);
                                }
                            }

                            metric.insert_tag(INSTANCE_KEY, instance.clone());
                        });

                        if let Err(err) = output.send_batch(metrics).await {
                            error!(
                                message = "Error sending scraped metrics",
                                %err
                            );

                            return;
                        }
                    }
                })
            })
            .collect::<Vec<_>>();

        futures::future::join_all(handles).await;

        Ok(())
    })
}

async fn scrape_one(
    client: &HttpClient,
    auth: &Option<Auth>,
    url: &Uri,
) -> Result<Vec<Metric>, ()> {
    let mut req = http::Request::get(url)
        .body(hyper::body::Body::empty())
        .expect("error creating request");
    if let Some(auth) = auth {
        auth.apply(&mut req);
    }

    let metrics = match client.send(req).await {
        Ok(resp) => {
            let (header, body) = resp.into_parts();

            if header.status != StatusCode::OK {
                debug!(
                    message = "Target server returned unexpected HTTP status code",
                    target = ?url,
                    status_code = ?header.status,
                );

                return Err(());
            }

            match hyper::body::to_bytes(body).await {
                Ok(data) => {
                    let body = String::from_utf8_lossy(&data);
                    match prometheus::parse_text(&body) {
                        Ok(groups) => convert_metrics(groups),
                        Err(err) => {
                            debug!(
                                message = "Parsing prometheus text failed",
                                ?err,
                                target = ?url,
                                internal_log_rate_secs = 60
                            );

                            return Err(());
                        }
                    }
                }
                Err(err) => {
                    debug!(
                        message = "Read target's response failed",
                        ?err,
                        target = ?url,
                        internal_log_rate_secs = 60
                    );

                    return Err(());
                }
            }
        }
        Err(err) => {
            debug!(
                message = "Request target failed",
                ?err,
                target = ?url,
                internal_log_rate_secs = 60
            );

            return Err(());
        }
    };

    Ok(metrics)
}

fn convert_metrics(groups: Vec<MetricGroup>) -> Vec<Metric> {
    let mut events = Vec::with_capacity(groups.len());
    let start = Utc::now();

    for group in groups {
        let name = &group.name;
        match group.metrics {
            GroupKind::Counter(map) => {
                for (key, metric) in map {
                    let counter = Metric::sum(name, "", metric.value)
                        .with_timestamp(utc_timestamp(key.timestamp, start))
                        .with_tags(key.labels.into());

                    events.push(counter);
                }
            }
            GroupKind::Gauge(metrics) | GroupKind::Untyped(metrics) => {
                for (key, metric) in metrics {
                    let gauge = Metric::gauge(name, "", metric.value)
                        .with_timestamp(utc_timestamp(key.timestamp, start))
                        .with_tags(key.labels.into());

                    events.push(gauge);
                }
            }
            GroupKind::Summary(metrics) => {
                for (key, metric) in metrics {
                    let m = Metric::summary(
                        name,
                        "",
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
                    .with_timestamp(utc_timestamp(key.timestamp, start));

                    events.push(m);
                }
            }
            GroupKind::Histogram(metrics) => {
                for (key, metric) in metrics {
                    let m = Metric::histogram(
                        name,
                        "",
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
                    .with_timestamp(utc_timestamp(key.timestamp, start));

                    events.push(m);
                }
            }
        }
    }

    events
}

fn utc_timestamp(timestamp: Option<i64>, default: DateTime<Utc>) -> Option<DateTime<Utc>> {
    match timestamp {
        None => Some(default),
        Some(timestamp) => Utc
            .timestamp_opt(timestamp / 1000, (timestamp % 1000) as u32 * 1000000)
            .latest(),
    }
}

#[cfg(test)]
mod tests {
    use crate::sources::prometheus_scrape::{offset, PrometheusScrapeConfig};
    use framework::config::default_interval;
    use testify::random::random_string;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<PrometheusScrapeConfig>();
    }

    #[test]
    fn spread_offset() {
        let n = 1000;
        let now = chrono::Utc::now().timestamp_nanos();
        let interval = default_interval();

        for _i in 0..n {
            let s = random_string(20);
            let o = offset(&s, now, interval, 0);
            assert!(o < interval);
        }
    }

    #[test]
    fn equal_offset() {
        let t1 = String::from("boo");
        let t2 = String::from("boo");
        let t3 = String::from("far");

        let now = chrono::Utc::now().timestamp_nanos();
        let interval = default_interval();

        let o1 = offset(&t1, now, interval, 0);
        let o2 = offset(&t2, now, interval, 0);
        let o3 = offset(&t3, now, interval, 0);
        assert!(o1 < interval);
        assert!(o2 < interval);
        assert!(o3 < interval);
        assert_eq!(o1, o2);
        assert_ne!(o2, o3);
    }
}
