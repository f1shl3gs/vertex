mod client;
#[cfg(test)]
mod tests;

use std::num::ParseFloatError;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::Utc;
use configurable::configurable_component;
use event::tags::{Key, Value};
use event::{tags, Bucket, Metric};
use framework::config::{default_interval, DataType, Output, SourceConfig, SourceContext};
use framework::http::HttpClient;
use framework::Source;

/// Make sure BIND was built with libxml2 support. You can check with the following command:
///    named -V | grep libxml2
///
/// Configure BIND to open a statistics channel. e.g.
///
/// statistics-channels {
///   inet 127.0.0.1 port 8053 allow { 127.0.0.1; };
/// };
#[configurable_component(source, name = "bind")]
#[derive(Debug)]
#[serde(deny_unknown_fields)]
struct Config {
    /// Endpoint for the BIND statistics api
    #[configurable(required, format = "uri", example = "http://127.0.0.1:8053")]
    endpoint: String,

    /// Duration between each scrape.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,
}

#[async_trait]
#[typetag::serde(name = "bind")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        let mut interval = tokio::time::interval(self.interval);
        let mut shutdown = cx.shutdown;
        let mut output = cx.output;
        let http_client = HttpClient::new(&None, &cx.proxy)?;
        let client = client::Client::new(self.endpoint.clone(), http_client);
        let endpoint = self.endpoint.clone();

        Ok(Box::pin(async move {
            let endpoint_key = Key::from("endpoint");
            let endpoint_value = Value::from(endpoint);

            loop {
                tokio::select! {
                    _ = &mut shutdown => break,
                    _ = interval.tick() => {
                        let start = Instant::now();
                        let result = client.stats().await;
                        let elapsed = start.elapsed().as_secs_f64();
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
                            metric.insert_tag(endpoint_key.clone(), endpoint_value.clone());
                            metric.timestamp = Some(now);
                        });

                        if let Err(err) = output.send(metrics).await {
                            error!(message = "Error sending metrics", ?err);
                            return Err(());
                        }
                    }
                }
            }

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }
}

fn statistics_to_metrics(s: client::Statistics) -> Vec<Metric> {
    let mut metrics = server_metrics(s.server);
    metrics.extend(task_metrics(s.task_manager));
    metrics.extend(view_metrics(s.views, s.zone_views));
    metrics
}

fn server_metric(c: client::Counter) -> Option<Metric> {
    match c.name.as_str() {
        "QryDuplicate" => Some(Metric::sum(
            "bind_query_duplicates_total",
            "Number of duplicated queries received",
            c.counter as f64,
        )),
        "QryRecursion" => Some(Metric::sum(
            "bind_query_recursions_total",
            "Number of queries causing recursion",
            c.counter as f64,
        )),
        "XfrRej" => Some(Metric::sum(
            "bind_zone_transfer_rejected_total",
            "Number of rejected zone transfers",
            c.counter as f64,
        )),
        "XfrSuccess" => Some(Metric::sum(
            "bind_zone_transfer_success_total",
            "Number of successful zone transfers",
            c.counter as f64,
        )),
        "XfrFail" => Some(Metric::sum(
            "bind_zone_transfer_failure_total",
            "Number of failed zone transfers",
            c.counter as f64,
        )),
        "RecursClients" => Some(Metric::sum(
            "bind_recursive_clients",
            "Number of current recursive clients",
            c.counter as f64,
        )),
        _ => None,
    }
}

fn server_metrics(s: client::Server) -> Vec<Metric> {
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
            c.counter,
            tags!(
                "type" => c.name.clone(),
            ),
        ));
    }

    for s in s.incoming_requests {
        metrics.push(Metric::sum_with_tags(
            "bind_incoming_requests_total",
            "Number of incoming DNS requests",
            s.counter,
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
                    c.counter,
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
                    c.counter,
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
            c.counter,
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
fn histogram(stats: &Vec<client::Counter>) -> Result<(Vec<Bucket>, u64), ParseFloatError> {
    let mut count = 0;
    let mut buckets = vec![];

    for c in stats {
        if c.name.starts_with("QryRTT") {
            let mut b = f64::INFINITY;

            if !c.name.ends_with('+') {
                if let Some(rtt) = c.name.strip_prefix("QryRTT") {
                    b = rtt.parse()?;
                }
            }

            count += c.counter;
            buckets.push(Bucket {
                upper: b / 1000.0,
                count,
            });
        }
    }

    buckets.sort_by(|a, b| a.upper.total_cmp(&b.upper));

    Ok((buckets, count))
}

fn view_metrics(views: Vec<client::View>, zone_views: Vec<client::ZoneView>) -> Vec<Metric> {
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
                c.counter,
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
                    ?err,
                    internal_log_rate_limit = true
                );
            }
        }

        for c in view.resolver_stats {
            match c.name.as_str() {
                "Lame" => metrics.push(Metric::sum_with_tags(
                    "bind_resolver_response_lame_total",
                    "Number of lame delegation responses received",
                    c.counter,
                    tags!(
                        "view" => view.name.clone(),
                    ),
                )),
                "EDNS0Fail" => metrics.push(Metric::sum_with_tags(
                    "bind_resolver_query_edns0_errors_total",
                    "Number of EDNS(0) query errors",
                    c.counter,
                    tags!(
                        "view" => view.name.clone(),
                    ),
                )),
                "Mismatch" => metrics.push(Metric::sum_with_tags(
                    "bind_resolver_response_mismatch_total",
                    "Number of mismatch responses received",
                    c.counter,
                    tags!(
                        "view" => view.name.clone()
                    ),
                )),
                "Retry" => metrics.push(Metric::sum_with_tags(
                    "bind_resolver_query_retries_total",
                    "Number of resolver query retries",
                    c.counter,
                    tags!(
                        "view" => view.name.clone(),
                    ),
                )),
                "Truncated" => metrics.push(Metric::sum_with_tags(
                    "bind_resolver_response_truncated_total",
                    "Number of truncated responses received",
                    c.counter,
                    tags!(
                        "view" => view.name.clone(),
                    ),
                )),
                "ValFail" => metrics.push(Metric::sum_with_tags(
                    "bind_resolver_dnssec_validation_errors_total",
                    "Number of DNSSEC validation attempt errors",
                    c.counter,
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
                        c.counter,
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
                        c.counter,
                        tags!(
                            "view" => view.name.clone(),
                            "error" => c.name,
                        ),
                    ))
                }
                "ValOk" | "ValNegOk" => metrics.push(Metric::sum_with_tags(
                    "bind_resolver_dnssec_validation_success_total",
                    "Number of DNSSEC validation attempts succeeded",
                    c.counter,
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
            if let Ok(v) = zone.serial.parse::<u64>() {
                metrics.push(Metric::sum_with_tags(
                    "bind_zone_serial",
                    "Zone serial number",
                    v,
                    tags!(
                        "view" => view.name.clone(),
                        "zone_name" => zone.name,
                    ),
                ))
            }
        }
    }

    metrics
}

fn task_metrics(s: client::TaskManager) -> Vec<Metric> {
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
