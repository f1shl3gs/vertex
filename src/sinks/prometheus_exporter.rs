use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use chrono::Utc;
use configurable::configurable_component;
use event::tags::{Tags, Value};
use event::{Bucket, EventStatus, Events, MetricValue, Quantile};
use flate2::write::GzEncoder;
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
        let const_labels = self
            .const_labels
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>();

        let sink = PrometheusExporter::new(
            self.listen,
            self.auth.as_ref().map(|auth| auth.authorizer()),
            self.tls.clone(),
            self.ttl,
            const_labels,
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
    const_labels: Arc<[(String, String)]>,
}

impl PrometheusExporter {
    fn new(
        listen: SocketAddr,
        auth: Option<Authorizer>,
        tls: Option<TlsConfig>,
        ttl: Duration,
        const_labels: Vec<(String, String)>,
    ) -> Self {
        Self {
            listen,
            auth,
            tls,
            ttl,
            const_labels: Arc::from(const_labels),
        }
    }
}

fn write_sample<T: Write>(
    buf: &mut T,
    name: &str,
    suffix: Option<&'static str>,
    tags: &Tags,
    value: f64,
    const_labels: &[(String, String)],
    additional: Option<(&'static str, f64)>,
) {
    buf.write_all(name.as_bytes()).unwrap();
    if let Some(suffix) = suffix {
        buf.write_all(suffix.as_bytes()).unwrap();
    }

    let mut tag_count = tags.len() + const_labels.len() + additional.is_some() as usize;
    if tag_count != 0 {
        buf.write_all(b"{").unwrap();

        for (key, value) in tags {
            buf.write_fmt(format_args!("{}=\"", key.as_str())).unwrap();
            match value {
                Value::String(s) => {
                    buf.write_all(s.as_bytes()).unwrap();
                }
                Value::I64(i) => {
                    buf.write_all(i.to_string().as_bytes()).unwrap();
                }
                Value::F64(f) => {
                    buf.write_all(f.to_string().as_bytes()).unwrap();
                }
                Value::Bool(b) => {
                    let s = if *b { "true" } else { "false" };
                    buf.write_all(s.as_bytes()).unwrap();
                }
                Value::Array(arr) => {
                    let value = serde_json::to_string(arr).unwrap();
                    buf.write_all(value.as_bytes()).unwrap();
                }
            }

            tag_count -= 1;
            if tag_count != 0 {
                buf.write_all(b"\",").unwrap();
            } else {
                buf.write_all(b"\"").unwrap();
            }
        }

        for (key, value) in const_labels {
            buf.write_fmt(format_args!("{}=\"{}\"", key, value))
                .unwrap();

            tag_count -= 1;
            if tag_count != 0 {
                buf.write_all(b",").unwrap();
            }
        }

        if let Some((key, value)) = additional {
            if value.is_nan() {
                buf.write_fmt(format_args!("{}=\"NaN\"", key)).unwrap();
            } else if value == f64::NEG_INFINITY {
                buf.write_fmt(format_args!("{}=\"-Inf\"", key)).unwrap();
            } else if value == f64::INFINITY {
                buf.write_fmt(format_args!("{}=\"+Inf\"", key)).unwrap();
            } else {
                buf.write_fmt(format_args!("{}=\"{}\"", key, value))
                    .unwrap();
            }

            tag_count -= 1;
            if tag_count != 0 {
                buf.write_all(b",").unwrap();
            }
        }

        buf.write_all(b"}").unwrap();
    }

    buf.write_fmt(format_args!(" {}\n", value)).unwrap();
}

fn write_histogram<T: Write>(
    buf: &mut T,
    name: &str,
    tags: &Tags,
    buckets: &[Bucket],
    sum: f64,
    count: u64,
    const_labels: &[(String, String)],
) {
    for bucket in buckets {
        write_sample(
            buf,
            name,
            Some("_bucket"),
            tags,
            bucket.count as f64,
            const_labels,
            Some(("le", bucket.upper)),
        )
    }

    write_sample(buf, name, Some("_sum"), tags, sum, const_labels, None);
    write_sample(
        buf,
        name,
        Some("_count"),
        tags,
        count as f64,
        const_labels,
        None,
    );
}

fn write_summary<T: Write>(
    buf: &mut T,
    name: &str,
    tags: &Tags,
    quantiles: &[Quantile],
    sum: f64,
    count: u64,
    const_labels: &[(String, String)],
) {
    for quantile in quantiles {
        write_sample(
            buf,
            name,
            None,
            tags,
            quantile.value,
            const_labels,
            Some(("quantile", quantile.quantile)),
        );
    }

    write_sample(buf, name, Some("_sum"), tags, sum, const_labels, None);
    write_sample(
        buf,
        name,
        Some("_count"),
        tags,
        count as f64,
        const_labels,
        None,
    );
}

fn handle(
    req: Request<Incoming>,
    metrics: Arc<RwLock<BTreeMap<Cow<'static, str>, Sets>>>,
    const_labels: Arc<[(String, String)]>,
) -> http::Result<Response<Full<Bytes>>> {
    if req.method() != Method::GET || req.uri().path() != "/metrics" {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::default());
    }

    let mut buf = match req.headers().get(ACCEPT_ENCODING) {
        None => RespWriter::identity(),
        Some(value) => {
            if value.as_bytes().windows(4).any(|s| s == b"gzip") {
                RespWriter::gzip()
            } else {
                RespWriter::identity()
            }
        }
    };

    let now = Utc::now().timestamp_millis();
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
                MetricValue::Sum(value) | MetricValue::Gauge(value) => {
                    write_sample(&mut buf, name, None, tags, *value, &const_labels, None);
                }
                MetricValue::Histogram {
                    count,
                    sum,
                    buckets,
                } => {
                    write_histogram(&mut buf, name, tags, buckets, *sum, *count, &const_labels);
                }
                MetricValue::Summary {
                    count,
                    sum,
                    quantiles,
                } => {
                    write_summary(&mut buf, name, tags, quantiles, *sum, *count, &const_labels);
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

    builder.body(Full::new(buf.into_bytes()))
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
        let const_labels = Arc::clone(&self.const_labels);

        let service = service_fn(move |req: Request<Incoming>| {
            let auth = Arc::clone(&auth);
            let metrics = Arc::clone(&http_states);
            let const_labels = Arc::clone(&const_labels);

            async move {
                if let Some(auth) = auth.as_ref()
                    && !auth.authorized(&req)
                {
                    return Response::builder()
                        .status(StatusCode::UNAUTHORIZED)
                        .body(Full::new(Bytes::new()));
                }

                handle(req, metrics, const_labels)
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
    Identity(BytesMut),
    Gzip {
        // A buffer to temporarily store the data write to GzEncoder,
        //
        // The write_sample, write_histogram and write_summary writes a lot
        // small chunk (most of them less than 20 bytes), which will not
        // take advantage of the SIMD optimization implemented in GzEncoder.
        // Without this buffer, vertex almost double the CPU usage, when serve
        // node_exporter's metrics.
        buf: Vec<u8>,
        encoder: GzEncoder<Vec<u8>>,
    },
}

impl RespWriter {
    const INITIAL_BUFFER_CAPACITY: usize = 4 * 1024;

    fn identity() -> Self {
        Self::Identity(BytesMut::with_capacity(Self::INITIAL_BUFFER_CAPACITY))
    }

    fn gzip() -> Self {
        Self::Gzip {
            buf: Vec::with_capacity(Self::INITIAL_BUFFER_CAPACITY),
            encoder: GzEncoder::new(Vec::new(), flate2::Compression::default()),
        }
    }

    fn content_encoding(&self) -> Option<&'static str> {
        match &self {
            Self::Identity(_) => None,
            Self::Gzip { .. } => Some("gzip"),
        }
    }

    fn into_bytes(self) -> Bytes {
        match self {
            Self::Identity(buf) => buf.freeze(),
            Self::Gzip { buf, mut encoder } => {
                if !buf.is_empty() {
                    encoder
                        .write_all(&buf)
                        .expect("write last buffered data to GzEncoder");
                    encoder.flush().expect("flush GzEncoder");
                    // no need to clear `buf`, it is dropped here
                }

                encoder
                    .finish()
                    .expect("error encoding gzip content")
                    .into()
            }
        }
    }
}

impl Write for RespWriter {
    fn write(&mut self, src: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Identity(writer) => {
                writer.extend_from_slice(src);
            }
            Self::Gzip { buf, encoder } => {
                if src.len() > buf.capacity() - buf.len() && !buf.is_empty() {
                    // flush the current buffered data
                    encoder.write_all(buf)?;
                    buf.clear();
                }

                buf.reserve(src.len());
                unsafe {
                    // SAFETY: safe by above reserve.
                    std::ptr::copy_nonoverlapping(
                        src.as_ptr(),
                        buf.as_mut_ptr().add(buf.len()),
                        src.len(),
                    );
                    buf.set_len(buf.len() + src.len());
                }
            }
        }

        Ok(src.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Gzip { buf, encoder } => {
                encoder.write_all(buf)?;
                encoder.flush()?;
                buf.clear();
            }
            Self::Identity(_buf) => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use event::tags;

    use super::*;

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
        let mut buf = RespWriter::identity();
        write_summary(
            &mut buf,
            "rpc_duration_seconds",
            &tags!(),
            quantiles,
            sum,
            count,
            &[],
        );

        write_summary(
            &mut buf,
            "rpc_duration_seconds",
            &tags!("foo" => "bar"),
            quantiles,
            sum,
            count,
            &[],
        );

        assert_eq!(
            r#"rpc_duration_seconds{quantile="0.01"} 3102
rpc_duration_seconds{quantile="0.05"} 3272
rpc_duration_seconds{quantile="0.5"} 4773
rpc_duration_seconds{quantile="0.9"} 9001
rpc_duration_seconds{quantile="0.99"} 76656
rpc_duration_seconds_sum 17560473
rpc_duration_seconds_count 2693
rpc_duration_seconds{foo="bar",quantile="0.01"} 3102
rpc_duration_seconds{foo="bar",quantile="0.05"} 3272
rpc_duration_seconds{foo="bar",quantile="0.5"} 4773
rpc_duration_seconds{foo="bar",quantile="0.9"} 9001
rpc_duration_seconds{foo="bar",quantile="0.99"} 76656
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
                upper: f64::INFINITY,
                count: 144320,
            },
        ];

        // no tags
        let mut buf = RespWriter::identity();
        write_histogram(
            &mut buf,
            "http_request_duration_seconds",
            &tags!(),
            buckets,
            sum,
            count,
            &[],
        );

        // with tags
        write_histogram(
            &mut buf,
            "http_request_duration_seconds",
            &tags!(
                "foo" => "bar",
            ),
            buckets,
            sum,
            count,
            &[],
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
http_request_duration_seconds_bucket{foo="bar",le="0.05"} 24054
http_request_duration_seconds_bucket{foo="bar",le="0.1"} 33444
http_request_duration_seconds_bucket{foo="bar",le="0.2"} 100392
http_request_duration_seconds_bucket{foo="bar",le="0.5"} 129389
http_request_duration_seconds_bucket{foo="bar",le="1"} 133988
http_request_duration_seconds_bucket{foo="bar",le="+Inf"} 144320
http_request_duration_seconds_sum{foo="bar"} 53423
http_request_duration_seconds_count{foo="bar"} 144320
"#,
            std::str::from_utf8(buf.into_bytes().as_ref()).unwrap()
        );
    }

    #[test]
    fn identity() {
        let input = "dummy test content";
        let want = input;

        let mut writer = RespWriter::identity();
        writer.write_all(b"dummy ").unwrap();
        writer.write_all(b"test ").unwrap();
        writer.write_all(b"content").unwrap();
        let got = writer.into_bytes();

        assert_eq!(want, got);
    }

    #[test]
    fn gzip() {
        let input = "dummy test content";

        let mut writer = RespWriter::Gzip {
            // 4 will trigger flush when write_all called
            buf: Vec::with_capacity(4),
            encoder: GzEncoder::new(Vec::new(), flate2::Compression::default()),
        };
        writer.write_all(b"dummy ").unwrap();
        writer.write_all(b"test ").unwrap();
        writer.write_all(b"content").unwrap();

        let got = writer.into_bytes();

        // assert_eq!(want, got.as_ref());
        let mut decoder = flate2::read::GzDecoder::new(&got[..]);
        let mut got = String::new();
        decoder.read_to_string(&mut got).unwrap();

        assert_eq!(input.as_bytes(), got.as_bytes());
    }
}
