use std::cell::RefCell;
use std::collections::HashMap;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant, SystemTime};

use bytes::Bytes;
use chrono::Utc;
use configurable::configurable_component;
use event::{Metric, tags};
use framework::config::{Output, ProxyConfig, SourceConfig, SourceContext, default_interval};
use framework::dns::{DnsError, Resolver};
use framework::http::Auth;
use framework::tls::{TlsConfig, TlsError};
use framework::{Error, Pipeline, ShutdownSignal, Source};
use http::header::{CONTENT_LENGTH, LAST_MODIFIED, LOCATION};
use http::response::Parts;
use http::uri::Scheme;
use http::{HeaderMap, HeaderName, HeaderValue, Method, Request, Uri, Version};
use http_body_util::{BodyExt, Full};
use httpdate::HttpDate;
use hyper_rustls::ConfigBuilderExt;
use hyper_util::client::legacy::Client;
use hyper_util::rt::{TokioExecutor, TokioIo};
use rustls::ClientConfig;
use tokio::net::TcpStream;
use tokio::task::JoinSet;
use url::Url;

use crate::common::offset::offset;

const fn default_timeout() -> Duration {
    Duration::from_secs(5)
}

/// The HTTP Check source can be used for synthetic checks against HTTP
/// endpoints. This source will make a request to the specified `endpoint`
/// using the configured `method`. This scraper generates a metric with
/// a label for each HTTP response status class with a value of `1` if
/// the status code matches the class.
#[configurable_component(source, name = "http_check")]
struct Config {
    /// Targets to probe
    #[configurable(required, format = "uri", example = "http://127.0.0.1:8080")]
    targets: Vec<Url>,

    /// TLS configuration
    tls: Option<TlsConfig>,

    auth: Option<Auth>,

    /// Extra HTTP headers
    #[serde(default)]
    headers: HashMap<String, String>,

    /// Timeout for HTTP request, it's value should be less than `interval`.
    #[serde(with = "humanize::duration::serde", default = "default_timeout")]
    timeout: Duration,

    /// This sources collects metrics on an interval.
    #[serde(with = "humanize::duration::serde", default = "default_interval")]
    interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "http_check")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        let mut headers = HeaderMap::new();
        for (key, value) in &self.headers {
            let key = HeaderName::from_bytes(key.as_bytes())?;
            let value = HeaderValue::from_bytes(value.as_bytes())?;

            headers.insert(key, value);
        }

        let interval = self.interval;
        let timeout = self.timeout;
        let targets = self.targets.clone();
        let tls = self.tls.clone();
        let auth = self.auth.clone();

        Ok(Box::pin(async move {
            let mut tasks = JoinSet::new();

            for target in targets {
                tasks.spawn(run(
                    target,
                    tls.clone(),
                    headers.clone(),
                    auth.clone(),
                    timeout,
                    interval,
                    cx.proxy.clone(),
                    cx.output.clone(),
                    cx.shutdown.clone(),
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
    target: Url,
    tls: Option<TlsConfig>,
    headers: HeaderMap,
    auth: Option<Auth>,
    timeout: Duration,
    interval: Duration,
    _proxy: ProxyConfig,
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
) {
    let now = Utc::now()
        .timestamp_nanos_opt()
        .expect("timestamp can not be represented in a timestamp with nanosecond precision.");
    let mut ticker = tokio::time::interval_at(
        tokio::time::Instant::now() + offset(&target, interval, 0, now),
        interval,
    );

    let instance = target.host_str().unwrap_or("");

    loop {
        tokio::select! {
            _ = ticker.tick() => {},
            _ = &mut shutdown => {
                break;
            }
        }

        let result = probe(&target, tls.as_ref(), auth.as_ref(), &headers, timeout).await;

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
                        "http_redirects",
                        "The number of redirects",
                        trace.redirects(),
                        tags!(
                            "instance" => instance,
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "http_duration_seconds",
                        "Duration of http request by phase",
                        trace.resolving(),
                        tags!(
                            "phase" => "resolve",
                            "instance" => instance,
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "http_duration_seconds",
                        "Duration of http request by phase",
                        trace.connecting(),
                        tags!(
                            "phase" => "connect",
                            "instance" => instance,
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "http_duration_seconds",
                        "Duration of http request by phase",
                        trace.processing(),
                        tags!(
                            "phase" => "processing",
                            "instance" => instance,
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "http_duration_seconds",
                        "Duration of http request by phase",
                        trace.transferring(),
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
                debug!(message = "http check failed", ?target, ?err);

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

#[derive(Debug)]
struct Inner {
    start: Instant,
    resolved: Instant,
    connected: Instant,
    processed: Instant,
    transferred: Instant,

    redirects: u64,
}

#[derive(Debug)]
struct Trace(RefCell<Inner>);

unsafe impl Sync for Trace {}

impl Trace {
    fn new() -> Self {
        let now = Instant::now();

        Self(RefCell::new(Inner {
            start: now,
            resolved: now,
            connected: now,
            processed: now,
            transferred: now,
            redirects: 0,
        }))
    }

    #[inline]
    fn redirected(&self) {
        self.0.borrow_mut().redirects += 1;
    }

    #[inline]
    fn redirects(&self) -> u64 {
        self.0.borrow().redirects
    }

    #[inline]
    fn resolved(&self) {
        self.0.borrow_mut().resolved = Instant::now();
    }

    #[inline]
    fn resolving(&self) -> Duration {
        let inner = self.0.borrow();
        inner.resolved - inner.start
    }

    #[inline]
    fn connected(&self) {
        self.0.borrow_mut().connected = Instant::now();
    }

    #[inline]
    fn connecting(&self) -> Duration {
        let inner = self.0.borrow();
        inner.connected - inner.resolved
    }

    #[inline]
    fn processed(&self) {
        self.0.borrow_mut().processed = Instant::now();
    }

    #[inline]
    fn processing(&self) -> Duration {
        let inner = self.0.borrow();
        inner.processed - inner.connected
    }

    #[inline]
    fn transferred(&self) {
        self.0.borrow_mut().transferred = Instant::now();
    }

    #[inline]
    fn transferring(&self) -> Duration {
        let inner = self.0.borrow();
        inner.transferred - inner.processed
    }
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
    trace: Arc<Trace>,
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
        let trace = Arc::clone(&self.trace);

        Box::pin(async move {
            let (host, port) = get_host_port(&dst)?;
            let host = host.trim_start_matches('[').trim_end_matches(']');

            let addrs = resolver
                .lookup_ip(host)
                .await
                .map_err(ConnectError::Resolve)?;

            trace.resolved();

            for mut addr in addrs {
                addr.set_port(port);

                match TcpStream::connect(addr).await {
                    Ok(stream) => {
                        trace.connected();
                        return Ok(TokioIo::new(stream));
                    }
                    Err(_err) => continue,
                }
            }

            trace.connected();

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

async fn probe(
    target: &Url,
    tls: Option<&TlsConfig>,
    auth: Option<&Auth>,
    headers: &HeaderMap,
    timeout: Duration,
) -> Result<(Parts, Arc<Trace>), Error> {
    let result = tokio::time::timeout(timeout, async move {
        let trace = Arc::new(Trace::new());

        let connector = TraceConnector {
            resolver: Resolver::new(),
            trace: Arc::clone(&trace),
        };

        let config = match tls {
            Some(config) => config.client_config()?,
            None => ClientConfig::builder()
                .with_native_roots()
                .map_err(TlsError::NativeCerts)?
                .with_no_client_auth(),
        };
        let connector = hyper_rustls::HttpsConnector::from((connector, config));

        let client = Client::builder(TokioExecutor::new()).build(connector);

        let mut uri = target.to_string();
        let (parts, mut incoming) = loop {
            let mut req = Request::builder()
                .method(Method::GET)
                .uri(&uri)
                .body(Full::<Bytes>::default())?;

            if let Some(auth) = auth {
                auth.apply(&mut req);
            }

            req.headers_mut().extend(
                headers
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone())),
            );

            let resp = client.request(req).await?;
            trace.processed();

            let (parts, incoming) = resp.into_parts();
            if parts.status.is_redirection() {
                trace.redirected();

                if let Some(to) = parts.headers.get(LOCATION) {
                    uri = to.to_str()?.to_string();
                    continue;
                } else {
                    break (parts, incoming);
                }
            } else {
                break (parts, incoming);
            }
        };

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

        trace.transferred();

        Ok((parts, trace))
    })
    .await;

    result.unwrap_or_else(|err| Err(err.into()))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

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

    use super::*;
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
        let content = std::fs::read_to_string("/etc/hosts").unwrap();
        println!("{}", content);

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

        for code in [
            // hyper client cannot handle `100`,
            // https://github.com/hyperium/hyper/issues/2565
            101, 200, 203, 301, 404, 502,
        ] {
            let (output, receiver) = Pipeline::new_test();
            let shutdown = ShutdownSignal::noop();

            let task = tokio::spawn(run(
                Url::parse(format!("{endpoint}/{code}").as_str()).unwrap(),
                None,
                Default::default(),
                None,
                default_timeout(),
                Duration::from_secs(1),
                ProxyConfig::default(),
                output,
                shutdown,
            ));

            tokio::task::yield_now().await;

            let events = collect_one(receiver).await;
            task.abort();

            let metrics = events.into_metrics().unwrap();

            assert!(
                metrics.iter().any(|m| m.name() == "http_status_code"
                    && m.value == MetricValue::Gauge(code as f64))
            );
        }
    }
}
