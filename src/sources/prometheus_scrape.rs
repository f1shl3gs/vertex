use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use configurable::configurable_component;
use event::{Bucket, EXPORTED_INSTANCE_KEY, INSTANCE_KEY, Metric, Quantile};
use framework::Source;
use framework::config::{Output, SourceConfig, SourceContext, default_interval};
use framework::http::{Auth, HttpClient, HttpError};
use framework::tls::TlsConfig;
use http::{StatusCode, Uri};
use http_body_util::{BodyExt, Full};
use prometheus::{GroupKind, MetricGroup};
use thiserror::Error;
use tokio::task::JoinSet;

/// Collect metrics from prometheus clients.
#[configurable_component(source, name = "prometheus_scrape")]
#[serde(deny_unknown_fields)]
struct Config {
    /// Endpoints to scrape metrics from.
    #[configurable(required, format = "uri", example = "http://example.com/metrics")]
    targets: Vec<String>,

    /// Duration between each scrape.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    /// Controls how tag conflicts are handled if the scraped source has tags
    /// that Vertex would add. If true Vertex will not add the new tag if the
    /// scraped metric has the tag already. If false, Vertex will rename the
    /// conflicting tag by adding "exported_" to it. This matches Prometheus's
    /// "honor_labels" configuration.
    #[serde(default)]
    honor_labels: bool,

    tls: Option<TlsConfig>,

    auth: Option<Auth>,

    /// Global jitterSeed seed is used to spread scrape workload across HA setup.
    #[serde(default)]
    jitter_seed: u64,
}

#[async_trait::async_trait]
#[typetag::serde(name = "prometheus_scrape")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let targets = self
            .targets
            .iter()
            .map(|s| s.parse::<Uri>())
            .collect::<Result<Vec<Uri>, _>>()?;
        let client = HttpClient::new(self.tls.as_ref(), &cx.proxy)?;

        let shutdown = cx.shutdown;
        let output = cx.output;
        let auth = Arc::new(self.auth.clone());
        let interval = self.interval;
        let jitter_seed = self.jitter_seed;
        let honor_labels = self.honor_labels;

        Ok(Box::pin(async move {
            let mut set = JoinSet::new();

            for target in targets {
                let mut output = output.clone();
                let mut shutdown = shutdown.clone();

                let client = client.clone();
                let auth = Arc::clone(&auth);
                let instance = format!(
                    "{}:{}",
                    target.host().unwrap_or_default(),
                    target.port_u16().unwrap_or_else(|| match target.scheme() {
                        Some(scheme) if scheme == &http::uri::Scheme::HTTP => 80,
                        Some(scheme) if scheme == &http::uri::Scheme::HTTPS => 443,
                        _ => 0,
                    })
                );

                set.spawn(async move {
                    let now = Utc::now()
                        .timestamp_nanos_opt()
                        .expect("timestamp can not be represented in a timestamp with nanosecond precision.");
                    let mut ticker = tokio::time::interval_at(
                        tokio::time::Instant::now() + offset(&target, interval, jitter_seed, now),
                        interval,
                    );

                    loop {
                        tokio::select! {
                            _ = &mut shutdown => break,
                            _ = ticker.tick() => {}
                        }

                        let start = Instant::now();
                        let result = scrape_one(&client, auth.as_ref(), &target).await;
                        let elapsed = start.elapsed();

                        let (mut metrics, success) = match result {
                            Ok(metrics) => {
                                if metrics.is_empty() {
                                    warn!(
                                        message = "cannot read or parse metrics",
                                        instance,
                                        internal_log_rate_limit = 60
                                    );
                                }

                                (metrics, true)
                            }
                            Err(err) => {
                                warn!(
                                    message = "scrape metrics failed",
                                    %err,
                                    instance,
                                );

                                (vec![], false)
                            }
                        };

                        metrics.extend([
                            Metric::gauge("up", "", success),
                            Metric::gauge("scrape_duration_seconds", "", elapsed),
                            Metric::gauge("scrape_samples_scraped", "", metrics.len()),
                        ]);

                        // NOTE: timestamp already set in the conversion function, so we don't
                        // need to set it here
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
                });
            }

            set.join_all().await;

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::metrics()]
    }

    fn can_acknowledge(&self) -> bool {
        false
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
fn offset<H: Hash>(h: &H, interval: Duration, jitter_seed: u64, now: i64) -> Duration {
    let hv = calculate_hash(h);
    let base = interval.as_nanos() as i64 - now % interval.as_nanos() as i64;
    let offset = (hv ^ jitter_seed) % interval.as_nanos() as u64;

    let mut next = base + offset as i64;
    if next > interval.as_nanos() as i64 {
        next -= interval.as_nanos() as i64
    }

    Duration::from_nanos(next as u64)
}

#[derive(Debug, Error)]
enum ScrapeError {
    #[error("http error, {0}")]
    Http(#[from] HttpError),

    #[error("unexpected status code {0}")]
    UnexpectedStatusCode(StatusCode),

    #[error("parse metrics failed {0}")]
    Parse(prometheus::Error),
}

async fn scrape_one(
    client: &HttpClient,
    auth: &Option<Auth>,
    uri: &Uri,
) -> Result<Vec<Metric>, ScrapeError> {
    let mut req = http::Request::get(uri)
        .body(Full::default())
        .map_err(HttpError::BuildRequest)?;
    if let Some(auth) = auth {
        auth.apply(&mut req);
    }

    let resp = client.send(req).await.map_err(ScrapeError::Http)?;

    let (header, incoming) = resp.into_parts();
    if header.status != StatusCode::OK {
        return Err(ScrapeError::UnexpectedStatusCode(header.status));
    }

    let data = incoming
        .collect()
        .await
        .map_err(|err| ScrapeError::Http(HttpError::ReadIncoming(err)))?
        .to_bytes();
    let body = String::from_utf8_lossy(&data);

    let metrics = prometheus::parse_text(&body).map_err(ScrapeError::Parse)?;

    Ok(convert_metrics(metrics))
}

fn convert_metrics(groups: Vec<MetricGroup>) -> Vec<Metric> {
    let mut events = Vec::with_capacity(groups.len());
    let start = Utc::now();

    for group in groups {
        let MetricGroup {
            name,
            description,
            metrics,
        } = group;

        match metrics {
            GroupKind::Counter(metrics) => {
                for (key, metric) in metrics {
                    let metric = Metric::sum(&name, &description, metric.value)
                        .with_tags(key.labels.into())
                        .with_timestamp(utc_timestamp(key.timestamp, start));

                    events.push(metric);
                }
            }
            GroupKind::Gauge(metrics) | GroupKind::Untyped(metrics) => {
                for (key, metric) in metrics {
                    let metric = Metric::gauge(&name, &description, metric.value)
                        .with_tags(key.labels.into())
                        .with_timestamp(utc_timestamp(key.timestamp, start));

                    events.push(metric);
                }
            }
            GroupKind::Summary(metrics) => {
                for (key, metric) in metrics {
                    let metric = Metric::summary(
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
                    .with_tags(key.labels.into())
                    .with_timestamp(utc_timestamp(key.timestamp, start));

                    events.push(metric);
                }
            }
            GroupKind::Histogram(metrics) => {
                for (key, metric) in metrics {
                    let metric = Metric::histogram(
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
                    .with_tags(key.labels.into())
                    .with_timestamp(utc_timestamp(key.timestamp, start));

                    events.push(metric);
                }
            }
        }
    }

    events
}

fn utc_timestamp(timestamp: Option<i64>, default: DateTime<Utc>) -> Option<DateTime<Utc>> {
    match timestamp {
        None => Some(default),
        Some(timestamp) => DateTime::<Utc>::from_timestamp_millis(timestamp),
    }
}

#[cfg(test)]
mod tests {
    use framework::config::default_interval;
    use testify::random::random_string;

    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }

    #[test]
    fn spread_offset() {
        let n = 1000;
        let interval = default_interval();

        for _i in 0..n {
            let s = random_string(20);
            let o = offset(&s, interval, 0, 100);
            assert!(o < interval);
        }
    }

    #[test]
    fn equal_offset() {
        let t1 = String::from("boo");
        let t2 = String::from("boo");
        let t3 = String::from("far");

        let interval = default_interval();

        let now = 100;
        let o1 = offset(&t1, interval, 0, now);
        let o2 = offset(&t2, interval, 0, now);
        let o3 = offset(&t3, interval, 0, now);
        assert!(o1 < interval);
        assert!(o2 < interval);
        assert!(o3 < interval);
        assert_eq!(o1, o2);
        assert_ne!(o2, o3);
    }
}
