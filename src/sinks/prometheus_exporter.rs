use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use bytes::{BufMut, Bytes, BytesMut};
use chrono::Utc;
use configurable::configurable_component;
use event::tags::Tags;
use event::{Bucket, EventStatus, Events, MetricValue, Quantile};
use framework::config::{InputType, Resource, SinkConfig, SinkContext};
use framework::http::{Auth, Authorizer};
use framework::tls::{MaybeTlsListener, TlsConfig};
use framework::{Healthcheck, ShutdownSignal, Sink, StreamSink};
use futures::stream::BoxStream;
use futures::{FutureExt, StreamExt};
use http::header::{ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_TYPE};
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use parking_lot::RwLock;

#[configurable_component(sink, name = "prometheus_exporter")]
#[serde(deny_unknown_fields)]
struct Config {
    /// The address the prometheus server will listen at
    #[serde(default = "default_listen")]
    #[configurable(format = "ip-address", example = "0.0.0.0:9100")]
    listen: SocketAddr,

    auth: Option<Auth>,

    tls: Option<TlsConfig>,

    /// TTL for metrics, any metrics not received for ttl will be removed
    /// from cache.
    #[serde(default = "default_ttl")]
    #[serde(with = "humanize::duration::serde")]
    ttl: Duration,

    /// This allows you to add custom labels to all metrics exposed through
    /// this prometheus exporter
    ///
    /// `const_labels` honors the original metric tags
    #[serde(default)]
    const_labels: BTreeMap<String, String>,
}

fn default_listen() -> SocketAddr {
    SocketAddr::from(([0, 0, 0, 0], 9100))
}

const fn default_ttl() -> Duration {
    Duration::from_secs(5 * 60)
}

#[async_trait::async_trait]
#[typetag::serde(name = "prometheus_exporter")]
impl SinkConfig for Config {
    async fn build(&self, _cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        let sink = PrometheusExporter::new(
            self.listen,
            self.auth.as_ref().map(|auth| auth.authorizer()),
            self.tls.clone(),
            self.ttl,
            self.const_labels.clone(),
        );
        let health_check = futures::future::ok(()).boxed();

        Ok((Sink::Stream(Box::new(sink)), health_check))
    }

    fn input_type(&self) -> InputType {
        InputType::metric()
    }

    fn resources(&self) -> Vec<Resource> {
        vec![Resource::tcp(self.listen)]
    }
}

struct ExpiringEntry {
    tags: Tags,
    value: MetricValue,

    // unix timestamp in milli seconds
    expired_at: i64,
}

impl PartialEq<Self> for ExpiringEntry {
    fn eq(&self, other: &Self) -> bool {
        self.tags.eq(&other.tags)
    }
}

impl Eq for ExpiringEntry {}

impl PartialOrd<Self> for ExpiringEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ExpiringEntry {
    // the tags order might not same, so we have to compare one by one
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
    description: Cow<'static, str>,
    metrics: BTreeSet<ExpiringEntry>,
}

struct PrometheusExporter {
    listen: SocketAddr,
    auth: Option<Authorizer>,
    tls: Option<TlsConfig>,
    ttl: Duration,
    const_labels: BTreeMap<String, String>,
}

impl PrometheusExporter {
    fn new(
        listen: SocketAddr,
        auth: Option<Authorizer>,
        tls: Option<TlsConfig>,
        ttl: Duration,
        const_labels: BTreeMap<String, String>,
    ) -> Self {
        Self {
            listen,
            auth,
            tls,
            ttl,
            const_labels,
        }
    }
}

#[inline]
fn write_tags<T: Write>(buf: &mut T, tags: &Tags, const_labels: &BTreeMap<String, String>) {
    if tags.is_empty() && const_labels.is_empty() {
        let _ = buf.write_all(b" ");
        return;
    }

    let mut first = true;
    for (key, value) in tags {
        if first {
            first = false;
            buf.write_all(b"{").unwrap();
        } else {
            buf.write_all(b",").unwrap();
        }

        buf.write_all(key.as_bytes()).unwrap();
        buf.write_all(b"=\"").unwrap();
        buf.write_all(value.to_string_lossy().as_bytes()).unwrap();
        buf.write_all(b"\"").unwrap();
    }

    for (key, value) in const_labels {
        if tags.contains(key) {
            continue;
        }

        if first {
            first = false;
            buf.write_all(b"{").unwrap();
        } else {
            buf.write_all(b",").unwrap();
        }

        buf.write_all(key.as_bytes()).unwrap();
        buf.write_all(b"=\"").unwrap();
        buf.write_all(value.as_bytes()).unwrap();
        buf.write_all(b"\"").unwrap();
    }

    buf.write_all(b"} ").unwrap();
}

#[inline]
fn write_simple_metric<T: Write>(
    buf: &mut T,
    name: &str,
    tags: &Tags,
    value: f64,
    const_labels: &BTreeMap<String, String>,
) {
    buf.write_all(name.as_bytes()).unwrap();
    write_tags(buf, tags, const_labels);
    buf.write_all(value.to_string().as_bytes()).unwrap();
    buf.write_all(b"\n").unwrap();
}

fn write_summary_metric<T: Write>(
    buf: &mut T,
    name: &str,
    tags: &Tags,
    quantiles: &[Quantile],
    sum: f64,
    count: u64,
    const_labels: &BTreeMap<String, String>,
) {
    // write quantiles
    for quantile in quantiles {
        buf.write_all(name.as_bytes()).unwrap();

        buf.write_all(b"{quantile=\"").unwrap();
        buf.write_all(quantile.quantile.to_string().as_bytes())
            .unwrap();
        buf.write_all(b"\"").unwrap();
        // handle other tags
        for (key, value) in tags {
            buf.write_all(b",").unwrap();
            buf.write_all(key.as_bytes()).unwrap();
            buf.write_all(b"=\"").unwrap();
            buf.write_all(value.to_string_lossy().as_bytes()).unwrap();
            buf.write_all(b"\"").unwrap();
        }
        buf.write_all(b"} ").unwrap();

        buf.write_all(quantile.value.to_string().as_bytes())
            .unwrap();
        buf.write_all(b"\n").unwrap();
    }

    // write sum
    buf.write_all(name.as_bytes()).unwrap();
    buf.write_all(b"_sum").unwrap();
    write_tags(buf, tags, const_labels);
    buf.write_all(sum.to_string().as_bytes()).unwrap();
    buf.write_all(b"\n").unwrap();

    // write count
    buf.write_all(name.as_bytes()).unwrap();
    buf.write_all(b"_count").unwrap();
    write_tags(buf, tags, const_labels);
    buf.write_all(count.to_string().as_bytes()).unwrap();
    buf.write_all(b"\n").unwrap();
}

fn write_histogram_metric<T: Write>(
    buf: &mut T,
    name: &str,
    tags: &Tags,
    buckets: &[Bucket],
    sum: f64,
    count: u64,
    const_labels: &BTreeMap<String, String>,
) {
    // write buckets
    for bucket in buckets {
        buf.write_all(name.as_bytes()).unwrap();

        buf.write_all(b"_bucket{le=\"").unwrap();
        if bucket.upper == f64::MAX {
            buf.write_all(b"+Inf\"").unwrap();
        } else {
            buf.write_all(bucket.upper.to_string().as_bytes()).unwrap();
            buf.write_all(b"\"").unwrap();
        }

        // handle other tags
        for (key, value) in tags {
            buf.write_all(b",").unwrap();
            buf.write_all(key.as_bytes()).unwrap();
            buf.write_all(b"=\"").unwrap();
            buf.write_all(value.to_string_lossy().as_bytes()).unwrap();
            buf.write_all(b"\"").unwrap();
        }
        buf.write_all(b"} ").unwrap();

        buf.write_all(bucket.count.to_string().as_bytes()).unwrap();
        buf.write_all(b"\n").unwrap();
    }

    // write sum
    buf.write_all(name.as_bytes()).unwrap();
    buf.write_all(b"_sum").unwrap();
    write_tags(buf, tags, const_labels);
    buf.write_all(sum.to_string().as_bytes()).unwrap();
    buf.write_all(b"\n").unwrap();

    // write count
    buf.write_all(name.as_bytes()).unwrap();
    buf.write_all(b"_count").unwrap();
    write_tags(buf, tags, const_labels);
    buf.write_all(count.to_string().as_bytes()).unwrap();
    buf.write_all(b"\n").unwrap();
}

fn handle(
    req: Request<Incoming>,
    metrics: Arc<RwLock<BTreeMap<Cow<'static, str>, Sets>>>,
    const_labels: BTreeMap<String, String>,
) -> Response<Full<Bytes>> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/metrics") => {
            let now = Utc::now().timestamp_millis();

            let mut buf = match req.headers().get(ACCEPT_ENCODING) {
                None => RespWriter::plain(),
                Some(value) => {
                    if value.as_bytes().windows(4).any(|s| s == b"gzip") {
                        RespWriter::gzip()
                    } else {
                        RespWriter::plain()
                    }
                }
            };

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
                            write_simple_metric(&mut buf, name, tags, *value, &const_labels);
                        }
                        MetricValue::Gauge(value) => {
                            write_simple_metric(&mut buf, name, tags, *value, &const_labels);
                        }
                        MetricValue::Histogram {
                            count,
                            sum,
                            buckets,
                        } => {
                            write_histogram_metric(
                                &mut buf,
                                name,
                                tags,
                                buckets,
                                *sum,
                                *count,
                                &const_labels,
                            );
                        }
                        MetricValue::Summary {
                            count,
                            sum,
                            quantiles,
                        } => {
                            write_summary_metric(
                                &mut buf,
                                name,
                                tags,
                                quantiles,
                                *sum,
                                *count,
                                &const_labels,
                            );
                        }
                    }
                }
            });

            let mut builder = Response::builder()
                .header(CONTENT_TYPE, "text/plain; charset=utf-8")
                .status(StatusCode::OK);

            if let Some(encoding) = buf.content_encoding() {
                builder = builder.header(CONTENT_ENCODING, encoding);
            }

            builder
                .body(Full::new(buf.into_bytes()))
                .expect("Response build failed") // error should never have happened
        }
        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::default())
            .expect("Response build failed"),
    }
}

#[async_trait::async_trait]
impl StreamSink for PrometheusExporter {
    async fn run(mut self: Box<Self>, mut input: BoxStream<'_, Events>) -> Result<(), ()> {
        // The state key is metric name, `Sets` is a container of tags, value and timestamp.
        // HashMap might have better performance, but we want the output is ordered so that's
        // why we choose BTreeMap.
        let states = Arc::new(RwLock::new(BTreeMap::<Cow<'static, str>, Sets>::new()));
        let (trigger_shutdown, shutdown, _shutdown_done) = ShutdownSignal::new_wired();

        // HTTP server routine
        let listener = MaybeTlsListener::bind(&self.listen, self.tls.as_ref())
            .await
            .map_err(|err| error!(message = "Server TLS error", %err))?;
        let auth = Arc::new(self.auth.clone());
        let http_states = Arc::clone(&states);
        let http_shutdown = shutdown.clone();
        let const_labels = self.const_labels.clone();

        let service = service_fn(move |req: Request<Incoming>| {
            let auth = Arc::clone(&auth);
            let metrics = Arc::clone(&http_states);
            let const_labels = const_labels.clone();

            async move {
                if let Some(auth) = auth.as_ref()
                    && !auth.authorized(&req)
                {
                    let resp = Response::builder()
                        .status(StatusCode::UNAUTHORIZED)
                        .body(Full::new(Bytes::new()))
                        .unwrap();

                    return Ok(resp);
                }

                let resp = handle(req, metrics, const_labels);

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
                let ttl = self.ttl.as_millis() as i64;
                let mut state = states.write();

                metrics.into_iter().for_each(|metric| {
                    let (name, tags, description, value, timestamp, mut metadata) =
                        metric.into_parts();
                    let finalizers = metadata.take_finalizers();
                    let timestamp = timestamp.unwrap_or(now).timestamp_millis();

                    let sets = state.entry(name).or_insert(Sets {
                        description: description.unwrap_or_default(),
                        metrics: Default::default(),
                    });

                    sets.metrics.replace(ExpiringEntry {
                        tags,
                        value,
                        expired_at: timestamp + ttl,
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

enum RespWriter {
    Plain(BytesMut),
    Gzip(flate2::write::GzEncoder<bytes::buf::Writer<BytesMut>>),
}

impl Write for RespWriter {
    #[allow(clippy::disallowed_methods)]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            RespWriter::Plain(w) => w.writer().write(buf),
            RespWriter::Gzip(w) => w.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            RespWriter::Plain(w) => w.writer().flush(),
            RespWriter::Gzip(w) => w.flush(),
        }
    }
}

const DEFAULT_CAPACITY: usize = 16 * 1024;

impl RespWriter {
    fn plain() -> Self {
        Self::Plain(BytesMut::with_capacity(DEFAULT_CAPACITY))
    }

    fn gzip() -> Self {
        Self::Gzip(flate2::write::GzEncoder::new(
            BytesMut::with_capacity(DEFAULT_CAPACITY).writer(),
            flate2::Compression::default(),
        ))
    }

    fn into_bytes(self) -> Bytes {
        match self {
            RespWriter::Plain(w) => w.freeze(),
            RespWriter::Gzip(w) => w
                .finish()
                .expect("should be flushable")
                .into_inner()
                .freeze(),
        }
    }

    fn content_encoding(&self) -> Option<&'static str> {
        match self {
            RespWriter::Plain(_) => None,
            RespWriter::Gzip(_) => Some("gzip"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event::tags;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
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
        let mut buf = RespWriter::plain();
        write_summary_metric(
            &mut buf,
            "rpc_duration_seconds",
            &tags!(),
            quantiles,
            sum,
            count,
            &BTreeMap::new(),
        );

        write_summary_metric(
            &mut buf,
            "rpc_duration_seconds",
            &tags!("foo" => "bar"),
            quantiles,
            sum,
            count,
            &BTreeMap::new(),
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
            std::str::from_utf8(buf.into_bytes().as_ref()).unwrap()
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
        let mut buf = RespWriter::plain();
        write_histogram_metric(
            &mut buf,
            "http_request_duration_seconds",
            &tags!(),
            buckets,
            sum,
            count,
            &BTreeMap::new(),
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
            &BTreeMap::new(),
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
            std::str::from_utf8(buf.into_bytes().as_ref()).unwrap()
        );
    }
}
