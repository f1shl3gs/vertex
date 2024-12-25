use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::hash::{Hash, Hasher};
use std::io::Write as _;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::{BufMut, Bytes, BytesMut};
use chrono::Utc;
use configurable::configurable_component;
use event::tags::Tags;
use event::{Bucket, EventStatus, Events, Finalizable, Metric, MetricValue, Quantile};
use framework::config::{DataType, Resource, SinkConfig, SinkContext};
use framework::tls::{MaybeTlsListener, TlsConfig};
use framework::{Healthcheck, ShutdownSignal, Sink, StreamSink};
use futures::stream::BoxStream;
use futures::{FutureExt, StreamExt};
use http::HeaderMap;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use parking_lot::RwLock;

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
        let sink = PrometheusExporter::new(self.endpoint, self.tls.clone(), self.ttl);
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
    tags: Tags,
    value: MetricValue,

    // unix timestamp in milli seconds
    expired_at: i64,
}

impl Hash for ExpiringEntry {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.tags.hash(state)
    }
}

impl PartialEq<Self> for ExpiringEntry {
    fn eq(&self, other: &Self) -> bool {
        self.tags == other.tags
    }
}

impl Eq for ExpiringEntry {}

impl PartialOrd<Self> for ExpiringEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ExpiringEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        let a = &self.tags;
        let b = &other.tags;

        match a.len().cmp(&b.len()) {
            Ordering::Equal => {}
            ordering => return ordering,
        }

        for (ak, av) in a {
            match b.get(ak) {
                Some(bv) => {
                    let ordering = av.partial_cmp(bv);
                    if ordering != Some(Ordering::Equal) {
                        return ordering.unwrap_or(Ordering::Greater);
                    }
                }
                None => return Ordering::Greater,
            }
        }

        Ordering::Equal
    }
}

#[derive(Default)]
struct Sets {
    description: String,
    metrics: BTreeSet<ExpiringEntry>,
}

struct PrometheusExporter {
    endpoint: SocketAddr,
    tls: Option<TlsConfig>,
    ttl: Duration,
}

impl PrometheusExporter {
    fn new(endpoint: SocketAddr, tls: Option<TlsConfig>, ttl: Duration) -> Self {
        Self { endpoint, tls, ttl }
    }
}

#[inline]
fn write_tags(buf: &mut BytesMut, tags: &Tags) {
    if tags.is_empty() {
        buf.put_slice(b" ");
        return;
    }

    let mut first = true;
    for (key, value) in tags {
        if first {
            first = false;
            buf.put_u8(b'{');
        } else {
            buf.put_u8(b',');
        }

        buf.put_slice(key.as_bytes());
        buf.put("=\"".as_bytes());
        buf.put(value.to_string_lossy().as_ref().as_bytes());
        buf.put("\"".as_bytes());
    }

    buf.put_slice(b"} ");
}

#[inline]
fn write_simple_metric(buf: &mut BytesMut, name: &str, tags: &Tags, value: f64) {
    buf.put_slice(name.as_bytes());
    write_tags(buf, tags);
    buf.put_slice(value.to_string().as_bytes());
    buf.put_slice(b"\n");
}

fn write_summary_metric(
    buf: &mut BytesMut,
    name: &str,
    tags: &Tags,
    quantiles: &[Quantile],
    sum: f64,
    count: u64,
) {
    // write quantiles
    for quantile in quantiles {
        buf.put(name.as_bytes());

        buf.put_slice(b"{quantile=\"");
        buf.put(quantile.quantile.to_string().as_bytes());
        buf.put_slice(b"\"");
        // handle other tags
        for (key, value) in tags {
            buf.put_slice(b",");
            buf.put(key.as_bytes());
            buf.put_slice(b"=\"");
            buf.put(value.to_string_lossy().as_bytes());
            buf.put_slice(b"\"");
        }
        buf.put_slice(b"} ");

        buf.put(quantile.value.to_string().as_bytes());
        buf.put_slice(b"\n");
    }

    // write sum
    buf.put(name.as_bytes());
    buf.put_slice(b"_sum");
    write_tags(buf, tags);
    buf.put(sum.to_string().as_bytes());
    buf.put_slice(b"\n");

    // write count
    buf.put(name.as_bytes());
    buf.put_slice(b"_count");
    write_tags(buf, tags);
    buf.put(count.to_string().as_bytes());
    buf.put_slice(b"\n");
}

fn write_histogram_metric(
    buf: &mut BytesMut,
    name: &str,
    tags: &Tags,
    buckets: &[Bucket],
    sum: f64,
    count: u64,
) {
    // write buckets
    for bucket in buckets {
        buf.put(name.as_bytes());

        buf.put_slice(b"_bucket{le=\"");
        if bucket.upper == f64::MAX {
            buf.put_slice(b"+Inf\"");
        } else {
            buf.put(bucket.upper.to_string().as_bytes());
            buf.put_slice(b"\"");
        }

        // handle other tags
        for (key, value) in tags {
            buf.put_slice(b",");
            buf.put(key.as_bytes());
            buf.put_slice(b"=\"");
            buf.put(value.to_string_lossy().as_bytes());
            buf.put_slice(b"\"");
        }
        buf.put_slice(b"} ");

        buf.put(bucket.count.to_string().as_bytes());
        buf.put_slice(b"\n");
    }

    // write sum
    buf.put(name.as_bytes());
    buf.put_slice(b"_sum");
    write_tags(buf, tags);
    buf.put(sum.to_string().as_bytes());
    buf.put_slice(b"\n");

    // write count
    buf.put(name.as_bytes());
    buf.put_slice(b"_count");
    write_tags(buf, tags);
    buf.put(count.to_string().as_bytes());
    buf.put_slice(b"\n");
}

fn handle(
    req: Request<Incoming>,
    metrics: Arc<RwLock<BTreeMap<String, Sets>>>,
) -> Response<Full<Bytes>> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/metrics") => {
            let now = Utc::now().timestamp_millis();
            let mut buf = BytesMut::with_capacity(16 * 1024);

            metrics.read().iter().for_each(|(name, sets)| {
                let mut header = false;
                for entry in &sets.metrics {
                    let ExpiringEntry {
                        tags,
                        value,
                        expired_at,
                        ..
                    } = entry;

                    if *expired_at < now {
                        continue;
                    }

                    if !header {
                        header = true;

                        // write header like this
                        // # HELP node_cpu_scaling_governor Current enabled CPU frequency governor
                        // # TYPE node_cpu_scaling_governor gauge
                        let kind = match value {
                            MetricValue::Gauge(_) => "gauge",
                            MetricValue::Sum(_) => "counter",
                            MetricValue::Histogram { .. } => "histogram",
                            MetricValue::Summary { .. } => "summary",
                        };

                        writeln!(
                            &mut buf,
                            "# HELP {} {}\n# TYPE {} {}",
                            name, sets.description, name, kind
                        )
                        .unwrap();
                    }

                    match &value {
                        MetricValue::Sum(value) => {
                            write_simple_metric(&mut buf, name, tags, *value);
                        }
                        MetricValue::Gauge(value) => {
                            write_simple_metric(&mut buf, name, tags, *value);
                        }
                        MetricValue::Histogram {
                            count,
                            sum,
                            buckets,
                        } => {
                            write_histogram_metric(&mut buf, name, tags, buckets, *sum, *count);
                        }
                        MetricValue::Summary {
                            count,
                            sum,
                            quantiles,
                        } => {
                            write_summary_metric(&mut buf, name, tags, quantiles, *sum, *count);
                        }
                    }
                }
            });

            let mut builder = Response::builder()
                .header(http::header::CONTENT_TYPE, "text/plain; charset=utf-8")
                .status(StatusCode::OK);

            let body = if !should_compress(req.headers()) {
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
                .body(Full::new(body))
                .expect("Response build failed") // error should never have happened
        }
        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::default())
            .expect("Response build failed"),
    }
}

fn should_compress(headers: &HeaderMap) -> bool {
    match headers.get(http::header::ACCEPT_ENCODING) {
        Some(value) => match value.to_str() {
            Ok(value) => value.contains("gzip"),
            Err(_err) => false,
        },
        None => false,
    }
}

#[async_trait]
impl StreamSink for PrometheusExporter {
    async fn run(mut self: Box<Self>, mut input: BoxStream<'_, Events>) -> Result<(), ()> {
        // The state key is metric name, `Sets` is a container of tags, value and timestamp.
        // HashMap might have better performance, but we want the output is ordered so that's
        // why we choose BTreeMap.
        let states = Arc::new(RwLock::new(BTreeMap::<String, Sets>::new()));
        let (trigger_shutdown, shutdown, _shutdown_done) = ShutdownSignal::new_wired();

        // HTTP server routine
        let listener = MaybeTlsListener::bind(&self.endpoint, &self.tls)
            .await
            .map_err(|err| error!(message = "Server TLS error", %err))?;
        let http_states = Arc::clone(&states);
        let http_shutdown = shutdown.clone();
        let service = service_fn(move |req: Request<Incoming>| {
            let metrics = Arc::clone(&http_states);

            async move {
                let resp = handle(req, metrics);

                Ok::<_, hyper::Error>(resp)
            }
        });
        tokio::spawn(async move {
            let _ = framework::http::serve(listener, service)
                .with_graceful_shutdown(http_shutdown)
                .await;

            debug!(message = "http server shutdown successful");
        });

        // GC routine
        let mut ticker = tokio::time::interval(self.ttl);
        let mut gc_shutdown = shutdown.clone();
        let gc_states = Arc::clone(&states);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    biased;

                    _ = &mut gc_shutdown => break,
                    _ = ticker.tick() => {}
                }

                let mut cleaned = 0;
                let now = Utc::now().timestamp_millis();
                let mut state = gc_states.write();
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

            debug!(message = "gc routine shutdown success");
        });

        // Handle input metrics
        while let Some(events) = input.next().await {
            if let Events::Metrics(metrics) = events {
                let now = Utc::now();
                let mut state = states.write();

                metrics.into_iter().for_each(|mut metric| {
                    let finalizers = metric.take_finalizers();
                    let Metric {
                        series,
                        description,
                        timestamp,
                        value,
                        ..
                    } = metric;

                    // Looks a little bit dummy but this should avoid some allocation for state's K.
                    let sets = match state.get_mut(&series.name) {
                        Some(sets) => sets,
                        None => state.entry(series.name).or_insert(Sets {
                            description: description.unwrap_or_default(),
                            metrics: Default::default(),
                        }),
                    };

                    // `insert` does not update the entry, but `replace` does.
                    let timestamp = timestamp.unwrap_or(now).timestamp_millis();
                    sets.metrics.insert(ExpiringEntry {
                        tags: series.tags,
                        value,
                        expired_at: timestamp + self.ttl.as_millis() as i64,
                    });

                    finalizers.update_status(EventStatus::Delivered)
                })
            }
        }

        // shutdown background routines
        //
        // TODO: maybe we should wait for the background routines to exit
        trigger_shutdown.cancel();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event::tags;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }

    #[test]
    fn metrics_insert() {
        #[allow(clippy::mutable_key_type)]
        let mut set = BTreeSet::new();

        let now = Utc::now().timestamp_millis();
        let ent = ExpiringEntry {
            tags: tags!(),
            value: MetricValue::Gauge(0.1),
            expired_at: now + 60,
        };

        set.insert(ent);

        assert_eq!(set.len(), 1);

        let ent = ExpiringEntry {
            tags: tags!(),
            value: MetricValue::Gauge(0.2),
            expired_at: now + 120,
        };

        set.insert(ent);

        assert_eq!(set.len(), 1);
        assert_eq!(set.first().unwrap().expired_at, now + 60);
    }

    #[test]
    fn summary() {
        let quantiles = &[
            Quantile {
                quantile: 0.01,
                value: 3102.0,
            },
            Quantile {
                quantile: 0.05,
                value: 3272.0,
            },
            Quantile {
                quantile: 0.5,
                value: 4773.0,
            },
            Quantile {
                quantile: 0.9,
                value: 9001.0,
            },
            Quantile {
                quantile: 0.99,
                value: 76656.0,
            },
        ];
        let sum = 17560473.0;
        let count = 2693;

        // no tags
        let mut buf = BytesMut::new();
        write_summary_metric(
            &mut buf,
            "rpc_duration_seconds",
            &tags!(),
            quantiles,
            sum,
            count,
        );

        write_summary_metric(
            &mut buf,
            "rpc_duration_seconds",
            &tags!("foo" => "bar"),
            quantiles,
            sum,
            count,
        );

        assert_eq!(
            r#"rpc_duration_seconds{quantile="0.01"} 3102
rpc_duration_seconds{quantile="0.05"} 3272
rpc_duration_seconds{quantile="0.5"} 4773
rpc_duration_seconds{quantile="0.9"} 9001
rpc_duration_seconds{quantile="0.99"} 76656
rpc_duration_seconds_sum 17560473
rpc_duration_seconds_count 2693
rpc_duration_seconds{quantile="0.01",foo="bar"} 3102
rpc_duration_seconds{quantile="0.05",foo="bar"} 3272
rpc_duration_seconds{quantile="0.5",foo="bar"} 4773
rpc_duration_seconds{quantile="0.9",foo="bar"} 9001
rpc_duration_seconds{quantile="0.99",foo="bar"} 76656
rpc_duration_seconds_sum{foo="bar"} 17560473
rpc_duration_seconds_count{foo="bar"} 2693
"#,
            std::str::from_utf8(&buf).unwrap()
        );
    }

    #[test]
    fn histogram() {
        let sum = 53423.0;
        let count = 144320;
        let buckets = &[
            Bucket {
                upper: 0.05,
                count: 24054,
            },
            Bucket {
                upper: 0.1,
                count: 33444,
            },
            Bucket {
                upper: 0.2,
                count: 100392,
            },
            Bucket {
                upper: 0.5,
                count: 129389,
            },
            Bucket {
                upper: 1.0,
                count: 133988,
            },
            Bucket {
                upper: f64::MAX,
                count: 144320,
            },
        ];

        // no tags
        let mut buf = BytesMut::new();
        write_histogram_metric(
            &mut buf,
            "http_request_duration_seconds",
            &tags!(),
            buckets,
            sum,
            count,
        );

        // with tags
        write_histogram_metric(
            &mut buf,
            "http_request_duration_seconds",
            &tags!(
                "foo" => "bar",
            ),
            buckets,
            sum,
            count,
        );

        assert_eq!(
            r#"http_request_duration_seconds_bucket{le="0.05"} 24054
http_request_duration_seconds_bucket{le="0.1"} 33444
http_request_duration_seconds_bucket{le="0.2"} 100392
http_request_duration_seconds_bucket{le="0.5"} 129389
http_request_duration_seconds_bucket{le="1"} 133988
http_request_duration_seconds_bucket{le="+Inf"} 144320
http_request_duration_seconds_sum 53423
http_request_duration_seconds_count 144320
http_request_duration_seconds_bucket{le="0.05",foo="bar"} 24054
http_request_duration_seconds_bucket{le="0.1",foo="bar"} 33444
http_request_duration_seconds_bucket{le="0.2",foo="bar"} 100392
http_request_duration_seconds_bucket{le="0.5",foo="bar"} 129389
http_request_duration_seconds_bucket{le="1",foo="bar"} 133988
http_request_duration_seconds_bucket{le="+Inf",foo="bar"} 144320
http_request_duration_seconds_sum{foo="bar"} 53423
http_request_duration_seconds_count{foo="bar"} 144320
"#,
            std::str::from_utf8(&buf).unwrap()
        );
    }
}
