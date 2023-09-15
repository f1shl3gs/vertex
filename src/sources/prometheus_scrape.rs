use std::borrow::Cow;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use configurable::configurable_component;
use event::{Bucket, Metric, Quantile, EXPORTED_INSTANCE_KEY, INSTANCE_KEY};
use framework::config::{default_interval, DataType, Output, SourceConfig, SourceContext};
use framework::http::{Auth, HttpClient, HttpError};
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::tls::TlsConfig;
use framework::Source;
use http::{StatusCode, Uri};
use prometheus::{GroupKind, MetricGroup};
use thiserror::Error;

/// Collect metrics from prometheus clients.
#[configurable_component(source, name = "prometheus_scrape")]
#[serde(deny_unknown_fields)]
struct Config {
    /// Endpoints to scrape metrics from.
    #[configurable(required, format = "uri", example = "http://example.com/metrics")]
    endpoints: Vec<String>,

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
    jitter_seed: Option<u64>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "prometheus_scrape")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let urls = self
            .endpoints
            .iter()
            .map(|s| {
                s.parse::<Uri>()
                    .map_err(crate::sources::BuildError::UriParseError)
            })
            .collect::<Result<Vec<Uri>, crate::sources::BuildError>>()?;
        let client = HttpClient::new(&self.tls, &cx.proxy)?;

        Ok(scrape(
            client,
            urls,
            self.auth.clone(),
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
    client: HttpClient,
    urls: Vec<Uri>,
    auth: Option<Auth>,
    honor_labels: bool,
    interval: Duration,
    jitter_seed: u64,
    shutdown: ShutdownSignal,
    output: Pipeline,
) -> Source {
    let shutdown = shutdown;

    Box::pin(async move {
        let auth = Arc::new(auth);

        let handles = urls
            .into_iter()
            .map(|url| {
                let client = client.clone();
                let mut shutdown = shutdown.clone();
                let mut output = output.clone();
                let now = Utc::now().timestamp_nanos_opt().expect(
                    "timestamp can not be represented in a timestamp with nanosecond precision.",
                );
                let mut ticker = tokio::time::interval_at(
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
                    loop {
                        tokio::select! {
                            biased;

                            _ = &mut shutdown => break,
                            _ = ticker.tick() => {}
                        }

                        let start = Instant::now();
                        let result = scrape_one(&client, auth.as_ref(), &url).await;
                        let elapsed = start.elapsed();

                        let (mut metrics, success) = match result {
                            Ok(metrics) => {
                                if metrics.is_empty() {
                                    warn!(
                                        message = "cannot read or parse metrics",
                                        ?instance,
                                        internal_log_rate_limit = 60
                                    );
                                }

                                (metrics, true)
                            }
                            Err(err) => {
                                warn!(message = "scrape metrics failed", ?err, ?instance);

                                (vec![], false)
                            }
                        };
                        metrics.extend_from_slice(&[
                            Metric::gauge("up", "", success),
                            Metric::gauge("scrape_duration_seconds", "", elapsed),
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
    url: &Uri,
) -> Result<Vec<Metric>, ScrapeError> {
    let mut req = http::Request::get(url)
        .body(hyper::body::Body::empty())
        .map_err(HttpError::BuildRequest)?;
    if let Some(auth) = auth {
        auth.apply(&mut req);
    }

    let resp = client.send(req).await.map_err(ScrapeError::Http)?;

    let (header, body) = resp.into_parts();
    if header.status != StatusCode::OK {
        return Err(ScrapeError::UnexpectedStatusCode(header.status));
    }

    let data = hyper::body::to_bytes(body)
        .await
        .map_err(|err| ScrapeError::Http(HttpError::CallRequest(err)))?;
    let body = String::from_utf8_lossy(&data);

    let metrics = prometheus::parse_text(&body).map_err(ScrapeError::Parse)?;

    Ok(convert_metrics(metrics))
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
        Some(timestamp) => {
            DateTime::<Utc>::from_timestamp(timestamp / 1000, (timestamp % 1000) as u32 * 1000000)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::sources::prometheus_scrape::{offset, Config};
    use framework::config::default_interval;
    use testify::random::random_string;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>();
    }

    #[test]
    fn spread_offset() {
        let n = 1000;
        let now = chrono::Utc::now().timestamp_nanos_opt().unwrap();
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

        let now = chrono::Utc::now().timestamp_nanos_opt().unwrap();
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
