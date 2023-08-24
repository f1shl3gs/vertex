use std::io::BufRead;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use bytes::{Buf, Bytes};
use configurable::configurable_component;
use event::{tags, Metric};
use framework::config::{default_interval, DataType, Output, SourceConfig, SourceContext};
use framework::http::{Auth, HttpClient};
use framework::tls::TlsConfig;
use framework::{Pipeline, ShutdownSignal, Source};
use http::{Method, Request};
use hyper::Body;
use tokio::task::JoinError;
use url::Url;

#[configurable_component(source, name = "clickhouse_metrics")]
#[derive(Debug)]
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
        vec![Output::default(DataType::Metric)]
    }
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

        Collector {
            metric_uri,
            async_metric_uri,
            events_uri,
            parts_uri,

            endpoint: endpoint.to_string(),
            client,
            auth,
        }
    }

    async fn currently_metrics(&self) -> Vec<Metric> {
        let metrics = match self.fetch_metrics(&self.metric_uri).await {
            Ok(ms) => ms,
            Err(err) => {
                warn!(message = "fetch metrics field", ?err);
                vec![]
            }
        };

        metrics
            .iter()
            .map(|(key, value)| {
                Metric::gauge_with_tags(
                    sanitize_metric_name(key),
                    format!("Number of {key} currently processed"),
                    *value,
                    tags!(
                        "endpoint" => self.endpoint.clone(),
                    ),
                )
            })
            .collect::<Vec<_>>()
    }

    async fn async_metrics(&self) -> Vec<Metric> {
        let metrics = match self.fetch_metrics(&self.async_metric_uri).await {
            Ok(ms) => ms,
            Err(err) => {
                warn!(message = "fetch async metrics failed", ?err);
                vec![]
            }
        };

        metrics
            .iter()
            .map(|(key, value)| {
                Metric::gauge_with_tags(
                    sanitize_metric_name(key),
                    format!("Number of {key} async processed"),
                    *value,
                    tags!(
                        "endpoint" => self.endpoint.clone()
                    ),
                )
            })
            .collect::<Vec<_>>()
    }

    async fn event_metrics(&self) -> Vec<Metric> {
        let metrics = match self.fetch_metrics(&self.events_uri).await {
            Ok(ms) => ms,
            Err(err) => {
                warn!(message = "fetch events metrics failed", ?err);
                vec![]
            }
        };

        metrics
            .iter()
            .map(|(key, value)| {
                Metric::sum_with_tags(
                    sanitize_metric_name(key) + "_total",
                    format!("Number of {key} total processed"),
                    *value,
                    tags!(
                        "endpoint" => &self.endpoint
                    ),
                )
            })
            .collect::<Vec<_>>()
    }

    async fn parts_metrics(&self) -> Vec<Metric> {
        let body = match self.fetch(&self.parts_uri).await {
            Ok(body) => body,
            Err(err) => {
                warn!(message = "fetch parts metrics failed", ?err);

                return vec![];
            }
        };

        // The response body looks like
        //
        // system	query_thread_log	41287	2	263
        // system	metric_log	3467014	5	74875
        // system	query_log	62836	4	518
        // system	asynchronous_metric_log	223293	5	96173
        // system	trace_log	17850	3	995
        //
        // database table bytes count rows
        body.lines()
            .filter_map(|line| {
                if line.is_err() {
                    return None;
                }

                let line = match line {
                    Ok(line) => line,
                    Err(_err) => return None,
                };

                let parts = line.split_ascii_whitespace().collect::<Vec<_>>();
                if parts.len() != 5 {
                    return None;
                }

                let bytes: u64 = parts[2].parse().ok()?;
                let count: u64 = parts[3].parse().ok()?;
                let rows: u64 = parts[4].parse().ok()?;
                let tags = tags!(
                    "database" => parts[0].to_string(),
                    "table" => parts[1].to_string(),
                    "endpoint" => self.endpoint.clone(),
                );
                Some(vec![
                    Metric::gauge_with_tags(
                        "table_parts_bytes",
                        "Table size in bytes",
                        bytes,
                        tags.clone(),
                    ),
                    Metric::gauge_with_tags(
                        "table_parts_count",
                        "Number of parts of the table",
                        count,
                        tags.clone(),
                    ),
                    Metric::gauge_with_tags(
                        "table_parts_rows",
                        "Number of rows in the table",
                        rows,
                        tags,
                    ),
                ])
            })
            .flatten()
            .collect()
    }

    async fn fetch(&self, uri: &str) -> crate::Result<Bytes> {
        let mut req = Request::builder()
            .method(Method::GET)
            .uri(uri)
            .body(Body::empty())?;

        if let Some(auth) = &self.auth {
            auth.apply(&mut req);
        }

        let resp = self.client.send(req).await?;
        let (parts, body) = resp.into_parts();

        if !parts.status.is_success() {
            return Err(format!("unexpected status code {}", parts.status).into());
        }

        hyper::body::to_bytes(body).await.map_err(Into::into)
    }

    async fn fetch_metrics(&self, uri: &str) -> crate::Result<Vec<(String, f64)>> {
        let body = self.fetch(uri).await?;
        let buf = body.reader();

        let results = buf
            .lines()
            .filter_map(|s| match s {
                Ok(line) => {
                    let parts = line.split_ascii_whitespace().collect::<Vec<_>>();
                    if parts.len() != 2 {
                        return None;
                    }

                    let v = parts[1].parse::<f64>().ok()?;

                    Some((parts[0].to_string(), v))
                }
                Err(_) => None,
            })
            .collect::<Vec<_>>();

        Ok(results)
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
            biased;

            _ = &mut shutdown => return Ok(()),
            _ = ticker.tick() => {}
        }

        let mut tasks = vec![];
        let c = Arc::clone(&collector);
        tasks.push(tokio::spawn(async move { c.currently_metrics().await }));
        let c = Arc::clone(&collector);
        tasks.push(tokio::spawn(async move { c.async_metrics().await }));
        let c = Arc::clone(&collector);
        tasks.push(tokio::spawn(async move { c.event_metrics().await }));
        let c = Arc::clone(&collector);
        tasks.push(tokio::spawn(async move { c.parts_metrics().await }));

        match futures::future::join_all(tasks)
            .await
            .into_iter()
            .collect::<Result<Vec<Vec<Metric>>, JoinError>>()
        {
            Ok(metrics) => {
                if let Err(err) = output
                    .send(metrics.into_iter().flatten().collect::<Vec<_>>())
                    .await
                {
                    warn!(message = "send metrics failed", ?err);

                    return Ok(());
                }
            }
            Err(err) => {
                error!(message = "spawn tasks failed", ?err);
            }
        }
    }
}

#[instrument(skip(client, auth))]
async fn fetch_metrics(
    client: &HttpClient,
    auth: Option<&Auth>,
    url: &str,
) -> crate::Result<Vec<(String, i64)>> {
    let mut req = Request::builder()
        .method(Method::GET)
        .uri(url)
        .body(Body::empty())?;

    if let Some(auth) = auth {
        auth.apply(&mut req);
    }

    let resp = client.send(req).await?;
    let (parts, body) = resp.into_parts();

    if !parts.status.is_success() {
        return Err(format!("unexpected status code {}", parts.status).into());
    }

    let buf = hyper::body::aggregate(body).await?.reader();
    let results = buf
        .lines()
        .filter_map(|s| match s {
            Ok(line) => {
                let parts = line.split_ascii_whitespace().collect::<Vec<_>>();
                if parts.len() != 2 {
                    return None;
                }

                let v = parts[1].parse::<i64>().ok()?;

                Some((parts[0].to_string(), v))
            }
            Err(_) => None,
        })
        .collect::<Vec<_>>();

    Ok(results)
}

// sanitize_metric_name convert the given string to snake case following the Golang format:
// acronyms are converted to lower-case and preceded by an underscore.
fn sanitize_metric_name(name: &str) -> String {
    let mut converted = String::from("clickhouse_");
    let name = name.replace("MHz", "_mhz").replace("CPU", "cpu");

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
    use framework::config::ProxyConfig;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>();
    }

    #[test]
    fn sanitize_metric() {
        let input = "CPUFrequencyMHz_0";
        let sanitized = sanitize_metric_name(input);

        assert_eq!(sanitized, "clickhouse_cpu_frequency_mhz_0");
    }

    #[allow(clippy::print_stdout)]
    #[ignore]
    #[tokio::test]
    async fn parts() {
        let endpoint = "http://127.0.0.1:8123".parse().unwrap();
        let client = HttpClient::new(&None, &ProxyConfig::default()).unwrap();
        let c = Collector::new(client, None, endpoint);

        let body = c.fetch(&c.parts_uri).await.unwrap();
        println!("{}", String::from_utf8_lossy(body.as_ref()))
    }
}
