use std::num::ParseFloatError;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use configurable::configurable_component;
use event::{tags, Metric};
use framework::config::{default_interval, Output, SourceConfig, SourceContext};
use framework::http::{Auth, HttpClient, HttpError};
use framework::tls::TlsConfig;
use framework::{Pipeline, ShutdownSignal, Source};
use http::{Method, Request};
use http_body_util::{BodyExt, Full};
use thiserror::Error;
use tokio::task::JoinSet;
use url::Url;

#[configurable_component(source, name = "clickhouse_metrics")]
#[serde(deny_unknown_fields)]
struct Config {
    /// The endpoint of the ClickHouse server.
    #[configurable(required, format = "uri", example = "http://localhost:8123")]
    endpoint: Url,

    tls: Option<TlsConfig>,

    auth: Option<Auth>,

    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,
}

#[async_trait]
#[typetag::serde(name = "clickhouse_metrics")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        let client = HttpClient::new(&self.tls, &cx.proxy)?;

        Ok(Box::pin(run(
            client,
            self.auth.clone(),
            self.endpoint.clone(),
            self.interval,
            cx.output,
            cx.shutdown,
        )))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::metrics()]
    }
}

#[derive(Debug, Error)]
enum Error {
    #[error(transparent)]
    Http(#[from] HttpError),

    #[error("parse metric({key}) value failed, {err}")]
    Parse {
        key: String,
        #[source]
        err: ParseFloatError,
    },
}

fn parse_metric(input: &str) -> Result<Vec<(&str, f64)>, Error> {
    let mut pairs = vec![];

    for line in input.lines() {
        if let Some((key, value)) = line.split_once('\t') {
            let value = value.parse::<f64>().map_err(|err| Error::Parse {
                key: key.to_string(),
                err,
            })?;

            pairs.push((key, value));
        }
    }

    Ok(pairs)
}

struct Collector {
    metric_uri: String,
    async_metric_uri: String,
    events_uri: String,
    parts_uri: String,

    endpoint: String,
    client: HttpClient,
    auth: Option<Auth>,
}

impl Collector {
    fn new(client: HttpClient, auth: Option<Auth>, endpoint: Url) -> Collector {
        let metric_uri = endpoint
            .clone()
            .query_pairs_mut()
            .append_pair("query", "select metric, value from system.metrics")
            .finish()
            .to_string();
        let async_metric_uri = endpoint.clone()
            .query_pairs_mut()
            .append_pair("query", "select replaceRegexpAll(toString(metric), '-', '_') AS metric, value from system.asynchronous_metrics")
            .finish()
            .to_string();
        let events_uri = endpoint
            .clone()
            .query_pairs_mut()
            .append_pair("query", "select event, value from system.events")
            .finish()
            .to_string();
        let parts_uri = endpoint
            .clone()
            .query_pairs_mut()
            .append_pair("query", "select database, table, sum(bytes) as bytes, count() as parts, sum(rows) as rows from system.parts where active = 1 group by database, table")
            .finish()
            .to_string();

        let endpoint = match endpoint.port() {
            Some(port) => format!(
                "{}://{}:{}",
                endpoint.scheme(),
                endpoint.host_str().unwrap(),
                port
            ),
            None => format!("{}://{}", endpoint.scheme(), endpoint.host_str().unwrap()),
        };

        Collector {
            metric_uri,
            async_metric_uri,
            events_uri,
            parts_uri,

            endpoint,
            client,
            auth,
        }
    }

    async fn currently_metrics(&self) -> Result<Vec<Metric>, Error> {
        let data = self.fetch(&self.metric_uri).await?;

        let lines = parse_metric(unsafe { std::str::from_utf8_unchecked(&data) })?;

        let mut metrics = Vec::with_capacity(lines.len());
        for (key, value) in lines {
            metrics.push(Metric::gauge_with_tags(
                sanitize_metric_name(key),
                format!("Number of {key} currently processed"),
                value,
                tags!(
                    "endpoint" => self.endpoint.clone(),
                ),
            ))
        }

        Ok(metrics)
    }

    async fn async_metrics(&self) -> Result<Vec<Metric>, Error> {
        let data = self.fetch(&self.async_metric_uri).await?;

        let lines = parse_metric(unsafe { std::str::from_utf8_unchecked(&data) })?;

        let mut metrics = Vec::with_capacity(lines.len());
        for (key, value) in lines {
            metrics.push(Metric::gauge_with_tags(
                sanitize_metric_name(key),
                format!("Number of {key} async processed"),
                value,
                tags!(
                    "endpoint" => self.endpoint.clone()
                ),
            ))
        }

        Ok(metrics)
    }

    async fn event_metrics(&self) -> Result<Vec<Metric>, Error> {
        let data = self.fetch(&self.events_uri).await?;

        let lines = parse_metric(unsafe { std::str::from_utf8_unchecked(&data) })?;

        let mut metrics = Vec::with_capacity(lines.len());
        for (key, value) in lines {
            metrics.push(Metric::sum_with_tags(
                sanitize_metric_name(key) + "_total",
                format!("Number of {key} total processed"),
                value,
                tags!(
                    "endpoint" => self.endpoint.clone()
                ),
            ));
        }

        Ok(metrics)
    }

    async fn parts_metrics(&self) -> Result<Vec<Metric>, Error> {
        let body = self.fetch(&self.parts_uri).await?;

        let text = String::from_utf8_lossy(&body);

        // The response body looks like
        //
        // system	query_thread_log	41287	2	263
        // system	metric_log	3467014	5	74875
        // system	query_log	62836	4	518
        // system	asynchronous_metric_log	223293	5	96173
        // system	trace_log	17850	3	995
        //
        // 8 is the "system" database
        let mut metrics = Vec::with_capacity(3 * 8);
        for line in text.lines() {
            if line.is_empty() {
                continue;
            }

            let parts = line.split_ascii_whitespace().collect::<Vec<_>>();
            if parts.len() != 5 {
                continue;
            }

            let bytes: u64 = parts[2].parse().expect("parse ok");
            let count: u64 = parts[3].parse().expect("parse ok");
            let rows: u64 = parts[4].parse().expect("parse ok");
            let tags = tags!(
                "database" => parts[0].to_string(),
                "table" => parts[1].to_string(),
                "endpoint" => self.endpoint.clone(),
            );

            metrics.extend_from_slice(&[
                Metric::gauge_with_tags(
                    "clickhouse_table_parts_bytes",
                    "Table size in bytes",
                    bytes,
                    tags.clone(),
                ),
                Metric::gauge_with_tags(
                    "clickhouse_table_parts_count",
                    "Number of parts of the table",
                    count,
                    tags.clone(),
                ),
                Metric::gauge_with_tags(
                    "clickhouse_table_parts_rows",
                    "Number of rows in the table",
                    rows,
                    tags,
                ),
            ]);
        }

        Ok(metrics)
    }

    async fn fetch(&self, uri: &str) -> Result<Bytes, HttpError> {
        let mut req = Request::builder()
            .method(Method::GET)
            .uri(uri)
            .body(Full::<Bytes>::default())?;
        if let Some(auth) = &self.auth {
            auth.apply(&mut req);
        }

        let resp = self.client.send(req).await?;

        let (parts, incoming) = resp.into_parts();
        if !parts.status.is_success() {
            return Err(HttpError::UnexpectedStatus(parts.status));
        }

        incoming
            .collect()
            .await
            .map(|data| data.to_bytes())
            .map_err(Into::into)
    }
}

async fn run(
    client: HttpClient,
    auth: Option<Auth>,
    endpoint: Url,
    interval: Duration,
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let mut ticker = tokio::time::interval(interval);
    let collector = Arc::new(Collector::new(client, auth, endpoint));

    loop {
        tokio::select! {
            _ = &mut shutdown => return Ok(()),
            _ = ticker.tick() => {}
        }

        let mut group = JoinSet::new();

        let c = Arc::clone(&collector);
        group.spawn(async move {
            c.currently_metrics().await.unwrap_or_else(|err| {
                warn!(
                    message = "fetch currently metrics failed",
                    %err
                );

                vec![]
            })
        });

        let c = Arc::clone(&collector);
        group.spawn(async move {
            c.async_metrics().await.unwrap_or_else(|err| {
                warn!(
                    message = "fetch async metrics failed",
                    %err
                );

                vec![]
            })
        });

        let c = Arc::clone(&collector);
        group.spawn(async move {
            c.event_metrics().await.unwrap_or_else(|err| {
                warn!(
                    message = "fetch event metrics failed",
                    %err
                );

                vec![]
            })
        });

        let c = Arc::clone(&collector);
        group.spawn(async move {
            c.parts_metrics().await.unwrap_or_else(|err| {
                warn!(
                    message = "fetch parts metrics failed",
                    %err,
                    instance = c.endpoint
                );

                vec![]
            })
        });

        while let Some(Ok(metrics)) = group.join_next().await {
            if let Err(err) = output.send(metrics).await {
                warn!(
                    message = "send metrics failed",
                    %err
                );

                continue;
            }
        }
    }
}

// sanitize_metric_name convert the given string to snake case following the Golang format:
// acronyms are converted to lower-case and preceded by an underscore.
fn sanitize_metric_name(name: &str) -> String {
    let mut converted = String::from("clickhouse_");
    let name = name
        .replace("MHz", "Mhz")
        .replace("CPU", "Cpu")
        .replace("OS", "Os")
        .replace("IO", "Io")
        .replace("HTTP", "Http")
        .replace(".", "_");

    for (i, ch) in name.char_indices() {
        if ch == '.' {
            converted.push('_');
            continue;
        }

        if i > 0 && ch.is_uppercase() {
            converted.push('_');
        }

        converted.push(ch.to_ascii_lowercase());
    }

    converted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }

    #[test]
    fn sanitize_metric() {
        let input = "CPUFrequencyMHz_0";
        let sanitized = sanitize_metric_name(input);

        assert_eq!(sanitized, "clickhouse_cpu_frequency_mhz_0");
    }

    macro_rules! assert_eq_if {
        ($pair:expr, $key: expr, $value:expr) => {
            if $pair.0 == $key {
                assert_eq!($pair.1, $value, $key);
                continue;
            }
        };
    }

    #[test]
    fn metrics() {
        let input = include_str!("../../tests/clickhouse/metric.txt");

        let pairs = parse_metric(input).unwrap();
        for pair in pairs {
            assert_eq_if!(pair, "Query", 1.0);
            assert_eq_if!(pair, "PartMutation", 0.0);
            assert_eq_if!(pair, "BackgroundSchedulePoolSize", 512.0);
            assert_eq_if!(pair, "BackgroundBufferFlushSchedulePoolSize", 16.0);
        }
    }

    #[test]
    fn async_metrics() {
        // http://127.0.0.1:8123/?query=select+replaceRegexpAll%28toString%28metric%29%2C+%27-%27%2C+%27_%27%29+AS+metric%2C+value+from+system.asynchronous_metrics
        let input = include_str!("../../tests/clickhouse/async_metric.txt");

        let pairs = parse_metric(input).unwrap();
        for pair in pairs {
            assert_eq_if!(pair, "OSIOWaitTimeCPU14", 0.0);
            assert_eq_if!(pair, "OSUserTimeCPU25", 0.049997100168190256);
            assert_eq_if!(pair, "OSSystemTimeCPU27", 0.00999942003363805);
        }
    }

    #[test]
    fn events() {
        let input = include_str!("../../tests/clickhouse/event.txt");

        let pairs = parse_metric(input).unwrap();
        for pair in pairs {
            assert_eq_if!(pair, "Seek", 2663.0);
            assert_eq_if!(pair, "FileOpen", 849681.0);
            assert_eq_if!(pair, "ReadBufferFromFileDescriptorReadBytes", 10136645420.0);
            assert_eq_if!(pair, "OSCPUVirtualTimeMicroseconds", 802585162.0);
        }
    }
}

#[cfg(all(test, feature = "clickhouse-integration-tests"))]
mod integration_tests {
    use super::*;
    use crate::testing::components::{run_and_assert_source_compliance, SOURCE_TAGS};
    use crate::testing::{trace_init, ContainerBuilder, WaitFor};

    const PORT: u16 = 8123;

    #[tokio::test]
    async fn run() {
        trace_init();

        let container = ContainerBuilder::new("clickhouse/clickhouse-server:24.8-alpine")
            .with_port(PORT)
            .run()
            .unwrap();
        container
            .wait(WaitFor::Stderr("Logging errors to /var/log"))
            .unwrap();

        tokio::time::sleep(Duration::from_secs(2)).await;

        // TODO: make sure no warn or error log issued
        let addr = container.get_mapped_addr(PORT);
        let source = Config {
            endpoint: format!("http://{}", addr).parse().unwrap(),
            tls: None,
            auth: None,
            interval: Duration::from_secs(10),
        };

        let events =
            run_and_assert_source_compliance(source, Duration::from_secs(10), &SOURCE_TAGS).await;

        println!("got {} events", events.len());
    }
}
