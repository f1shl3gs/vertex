use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;
use std::fmt::Write;
use std::hash::Hasher;
use std::io::Write as IoWrite;
use std::net::SocketAddr;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::{BufMut, BytesMut};
use chrono::Utc;
use configurable::configurable_component;
use event::{EventStatus, Events, Finalizable, Metric, MetricValue};
use framework::config::{DataType, Resource, SinkConfig, SinkContext};
use framework::tls::{MaybeTlsListener, TlsConfig};
use framework::{Healthcheck, Sink, StreamSink};
use futures::prelude::stream::BoxStream;
use futures::{FutureExt, StreamExt};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use parking_lot::Mutex;
use tripwire::Tripwire;

#[configurable_component(sink, name = "prometheus_exporter")]
#[serde(deny_unknown_fields)]
struct Config {
    /// The address the prometheus server will listen at
    #[serde(default = "default_endpoint")]
    #[configurable(required, format = "ip-address", example = "0.0.0.0:9100")]
    endpoint: SocketAddr,

    /// TTL for metrics, any metrics not received for ttl will be removed
    /// from cache.
    #[serde(default = "default_ttl")]
    #[serde(with = "humanize::duration::serde")]
    ttl: Duration,

    tls: Option<TlsConfig>,
}

fn default_endpoint() -> SocketAddr {
    SocketAddr::from(([0, 0, 0, 0], 9100))
}

const fn default_ttl() -> Duration {
    Duration::from_secs(5 * 60)
}

#[async_trait]
#[typetag::serde(name = "prometheus_exporter")]
impl SinkConfig for Config {
    async fn build(&self, _cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        let sink = PrometheusExporter::new(self);
        let health_check = futures::future::ok(()).boxed();

        Ok((Sink::Stream(Box::new(sink)), health_check))
    }

    fn input_type(&self) -> DataType {
        DataType::Metric
    }

    fn resources(&self) -> Vec<Resource> {
        vec![Resource::tcp(self.endpoint)]
    }
}

struct ExpiringEntry {
    metric: Metric,
    expired_at: i64,
}

impl Deref for ExpiringEntry {
    type Target = Metric;

    fn deref(&self) -> &Self::Target {
        &self.metric
    }
}

impl DerefMut for ExpiringEntry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.metric
    }
}

impl std::hash::Hash for ExpiringEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.series.hash(state)
    }
}

impl PartialEq<Self> for ExpiringEntry {
    fn eq(&self, other: &Self) -> bool {
        self.series.eq(&other.series)
    }
}

impl Eq for ExpiringEntry {}

impl PartialOrd<Self> for ExpiringEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.series.partial_cmp(&other.series)
    }
}

impl Ord for ExpiringEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        let a = self.tags();
        let b = other.tags();

        a.partial_cmp(b).unwrap_or(Ordering::Greater)
    }
}

#[derive(Default)]
struct Sets {
    description: String,
    metrics: BTreeSet<ExpiringEntry>,
}

struct PrometheusExporter {
    ttl: Duration,
    tls: Option<TlsConfig>,
    endpoint: SocketAddr,
    metrics: Arc<Mutex<BTreeMap<String, Sets>>>,
}

impl PrometheusExporter {
    fn new(config: &Config) -> Self {
        Self {
            endpoint: config.endpoint,
            metrics: Arc::new(Mutex::new(BTreeMap::new())),
            ttl: config.ttl,
            tls: config.tls.clone(),
        }
    }
}

macro_rules! write_metric {
    ($dst:expr, $name:expr, $tags:expr, $value:expr) => {
        if $tags.is_empty() {
            writeln!(&mut $dst, "{} {}", $name.to_owned(), $value).unwrap();
        } else {
            writeln!(
                &mut $dst,
                "{}{{{}}} {}",
                $name,
                $tags
                    .iter()
                    .map(|(k, v)| format!("{}=\"{}\"", k, v))
                    .collect::<Vec<String>>()
                    .join(","),
                $value
            )
            .unwrap();
        }
    };
}

fn handle(
    req: Request<Body>,
    metrics: Arc<Mutex<BTreeMap<String, Sets>>>,
    now: i64,
) -> Response<Body> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/metrics") => {
            let mut buf = BytesMut::with_capacity(8 * 1024);

            metrics
                .lock()
                .iter()
                .filter_map(|(name, sets)| match sets.metrics.iter().next() {
                    None => None,
                    Some(entry) => {
                        let kind = match entry.value {
                            MetricValue::Gauge(_) => "gauge",
                            MetricValue::Sum(_) => "counter",
                            MetricValue::Histogram { .. } => "histogram",
                            MetricValue::Summary { .. } => "summary",
                        };

                        Some((name, kind, &sets.description, &sets.metrics))
                    }
                })
                .for_each(|(name, kind, description, metrics)| {
                    let mut header = false;

                    for entry in metrics {
                        let ExpiringEntry { metric, expired_at } = entry;
                        if *expired_at < now {
                            continue;
                        }

                        if !header {
                            writeln!(
                                &mut buf,
                                "# HELP {} {}\n# TYPE {} {}",
                                name, description, name, kind
                            )
                            .unwrap();
                            header = true;
                        }

                        match &metric.value {
                            MetricValue::Gauge(v) | MetricValue::Sum(v) => {
                                write_metric!(buf, metric.name(), metric.tags(), *v);
                            }
                            MetricValue::Summary {
                                ref quantiles,
                                sum,
                                count,
                            } => {
                                for q in quantiles {
                                    let mut tags = metric.tags().clone();
                                    tags.insert("quantile".to_string(), q.quantile.to_string());
                                    write_metric!(buf, metric.name(), tags, q.value)
                                }

                                write_metric!(
                                    buf,
                                    format!("{}_sum", metric.name()),
                                    metric.tags(),
                                    sum
                                );
                                write_metric!(
                                    buf,
                                    format!("{}_count", metric.name()),
                                    metric.tags(),
                                    count
                                );
                            }
                            MetricValue::Histogram {
                                ref buckets,
                                sum,
                                count,
                            } => {
                                for b in buckets {
                                    let mut tags = metric.tags().clone();
                                    if b.upper == f64::MAX {
                                        tags.insert("le".to_string(), "+Inf");
                                    } else {
                                        tags.insert("le".to_string(), b.upper.to_string());
                                    }

                                    write_metric!(buf, metric.name(), tags, b.count);
                                }

                                write_metric!(
                                    buf,
                                    format!("{}_sum", metric.name()),
                                    metric.tags(),
                                    sum
                                );
                                write_metric!(
                                    buf,
                                    format!("{}_count", metric.name()),
                                    metric.tags(),
                                    count
                                );
                            }
                        }
                    }
                });

            let mut builder = Response::builder()
                .header(http::header::CONTENT_TYPE, "text/plain; charset=utf-8")
                .status(StatusCode::OK);

            let body = if !should_compress(&req) {
                buf.freeze()
            } else {
                let mut encoder = flate2::write::GzEncoder::new(
                    BytesMut::new().writer(),
                    flate2::Compression::default(),
                );
                encoder.write_all(&buf).unwrap();

                builder = builder.header(http::header::CONTENT_ENCODING, "gzip");

                encoder.finish().unwrap().into_inner().freeze()
            };

            builder
                .body(Body::from(body))
                .expect("Response build failed") // error should never happened
        }
        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .expect("Response build failed"),
    }
}

fn should_compress(req: &Request<Body>) -> bool {
    match req.headers().get(http::header::ACCEPT_ENCODING) {
        Some(value) => {
            let value = value.to_str().unwrap_or("");

            value.contains("gzip")
        }
        None => false,
    }
}

#[async_trait]
impl StreamSink for PrometheusExporter {
    async fn run(mut self: Box<Self>, mut input: BoxStream<'_, Events>) -> Result<(), ()> {
        let (_trigger, tripwire) = Tripwire::new();
        let metrics = Arc::clone(&self.metrics);
        let mut ticker = tokio::time::interval(self.ttl);

        // GC routine
        let mut shutdown = tripwire.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    biased;

                    _ = &mut shutdown => break,
                    _ = ticker.tick() => {}
                }

                let mut cleaned = 0;
                let metrics = Arc::clone(&metrics);
                let mut state = metrics.lock();
                let now = Utc::now().timestamp();

                for (_name, set) in state.iter_mut() {
                    set.metrics.retain(|entry| {
                        let keep = entry.expired_at > now;
                        if !keep {
                            cleaned += 1;
                        }

                        keep
                    });
                }

                state.retain(|_name, set| !set.metrics.is_empty());

                debug!(message = "GC finished", cleaned);
            }
        });

        let metrics = Arc::clone(&self.metrics);
        let service = make_service_fn(move |_| {
            let metrics = Arc::clone(&metrics);

            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    let now = Utc::now().timestamp();
                    let metrics = Arc::clone(&metrics);
                    let resp = handle(req, metrics, now);

                    futures::future::ok::<_, Infallible>(resp)
                }))
            }
        });

        // HTTP server routine
        let address = self.endpoint;
        let tls = self.tls.clone();
        tokio::spawn(async move {
            let listener = MaybeTlsListener::bind(&address, &tls)
                .await
                .map_err(|err| error!(message = "Server TLS error", ?err))?;

            info!(message = "start http server", ?address);

            Server::builder(hyper::server::accept::from_stream(listener.accept_stream()))
                .serve(service)
                .with_graceful_shutdown(tripwire)
                .await
                .map_err(|err| error!(message = "Server error", ?err))?;

            Ok::<(), ()>(())
        });

        // Handle input metrics
        while let Some(events) = input.next().await {
            if let Events::Metrics(metrics) = events {
                let mut state = self.metrics.lock();
                let now = Utc::now();

                metrics.into_iter().for_each(|mut metric| {
                    let finalizers = metric.take_finalizers();

                    // Looks a little bit dummy but this should avoid some allocation for state's K.
                    let sets = match state.get_mut(metric.name()) {
                        Some(sets) => sets,
                        None => state.entry(metric.name().to_string()).or_insert(Sets {
                            description: metric.description.clone().unwrap_or_default(),
                            metrics: Default::default(),
                        }),
                    };

                    let timestamp = match metric.timestamp {
                        Some(ts) => ts,
                        None => now,
                    };

                    // `insert` will not update the entry, but `replace` will.
                    sets.metrics.replace(ExpiringEntry {
                        metric,
                        expired_at: timestamp.timestamp() + self.ttl.as_secs() as i64,
                    });
                    finalizers.update_status(EventStatus::Delivered)
                })
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }

    #[test]
    fn test_metrics_insert() {
        let mut set = BTreeSet::new();
        let m1 = Metric::gauge("foo", "", 0.1);
        let mut m2 = m1.clone();
        m2.value = MetricValue::Gauge(0.2);

        let now = Utc::now().timestamp();
        let ent = ExpiringEntry {
            metric: m1,
            expired_at: now + 60,
        };

        set.insert(ent);

        assert_eq!(set.len(), 1);

        let ent = ExpiringEntry {
            metric: m2,
            expired_at: now + 120,
        };

        set.insert(ent);

        assert_eq!(set.len(), 1);
        assert_eq!(set.first().unwrap().expired_at, now + 60);
    }
}
