use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::ops::Sub;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, SystemTime};

use bytes::Bytes;
use chrono::{DateTime, Utc};
use configurable::{Configurable, configurable_component};
use event::{Metric, tags};
use framework::config::{Output, ProxyConfig, SourceConfig, SourceContext, default_interval};
use framework::dns::{DnsError, Resolver};
use framework::http::Auth;
use framework::tls::TlsConfig;
use framework::{Error, Pipeline, ShutdownSignal, Source};
use http::header::{CONTENT_LENGTH, LAST_MODIFIED};
use http::response::Parts;
use http::uri::Scheme;
use http::{HeaderName, HeaderValue, Method, Request, Uri, Version};
use http_body_util::{BodyExt, Full};
use httpdate::HttpDate;
use hyper_util::client::legacy::Client;
use hyper_util::rt::{TokioExecutor, TokioIo};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::task::JoinSet;
use url::Url;

use crate::common::offset::offset;

const fn default_timeout() -> Duration {
    Duration::from_secs(5)
}

#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
struct Target {
    /// The URL of the endpoint to be monitored
    #[configurable(required, format = "uri", example = "http://localhost:8080")]
    url: Url,

    /// TLS configuration
    tls: Option<TlsConfig>,

    auth: Option<Auth>,

    /// Extra HTTP headers
    #[serde(default)]
    headers: HashMap<String, String>,

    /// Timeout for HTTP request, it's value should be less than `interval`.
    #[serde(with = "humanize::duration::serde", default = "default_timeout")]
    timeout: Duration,
}

impl Hash for Target {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.url.hash(state);
    }
}

/// The HTTP Check source can be used for synthethic checks against HTTP
/// endpoints. This source will make a request to the specified `endpoint`
/// using the configured `method`. This scraper generates a metric with
/// a label for each HTTP response status class with a value of `1` if
/// the status code matches the class.
#[configurable_component(source, name = "http_check")]
struct Config {
    /// Targets to probe
    #[configurable(required)]
    targets: Vec<Target>,

    /// This sources collects metrics on an interval.
    #[serde(with = "humanize::duration::serde", default = "default_interval")]
    interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "http_check")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        let interval = self.interval;
        let targets = self.targets.clone();

        Ok(Box::pin(async move {
            let mut tasks = JoinSet::new();

            for target in targets {
                tasks.spawn(run(
                    target,
                    interval,
                    cx.output.clone(),
                    cx.shutdown.clone(),
                    cx.proxy.clone(),
                ));
            }

            while tasks.join_next().await.is_some() {}

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

async fn run(
    target: Target,
    interval: Duration,
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
    _proxy: ProxyConfig,
) {
    let now = Utc::now()
        .timestamp_nanos_opt()
        .expect("timestamp can not be represented in a timestamp with nanosecond precision.");
    let mut ticker = tokio::time::interval_at(
        tokio::time::Instant::now() + offset(&target, interval, 0, now),
        interval,
    );

    let instance = target.url.host_str().unwrap_or("");

    loop {
        tokio::select! {
            _ = ticker.tick() => {},
            _ = &mut shutdown => {
                break;
            }
        }

        let result = probe(&target).await;
        let mut metrics = Vec::with_capacity(10);

        match result {
            Ok((parts, trace)) => {
                metrics.extend([
                    Metric::gauge_with_tags(
                        "http_up",
                        "Whether the target is success",
                        1,
                        tags!(
                            "instance" => instance,
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "http_duration_seconds",
                        "Duration of http request by phase",
                        trace.resolved.sub(trace.start).to_std().unwrap(),
                        tags!(
                            "phase" => "resolve",
                            "instance" => instance,
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "http_duration_seconds",
                        "Duration of http request by phase",
                        trace.connected.sub(trace.resolved).to_std().unwrap(),
                        tags!(
                            "phase" => "connect",
                            "instance" => instance,
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "http_duration_seconds",
                        "Duration of http request by phase",
                        trace.processed.sub(trace.connected).to_std().unwrap(),
                        tags!(
                            "phase" => "processing",
                            "instance" => instance,
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "http_duration_seconds",
                        "Duration of http request by phase",
                        trace.transferred.sub(trace.processed).to_std().unwrap(),
                        tags!(
                            "phase" => "transfer",
                            "instance" => instance,
                        ),
                    ),
                ]);

                let version = match parts.version {
                    Version::HTTP_09 => 0.9,
                    Version::HTTP_10 => 1.0,
                    Version::HTTP_11 => 1.1,
                    Version::HTTP_2 => 2.0,
                    Version::HTTP_3 => 3.0,
                    _ => 0.0,
                };
                metrics.push(Metric::gauge_with_tags(
                    "http_version",
                    "Returns the version of HTTP of the probe response",
                    version,
                    tags!(
                        "instance" => instance,
                    ),
                ));

                if let Some(value) = parts.headers.get(CONTENT_LENGTH) {
                    if let Ok(s) = value.to_str() {
                        if let Ok(content_length) = s.parse::<u64>() {
                            metrics.push(Metric::gauge_with_tags(
                                "http_content_length",
                                "Length of http content response",
                                content_length,
                                tags!(
                                    "instance" => instance,
                                ),
                            ));
                        }
                    }
                }

                if let Some(value) = parts.headers.get(LAST_MODIFIED) {
                    if let Ok(value) = value.to_str() {
                        if let Ok(date) = HttpDate::from_str(value) {
                            if let Ok(ts) =
                                SystemTime::from(date).duration_since(SystemTime::UNIX_EPOCH)
                            {
                                metrics.push(Metric::gauge_with_tags(
                                    "http_last_modified_timestamp_seconds",
                                    "Returns the Last-Modified HTTP response header in unixtime",
                                    ts.as_secs_f64(),
                                    tags!(
                                        "instance" => instance,
                                    ),
                                ));
                            }
                        }
                    }
                }

                metrics.extend([
                    // Metric::gauge_with_tags(
                    //     "http_uncompressed_body_length",
                    //     "Length of uncompressed response body",
                    //     trace.resp_bytes,
                    //     tags!(
                    //         "instance" => instance,
                    //     ),
                    // ),
                    Metric::gauge_with_tags(
                        "http_status_code",
                        "Response HTTP status code",
                        parts.status.as_u16(),
                        tags!(
                            "instance" => instance,
                        ),
                    ),
                ]);
            }
            Err(err) => {
                debug!(message = "http check failed", ?err, url = ?target.url);

                metrics.push(Metric::gauge_with_tags(
                    "http_up",
                    "Whether the target is success",
                    0,
                    tags!(
                        "instance" => instance,
                    ),
                ));
            }
        }

        if let Err(err) = output.send(metrics).await {
            warn!(message = "send metrics failed", %err);
            break;
        }
    }
}

#[derive(Debug, Default)]
struct Trace {
    start: DateTime<Utc>,
    resolved: DateTime<Utc>,
    connected: DateTime<Utc>,
    processed: DateTime<Utc>,
    transferred: DateTime<Utc>,
}

#[derive(thiserror::Error, Debug)]
enum ConnectError {
    #[error("missing scheme")]
    MissingScheme,
    #[error("no host found in uri")]
    NoHost,
    #[error("no available host")]
    NoAvailable,
    #[error(transparent)]
    Resolve(DnsError),
}

#[derive(Clone)]
struct TraceConnector {
    resolver: Resolver,

    resolved: Arc<Mutex<DateTime<Utc>>>,
    connected: Arc<Mutex<DateTime<Utc>>>,
}

impl tower::Service<Uri> for TraceConnector {
    type Response = TokioIo<TcpStream>;
    type Error = ConnectError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, dst: Uri) -> Self::Future {
        let resolver = self.resolver.clone();
        let connected = Arc::clone(&self.connected);
        let resolved = Arc::clone(&self.resolved);

        Box::pin(async move {
            let (host, port) = get_host_port(&dst)?;
            let host = host.trim_start_matches('[').trim_end_matches(']');

            let addrs = resolver
                .lookup_ip(host)
                .await
                .map_err(ConnectError::Resolve)?;

            *resolved.lock().unwrap() = Utc::now();

            for mut addr in addrs {
                addr.set_port(port);

                match TcpStream::connect(addr).await {
                    Ok(stream) => {
                        *connected.lock().unwrap() = Utc::now();
                        return Ok(TokioIo::new(stream));
                    }
                    Err(_err) => continue,
                }
            }

            *connected.lock().unwrap() = Utc::now();

            Err(ConnectError::NoAvailable)
        })
    }
}

fn get_host_port(dst: &Uri) -> Result<(&str, u16), ConnectError> {
    if dst.scheme().is_none() {
        return Err(ConnectError::MissingScheme);
    }

    let host = match dst.host() {
        Some(s) => s,
        None => return Err(ConnectError::NoHost),
    };

    let port = match dst.port() {
        Some(port) => port.as_u16(),
        None => {
            if dst.scheme() == Some(&Scheme::HTTPS) {
                443
            } else {
                80
            }
        }
    };

    Ok((host, port))
}

async fn probe(target: &Target) -> Result<(Parts, Trace), Error> {
    let result = tokio::time::timeout(target.timeout, async move {
        let connected = Arc::new(Mutex::new(Default::default()));
        let resolved = Arc::new(Mutex::new(Default::default()));

        let client = Client::builder(TokioExecutor::new()).build(TraceConnector {
            resolver: Resolver::new(),

            connected: Arc::clone(&connected),
            resolved: Arc::clone(&resolved),
        });

        let mut req = Request::builder()
            .method(Method::GET)
            .uri(target.url.as_str())
            .body(Full::<Bytes>::default())?;

        if let Some(auth) = &target.auth {
            auth.apply(&mut req);
        }

        for (key, value) in &target.headers {
            let key = HeaderName::from_bytes(key.as_bytes())?;
            let value = HeaderValue::from_bytes(value.as_bytes())?;

            req.headers_mut().insert(key, value);
        }

        let start = Utc::now();
        let resp = client.request(req).await?;
        let processed = Utc::now();

        let (parts, mut incoming) = resp.into_parts();

        let mut first_bytes_arrive = None;
        loop {
            match incoming.frame().await {
                Some(Ok(frame)) => {
                    if first_bytes_arrive.is_none() {
                        first_bytes_arrive = Some(Utc::now());
                    }

                    match frame.data_ref() {
                        Some(_data) => {
                            // trace.resp_bytes += data.len();
                        }
                        None => break,
                    }
                }
                Some(Err(err)) => {
                    // trace.read_err = true;
                    debug!(message = "read response failed", %err);
                    break;
                }
                None => break,
            }
        }

        let transferred = Utc::now();

        Ok((
            parts,
            Trace {
                start,
                resolved: *resolved.lock().unwrap(),
                connected: *connected.lock().unwrap(),
                processed,
                transferred,
            },
        ))
    })
    .await;

    match result {
        Ok(result) => result,
        Err(err) => Err(err.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use bytes::Bytes;
    use event::MetricValue;
    use framework::config::ProxyConfig;
    use framework::{Pipeline, ShutdownSignal};
    use http::Response;
    use hyper::body::Incoming;
    use hyper::service::service_fn;
    use hyper_util::rt::TokioIo;
    use testify::collect_one;
    use tokio::net::TcpListener;

    use crate::testing::trace_init;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }

    async fn handle(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, hyper::Error> {
        let text = req.uri().path().strip_prefix('/').unwrap().to_string();

        let status = text.parse::<u16>().unwrap();

        let resp = Response::builder()
            .status(status)
            .body(Full::new(Bytes::from(text)))
            .expect("build response success");

        Ok(resp)
    }

    #[tokio::test]
    async fn check() {
        use hyper::server::conn::http1;

        trace_init();

        let addr = testify::next_addr();
        let endpoint = format!("http://{}", addr);
        let listener = TcpListener::bind(addr).await.unwrap();

        // start http server
        tokio::spawn(async move {
            loop {
                let (conn, _peer) = listener.accept().await.unwrap();

                tokio::spawn(async move {
                    if let Err(err) = http1::Builder::new()
                        .serve_connection(TokioIo::new(conn), service_fn(handle))
                        .await
                    {
                        panic!("handle http connection failed, {err}")
                    }
                });
            }
        });

        for (code, class) in [
            // hyper client cannot handle `100`,
            // https://github.com/hyperium/hyper/issues/2565
            //
            // (100, "1xx"),
            (101, "1xx"),
            (200, "2xx"),
            (203, "2xx"),
            (301, "3xx"),
            (404, "4xx"),
            (502, "5xx"),
        ] {
            let (output, receiver) = Pipeline::new_test();
            let shutdown = ShutdownSignal::noop();

            let target = Target {
                url: Url::parse(format!("{endpoint}/{code}").as_str()).unwrap(),
                tls: None,
                auth: None,
                headers: Default::default(),
                timeout: default_timeout(),
            };

            let task = tokio::spawn(run(
                target,
                default_interval(),
                output,
                shutdown,
                ProxyConfig::default(),
            ));
            tokio::time::sleep(Duration::from_secs(1)).await;
            let events = collect_one(receiver).await;
            task.abort();

            let metrics = events.into_metrics().unwrap();
            assert_eq!(metrics.len(), 2);
            let metric = &metrics[1];
            assert_eq!(
                metric.value,
                MetricValue::Gauge(code as f64),
                "code: {code}"
            );
            assert_eq!(
                metric.tags().get("status_class").unwrap(),
                &tags::Value::from(class)
            );
        }
    }
}
