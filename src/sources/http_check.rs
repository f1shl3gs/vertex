use std::time::{Duration, Instant};

use async_trait::async_trait;
use configurable::{configurable_component, Configurable};
use event::{tags, Metric};
use framework::config::{
    default_interval, DataType, Output, ProxyConfig, SourceConfig, SourceContext,
};
use framework::http::HttpClient;
use framework::tls::MaybeTls;
use framework::{Pipeline, ShutdownSignal, Source};
use http::Request;
use hyper::Body;
use serde::{Deserialize, Serialize};
use tokio::time::{interval, timeout};
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

impl Method {
    const fn as_str(&self) -> &'static str {
        match self {
            Method::Get => "GET",
        }
    }
}

/// The HTTP Check source can be used for synthethic checks against HTTP
/// endpoints. This source will make a request to the specified `endpoint`
/// using the configured `method`. This scraper generates a metric with
/// a label for each HTTP response status class with a value of `1` if
/// the status code matches the class.
#[configurable_component(source, name = "http_check")]
#[derive(Clone, Debug)]
struct Config {
    /// The URL of the endpoint to be monitored.
    #[configurable(required, format = "uri", example = "http://localhost:80")]
    endpoint: Url,

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
        Ok(Box::pin(run(
            self.clone(),
            cx.proxy,
            cx.output,
            cx.shutdown,
        )))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }
}

async fn run(
    config: Config,
    proxy: ProxyConfig,
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let mut ticker = interval(config.interval);

    loop {
        tokio::select! {
            biased;

            _ = &mut shutdown => break,
            _ = ticker.tick() => {}
        };

        let start = Instant::now();
        let result = match timeout(
            config.timeout,
            scrape(config.method.as_str(), config.endpoint.as_str(), &proxy),
        )
        .await
        {
            Ok(result) => result,
            Err(err) => Err(err.into()),
        };
        let elapsed = start.elapsed().as_secs_f64();
        let mut metrics = vec![Metric::gauge_with_tags(
            "http_check_duration",
            "Measures the duration of the HTTP check",
            elapsed,
            tags!(
                "url" => config.endpoint.to_string()
            ),
        )];

        match result {
            Ok(status_code) => {
                let status_class = if status_code < 200 {
                    "1xx"
                } else if status_code < 300 {
                    "2xx"
                } else if status_code < 400 {
                    "3xx"
                } else {
                    "5xx"
                };

                metrics.push(Metric::gauge_with_tags(
                    "http_check_status",
                    "",
                    status_code,
                    tags!(
                        "url" => config.endpoint.to_string(),
                        "method" => config.method.as_str(),
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
                        "method" => config.method.as_str(),
                        "url" => config.endpoint.to_string(),
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

async fn scrape(
    method: &str,
    endpoint: &str,
    proxy: &ProxyConfig,
) -> Result<u16, framework::Error> {
    let tls = MaybeTls::from(None);
    let client = HttpClient::new(tls, proxy)?;

    let req = Request::builder()
        .method(method)
        .uri(endpoint)
        .body(Body::empty())?;

    let resp = client.send(req).await?;

    Ok(resp.status().as_u16())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }
}
