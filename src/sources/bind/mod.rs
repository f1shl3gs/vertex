mod json;
mod xml;

use std::num::ParseFloatError;
use std::time::{Duration, Instant};

use bytes::Buf;
use chrono::{DateTime, Utc};
use configurable::configurable_component;
use event::{Bucket, Metric, tags};
use framework::config::{OutputType, SourceConfig, SourceContext, default_interval};
use framework::http::{HttpClient, HttpError};
use framework::{Pipeline, ShutdownSignal, Source};
use http::StatusCode;
use http::header::CONTENT_TYPE;
use http_body_util::{BodyExt, Full};
use serde::Deserialize;
use tokio::task::JoinSet;

use crate::common::calculate_start;

/// Make sure BIND was built with libxml2 support. You can check with the following command:
///    named -V | grep libxml2
///
/// Configure BIND to open a statistics channel. e.g.
///
/// statistics-channels {
///   inet 127.0.0.1 port 8053 allow { 127.0.0.1; };
/// };
///
/// BIND Version	Statistics Format	Example URL                     Release Date
/// 9.6 - 9.8	    XML v2	            http://localhost:8053           2008-12-23 - 2014-09-29
/// 9.9	            XML v2	            http://localhost:8053/xml/v2    2012-02-29
/// 9.9+	        XML v3	            http://localhost:8053/xml/v3    2012-05-21
/// 9.10+	        JSON v1	            http://localhost:8053/json/v1   2014-04-30
#[configurable_component(source, name = "bind")]
#[serde(deny_unknown_fields)]
struct Config {
    /// Endpoint for the BIND statistics api
    #[configurable(required, format = "uri", example = "http://127.0.0.1:8053")]
    endpoints: Vec<String>,

    /// Duration between each scrape.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "bind")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        let http_client = HttpClient::new(None, &cx.proxy)?;
        let endpoints = self.endpoints.clone();
        let interval = self.interval;
        let shutdown = cx.shutdown;
        let output = cx.output;

        Ok(Box::pin(async move {
            let mut tasks = JoinSet::from_iter(endpoints.into_iter().map(|endpoint| {
                run(
                    Client {
                        endpoint,
                        client: http_client.clone(),
                    },
                    interval,
                    output.clone(),
                    shutdown.clone(),
                )
            }));

            while tasks.join_next().await.is_some() {}

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::metric()]
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

fn statistics_to_metrics(s: Statistics) -> Vec<Metric> {
    let mut metrics = server_metrics(s.server);
    metrics.extend(task_metrics(s.task_manager));
    metrics.extend(view_metrics(s.views, s.zone_views));
    metrics
}

fn server_metric(c: Counter) -> Option<Metric> {
    match c.name.as_str() {
        "QryDuplicate" => Some(Metric::sum(
            "bind_query_duplicates_total",
            "Number of duplicated queries received",
            c.value,
        )),
        "QryRecursion" => Some(Metric::sum(
            "bind_query_recursions_total",
            "Number of queries causing recursion",
            c.value,
        )),
        "XfrRej" => Some(Metric::sum(
            "bind_zone_transfer_rejected_total",
            "Number of rejected zone transfers",
            c.value,
        )),
        "XfrSuccess" => Some(Metric::sum(
            "bind_zone_transfer_success_total",
            "Number of successful zone transfers",
            c.value,
        )),
        "XfrFail" => Some(Metric::sum(
            "bind_zone_transfer_failure_total",
            "Number of failed zone transfers",
            c.value,
        )),
        "RecursClients" => Some(Metric::sum(
            "bind_recursive_clients",
            "Number of current recursive clients",
            c.value,
        )),
        _ => None,
    }
}

fn server_metrics(s: Server) -> Vec<Metric> {
    let mut metrics = vec![Metric::gauge(
        "bind_boot_time_seconds",
        "Start time of the BIND process since unix epoch in seconds",
        s.boot_time.timestamp(),
    )];

    if s.config_time.timestamp() != 0 {
        metrics.push(Metric::gauge(
            "bind_config_time_seconds",
            "Time of the last reconfiguration since unix epoch in seconds",
            s.config_time.timestamp(),
        ));
    }

    for c in s.incoming_queries {
        metrics.push(Metric::sum_with_tags(
            "bind_incoming_queries_total",
            "Number of incoming DNS queries",
            c.value,
            tags!(
                "type" => c.name.clone(),
            ),
        ));
    }

    for s in s.incoming_requests {
        metrics.push(Metric::sum_with_tags(
            "bind_incoming_requests_total",
            "Number of incoming DNS requests",
            s.value,
            tags!(
                "opcode" => s.name,
            ),
        ));
    }

    for c in s.name_server_stats {
        match c.name.as_str() {
            "QryDropped" | "QryFailure" => {
                let name = c
                    .name
                    .strip_prefix("Qry")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| c.name.clone());

                metrics.push(Metric::sum_with_tags(
                    "bind_query_errors_total",
                    "Number of query failures",
                    c.value,
                    tags!(
                        "error" => name,
                    ),
                ));
            }
            "QrySuccess" | "QryReferral" | "QryNxrrset" | "QrySERVFAIL" | "QryFORMERR"
            | "QryNXDOMAIN" => {
                let name = c
                    .name
                    .strip_prefix("Qry")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| c.name.clone());

                metrics.push(Metric::sum_with_tags(
                    "bind_responses_total",
                    "Number of response sent",
                    c.value,
                    tags!(
                        "result" => name,
                    ),
                ));
            }
            _ => {}
        }

        if let Some(m) = server_metric(c) {
            metrics.push(m);
        }
    }

    for c in s.server_rcodes {
        metrics.push(Metric::sum_with_tags(
            "bind_response_rcodes_total",
            "Number of response sent per RCODE",
            c.value,
            tags!(
                "rcode" => c.name,
            ),
        ));
    }

    for c in s.zone_statistics {
        if let Some(m) = server_metric(c) {
            metrics.push(m);
        }
    }

    metrics
}

// TODO: maybe embed this
fn histogram(stats: &Vec<Counter>) -> Result<(Vec<Bucket>, u64), ParseFloatError> {
    let mut count = 0;
    let mut buckets = vec![];

    for c in stats {
        if c.name.starts_with("QryRTT") {
            let mut b = f64::INFINITY;

            if !c.name.ends_with('+')
                && let Some(rtt) = c.name.strip_prefix("QryRTT")
            {
                b = rtt.parse()?;
            }

            count += c.value;
            buckets.push(Bucket {
                upper: b / 1000.0,
                count,
            });
        }
    }

    buckets.sort_by(|a, b| a.upper.total_cmp(&b.upper));

    Ok((buckets, count))
}

fn view_metrics(views: Vec<View>, zone_views: Vec<ZoneView>) -> Vec<Metric> {
    let mut metrics = vec![];

    for view in views {
        for g in view.cache {
            metrics.push(Metric::gauge_with_tags(
                "bind_resolver_cache_rrsets",
                "Number of RRSets in Cache database",
                g.value,
                tags!(
                    "view" => view.name.clone(),
                    "type" => g.name
                ),
            ));
        }

        for c in view.resolver_queries {
            metrics.push(Metric::sum_with_tags(
                "bind_resolver_queries_total",
                "Number of outgoing DNS queries",
                c.value,
                tags!(
                    "view" => view.name.clone(),
                    "type" => c.name,
                ),
            ))
        }

        match histogram(&view.resolver_stats) {
            Ok((buckets, count)) => metrics.push(Metric::histogram_with_tags(
                "bind_resolver_query_duration_seconds",
                "Resolver query round-trip time in seconds",
                tags!(
                    "view" => view.name.clone()
                ),
                count,
                0,
                buckets,
            )),
            Err(err) => {
                warn!(
                    message = "Error parsing RTT",
                    %err,
                    internal_log_rate_limit = true
                );
            }
        }

        for c in view.resolver_stats {
            match c.name.as_str() {
                "Lame" => metrics.push(Metric::sum_with_tags(
                    "bind_resolver_response_lame_total",
                    "Number of lame delegation responses received",
                    c.value,
                    tags!(
                        "view" => view.name.clone(),
                    ),
                )),
                "EDNS0Fail" => metrics.push(Metric::sum_with_tags(
                    "bind_resolver_query_edns0_errors_total",
                    "Number of EDNS(0) query errors",
                    c.value,
                    tags!(
                        "view" => view.name.clone(),
                    ),
                )),
                "Mismatch" => metrics.push(Metric::sum_with_tags(
                    "bind_resolver_response_mismatch_total",
                    "Number of mismatch responses received",
                    c.value,
                    tags!(
                        "view" => view.name.clone()
                    ),
                )),
                "Retry" => metrics.push(Metric::sum_with_tags(
                    "bind_resolver_query_retries_total",
                    "Number of resolver query retries",
                    c.value,
                    tags!(
                        "view" => view.name.clone(),
                    ),
                )),
                "Truncated" => metrics.push(Metric::sum_with_tags(
                    "bind_resolver_response_truncated_total",
                    "Number of truncated responses received",
                    c.value,
                    tags!(
                        "view" => view.name.clone(),
                    ),
                )),
                "ValFail" => metrics.push(Metric::sum_with_tags(
                    "bind_resolver_dnssec_validation_errors_total",
                    "Number of DNSSEC validation attempt errors",
                    c.value,
                    tags!(
                        "view" => view.name.clone()
                    ),
                )),
                _ => {}
            }

            match c.name.as_str() {
                "QueryAbort" | "QuerySockFail" | "QueryTimeout" => {
                    metrics.push(Metric::sum_with_tags(
                        "bind_resolver_query_errors_total",
                        "Number of resolver queries failed",
                        c.value,
                        tags!(
                            "view" => view.name.clone(),
                            "error" => c.name,
                        ),
                    ))
                }
                "NXDOMAIN" | "SERVFAIL" | "FORMERR" | "OtherError" | "REFUSED" => {
                    metrics.push(Metric::sum_with_tags(
                        "bind_resolver_response_errors_total",
                        "Number of resolver response errors received",
                        c.value,
                        tags!(
                            "view" => view.name.clone(),
                            "error" => c.name,
                        ),
                    ))
                }
                "ValOk" | "ValNegOk" => metrics.push(Metric::sum_with_tags(
                    "bind_resolver_dnssec_validation_success_total",
                    "Number of DNSSEC validation attempts succeeded",
                    c.value,
                    tags!(
                        "view" => view.name.clone(),
                        "error" => c.name,
                    ),
                )),
                _ => {}
            }
        }
    }

    for view in zone_views {
        for zone in view.zone_data {
            metrics.push(Metric::sum_with_tags(
                "bind_zone_serial",
                "Zone serial number",
                zone.serial,
                tags!(
                    "view" => view.name.clone(),
                    "zone_name" => zone.name,
                ),
            ))
        }
    }

    metrics
}

fn task_metrics(s: TaskManager) -> Vec<Metric> {
    vec![
        Metric::gauge(
            "bind_tasks_running",
            "Number of running tasks",
            s.thread_model.tasks_running,
        ),
        Metric::gauge(
            "bind_worker_threads",
            "Total number of available worker threads",
            s.thread_model.worker_threads,
        ),
    ]
}

#[derive(Clone, Copy)]
enum Version {
    XmlV3,
    JsonV1,
}

/// Counter represents a single counter value.
#[derive(Deserialize)]
pub struct Counter {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(alias = "$value")]
    pub value: u64,
}

#[derive(Default)]
struct Server {
    boot_time: DateTime<Utc>,
    config_time: DateTime<Utc>,
    incoming_queries: Vec<Counter>,
    incoming_requests: Vec<Counter>,
    name_server_stats: Vec<Counter>,
    zone_statistics: Vec<Counter>,
    server_rcodes: Vec<Counter>,
}

/// Gauge represents a single gauge value.
#[derive(Deserialize)]
pub struct Gauge {
    pub name: String,
    #[serde(rename = "counter")]
    pub value: u64,
}

/// View represents statistics for a single BIND view.
pub struct View {
    pub name: String,
    pub cache: Vec<Gauge>,
    pub resolver_stats: Vec<Counter>,
    pub resolver_queries: Vec<Counter>,
}

/// ZoneCounter represents a single zone counter value.
pub struct ZoneCounter {
    pub name: String,
    pub serial: u32,
}

/// ZoneView represents statistics for a single BIND zone view.
#[derive(Default)]
pub struct ZoneView {
    pub name: String,
    pub zone_data: Vec<ZoneCounter>,
}

/// ThreadModel contains task and worker information
#[derive(Default, Deserialize)]
pub struct ThreadModel {
    // #[serde(rename = "type")]
    // pub typ: String,
    #[serde(rename = "worker-threads")]
    pub worker_threads: u64,
    // #[serde(rename = "default-quantum")]
    // pub default_quantum: u64,
    #[serde(rename = "tasks-running")]
    pub tasks_running: u64,
}

/// TaskManager contains information about all running tasks.
#[derive(Default, Deserialize)]
pub struct TaskManager {
    #[serde(rename = "thread-model")]
    pub thread_model: ThreadModel,
}

/// Statistics is a generic representation of BIND statistics.
#[derive(Default)]
struct Statistics {
    server: Server,
    views: Vec<View>,
    zone_views: Vec<ZoneView>,
    task_manager: TaskManager,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("http request, {0}")]
    Request(#[from] HttpError),
    #[error("build http request failed, {0}")]
    Http(#[from] http::Error),
    #[error("unexpected status code {0}")]
    UnexpectedStatus(StatusCode),
    #[error("read response body failed, {0}")]
    ReadBody(#[from] hyper::Error),
    #[error("decode response failed, {0}")]
    Decode(String),
}

pub struct Client {
    endpoint: String,
    client: HttpClient,
}

impl Client {
    async fn probe(&self) -> Result<Version, Error> {
        let url = format!("{}/json/v1/status", self.endpoint);
        let req = http::Request::get(url).body(Full::default())?;
        let resp = self.client.send(req).await?;

        if resp.status().is_success() {
            Ok(Version::JsonV1)
        } else {
            Ok(Version::XmlV3)
        }
    }

    async fn fetch<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        let url = format!("{}{}", self.endpoint, path);
        let req = http::Request::get(url).body(Full::default())?;
        let resp = self.client.send(req).await?;
        let (parts, incoming) = resp.into_parts();
        if !parts.status.is_success() {
            return Err(Error::UnexpectedStatus(parts.status));
        }

        let body = incoming.collect().await?.to_bytes();

        match parts.headers.get(CONTENT_TYPE) {
            Some(value) => {
                if value
                    .as_bytes()
                    .windows(4)
                    .any(|w| w == b"application/json")
                {
                    serde_json::from_slice(&body).map_err(|err| Error::Decode(err.to_string()))
                } else {
                    quick_xml::de::from_reader(body.reader())
                        .map_err(|err| Error::Decode(err.to_string()))
                }
            }
            None => serde_json::from_slice(&body).map_err(|err| Error::Decode(err.to_string())),
        }
    }
}

async fn run(
    client: Client,
    interval: Duration,
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
) {
    let mut version = None;
    let start = calculate_start(&client.endpoint, interval);
    let mut ticker = tokio::time::interval_at(start.into(), interval);

    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            _ = ticker.tick() => {}
        }

        let version = match version {
            Some(version) => version,
            None => match client.probe().await {
                Ok(v) => {
                    version = Some(v);
                    v
                }
                Err(err) => {
                    warn!(message = "probe api version failed", endpoint = %client.endpoint, %err);
                    continue;
                }
            },
        };

        let start = Instant::now();
        let result = match version {
            Version::XmlV3 => client.xml_v3().await,
            Version::JsonV1 => client.json_v1().await,
        };
        let elapsed = start.elapsed();

        let success = result.is_ok();

        let mut metrics = vec![
            Metric::gauge("bind_up", "Was the Bind instance query successful", success),
            Metric::gauge(
                "bind_scrape_duration_seconds",
                "Duration of scraping",
                elapsed,
            ),
        ];

        if let Ok(s) = result {
            metrics.extend(statistics_to_metrics(s));
        }

        let now = Utc::now();
        metrics.iter_mut().for_each(|metric| {
            metric.insert_tag("endpoint", client.endpoint.clone());
            metric.timestamp = Some(now);
        });

        if let Err(_err) = output.send(metrics).await {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use framework::config::ProxyConfig;
    use http::{Method, Request, Response};
    use http_body_util::Full;
    use hyper::body::Incoming;
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper_util::rt::TokioIo;
    use testify::http::{file_send, not_found};
    use tokio::net::TcpListener;

    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }

    async fn assert_statistics<H, S>(handle: H)
    where
        H: Fn(Request<Incoming>) -> S + Copy + Send + Sync + 'static,
        S: Future<Output = hyper::Result<Response<Full<Bytes>>>> + Send + 'static,
    {
        let addr = testify::next_addr();
        let listener = TcpListener::bind(addr).await.unwrap();

        tokio::spawn(async move {
            loop {
                let (conn, _peer) = listener.accept().await.unwrap();

                let service = service_fn(handle);

                tokio::spawn(async move {
                    if let Err(err) = http1::Builder::new()
                        .serve_connection(TokioIo::new(conn), service)
                        .await
                    {
                        panic!("handle http connection failed, {err}");
                    }
                });
            }
        });

        // sleep 1s to wait for the http server
        tokio::time::sleep(Duration::from_secs(1)).await;

        let endpoint = format!("http://{addr}");

        let http_client = HttpClient::new(None, &ProxyConfig::default()).unwrap();
        let client = Client {
            endpoint,
            client: http_client,
        };

        let version = client.probe().await.unwrap();
        let s = match version {
            Version::XmlV3 => client.xml_v3().await.unwrap(),
            Version::JsonV1 => client.json_v1().await.unwrap(),
        };
        let got = statistics_to_metrics(s)
            .into_iter()
            .map(|m| m.to_string())
            .flat_map(|s| s.lines().map(|s| s.to_string()).collect::<Vec<_>>())
            .collect::<Vec<_>>();

        for want in [
            // server
            r#"bind_config_time_seconds 1626325868"#,
            r#"bind_response_rcodes_total{rcode="NOERROR"} 989812"#,
            r#"bind_response_rcodes_total{rcode="NXDOMAIN"} 33958"#,
            // view
            r#"bind_resolver_response_errors_total{error="REFUSED",view="_bind"} 17"#,
            r#"bind_resolver_response_errors_total{error="REFUSED",view="_default"} 5798"#,
            // task
            r#"bind_tasks_running 8"#,
            r#"bind_worker_threads 16"#,
        ] {
            assert!(got.contains(&want.to_string()), "want {want}")
        }
    }

    #[tokio::test]
    async fn xml_v3() {
        async fn handle(req: Request<Incoming>) -> hyper::Result<Response<Full<Bytes>>> {
            debug!(
                message = "serve http request",
                path = req.uri().path(),
                handler = "v3"
            );

            if req.method() != Method::GET {
                return Ok(not_found());
            }

            file_send(format!("tests/bind/{}.xml", req.uri().path())).await
        }

        assert_statistics(handle).await;
    }

    #[tokio::test]
    async fn json_v1() {
        async fn handle(req: Request<Incoming>) -> hyper::Result<Response<Full<Bytes>>> {
            debug!(
                message = "serve http request",
                path = req.uri().path(),
                handler = "json"
            );

            if req.method() != Method::GET {
                return Ok(not_found());
            }

            file_send(format!("tests/bind/{}.json", req.uri().path())).await
        }

        assert_statistics(handle).await;
    }
}
