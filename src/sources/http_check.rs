use std::time::{Duration, Instant};

use async_trait::async_trait;
use configurable::{configurable_component, Configurable};
use event::{tags, Metric};
use framework::config::{default_interval, Output, SourceConfig, SourceContext};
use framework::http::HttpClient;
use framework::tls::TlsConfig;
use framework::{Error, Pipeline, ShutdownSignal, Source};
use http::{Request, StatusCode};
use http_body_util::Full;
use serde::{Deserialize, Serialize};
use url::Url;

const fn default_timeout() -> Duration {
    Duration::from_secs(5)
}

/// Method is the HTTP methods that http check supported.
#[derive(Clone, Configurable, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
enum Method {
    #[default]
    Get,
}

/// The HTTP Check source can be used for synthethic checks against HTTP
/// endpoints. This source will make a request to the specified `endpoint`
/// using the configured `method`. This scraper generates a metric with
/// a label for each HTTP response status class with a value of `1` if
/// the status code matches the class.
#[configurable_component(source, name = "http_check")]
struct Config {
    /// The URL of the endpoint to be monitored.
    #[configurable(required, format = "uri", example = "http://localhost:80")]
    endpoint: Url,

    /// TLS configuration
    tls: Option<TlsConfig>,

    /// The method used to call the endpoint.
    #[serde(default)]
    method: Method,

    /// This sources collects metrics on an interval.
    #[serde(with = "humanize::duration::serde", default = "default_interval")]
    interval: Duration,

    /// Timeout for HTTP request, it's value should be less than `interval`.
    #[serde(with = "humanize::duration::serde", default = "default_timeout")]
    timeout: Duration,
}

#[async_trait]
#[typetag::serde(name = "http_check")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        let client = HttpClient::new(&self.tls, &cx.proxy)?;
        let method = match self.method {
            Method::Get => http::Method::GET,
        };

        Ok(Box::pin(run(
            client,
            method,
            self.endpoint.clone(),
            self.interval,
            self.timeout,
            cx.output,
            cx.shutdown,
        )))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::metrics()]
    }
}

async fn run(
    client: HttpClient,
    method: http::Method,
    endpoint: Url,
    interval: Duration,
    timeout: Duration,
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let mut ticker = tokio::time::interval(interval);

    loop {
        tokio::select! {
            biased;

            _ = &mut shutdown => break,
            _ = ticker.tick() => {}
        };

        let start = Instant::now();
        let result = probe(&client, &method, endpoint.as_str(), timeout).await;
        let elapsed = start.elapsed();
        let mut metrics = vec![Metric::gauge_with_tags(
            "http_check_duration",
            "Measures the duration of the HTTP check",
            elapsed,
            tags!(
                "url" => endpoint.to_string()
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
                        "url" => endpoint.to_string(),
                        "method" => method.as_str(),
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
                        "method" => method.as_str(),
                        "url" => endpoint.to_string(),
                        "err" => err.to_string(),
                    ),
                ));
            }
        }

        if let Err(err) = output.send(metrics).await {
            warn!(message = "send metrics failed", ?err);
            break;
        }
    }

    Ok(())
}

async fn probe(
    client: &HttpClient,
    method: &http::Method,
    url: &str,
    timeout: Duration,
) -> Result<StatusCode, Error> {
    let result = tokio::time::timeout(timeout, async move {
        let req = Request::builder()
            .method(method)
            .uri(url)
            .body(Full::default())?;

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
    use http::{Method, Response};
    use hyper::body::Incoming;
    use hyper::service::service_fn;
    use hyper_util::rt::TokioIo;
    use testify::collect_ready;
    use tokio::net::TcpListener;

    use super::*;
    use crate::testing::trace_init;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
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
            let client = HttpClient::new(&None, &ProxyConfig::default()).unwrap();

            let task = tokio::spawn(run(
                client,
                Method::GET,
                Url::parse(&format!("{endpoint}/{code}")).unwrap(),
                default_interval(),
                default_timeout(),
                output,
                shutdown,
            ));
            tokio::time::sleep(Duration::from_secs(1)).await;
            let events = collect_ready(receiver).await;
            task.abort();

            assert_eq!(events.len(), 2);
            let metric = events[1].as_metric();
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
