use std::collections::HashMap;
use std::time::{Duration, Instant};

use configurable::{Configurable, configurable_component};
use event::{Metric, tags};
use framework::config::{Output, SourceConfig, SourceContext, default_interval};
use framework::http::{Auth, HttpClient};
use framework::tls::TlsConfig;
use framework::{Error, Pipeline, ShutdownSignal, Source};
use futures_util::StreamExt;
use futures_util::stream::FuturesUnordered;
use http::{HeaderName, HeaderValue, Request, StatusCode};
use http_body_util::Full;
use serde::{Deserialize, Serialize};
use url::Url;

const fn default_timeout() -> Duration {
    Duration::from_secs(5)
}

/// Method is the HTTP methods that http check supported.
#[derive(Clone, Copy, Configurable, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
enum Method {
    #[default]
    Get,
}

impl From<Method> for http::Method {
    fn from(method: Method) -> Self {
        match method {
            Method::Get => http::Method::GET,
        }
    }
}

impl Method {
    fn as_str(self) -> &'static str {
        match self {
            Method::Get => "GET",
        }
    }
}

#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
struct Target {
    /// The method used to call the endpoint.
    #[serde(default)]
    method: Method,

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
        let mut targets = vec![];
        let interval = self.interval;

        for target in &self.targets {
            let client = HttpClient::new(target.tls.as_ref(), &cx.proxy)?;
            targets.push((client, target.clone()));
        }

        Ok(Box::pin(async move {
            let tasks = FuturesUnordered::new();
            for (client, target) in targets {
                tasks.push(tokio::spawn(run(
                    client,
                    target,
                    interval,
                    cx.output.clone(),
                    cx.shutdown.clone(),
                )))
            }

            let _ = tasks.collect::<Vec<_>>().await;

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
    client: HttpClient,
    target: Target,
    interval: Duration,
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
) {
    let mut ticker = tokio::time::interval(interval);

    loop {
        tokio::select! {
            _ = ticker.tick() => {},
            _ = &mut shutdown => {
                break;
            }
        }

        let start = Instant::now();
        let result = probe(&client, &target).await;
        let elapsed = start.elapsed();
        let mut metrics = vec![Metric::gauge_with_tags(
            "http_check_duration",
            "Measures the duration of the HTTP check",
            elapsed,
            tags!(
                "url" => target.url.to_string(),
            ),
        )];

        match result {
            Ok(status_code) => {
                let status_class = if status_code.is_informational() {
                    "1xx"
                } else if status_code.is_success() {
                    "2xx"
                } else if status_code.is_redirection() {
                    "3xx"
                } else if status_code.is_client_error() {
                    "4xx"
                } else {
                    "5xx"
                };

                metrics.push(Metric::gauge_with_tags(
                    "http_check_status",
                    "The check resulted in status_code.",
                    status_code.as_u16(),
                    tags!(
                        "url" => target.url.to_string(),
                        "method" => target.method.as_str(),
                        "status_class" => status_class,
                    ),
                ))
            }
            Err(err) => {
                metrics.push(Metric::gauge_with_tags(
                    "http_check_error",
                    "Records errors occurring during HTTP check.",
                    1,
                    tags!(
                        "method" => target.method.as_str(),
                        "url" => target.url.to_string(),
                        "err" => err.to_string(),
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

async fn probe(client: &HttpClient, target: &Target) -> Result<StatusCode, Error> {
    let result = tokio::time::timeout(target.timeout, async move {
        let mut req = Request::builder()
            .method::<http::Method>(target.method.into())
            .uri(target.url.as_str())
            .body(Full::default())?;

        if let Some(auth) = &target.auth {
            auth.apply(&mut req);
        }

        for (key, value) in &target.headers {
            let key = HeaderName::from_bytes(key.as_bytes())?;
            let value = HeaderValue::from_bytes(value.as_bytes())?;

            req.headers_mut().insert(key, value);
        }

        let resp = client.send(req).await?;

        Ok(resp.status())
    })
    .await;

    match result {
        Ok(result) => result,
        Err(err) => Err(err.into()),
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use event::MetricValue;
    use framework::config::ProxyConfig;
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
            let client = HttpClient::new(None, &ProxyConfig::default()).unwrap();
            let target = Target {
                method: Method::Get,
                url: Url::parse(format!("{endpoint}/{code}").as_str()).unwrap(),
                tls: None,
                auth: None,
                headers: Default::default(),
                timeout: default_timeout(),
            };

            let task = tokio::spawn(run(client, target, default_interval(), output, shutdown));
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
