use std::io::BufRead;
use std::time::Duration;

use chrono::Utc;
use configurable::configurable_component;
use event::tags::{Key, Tags};
use event::{Kind as MetricKind, Metric, tags};
use framework::config::{Output, SourceConfig, SourceContext, default_interval};
use framework::http::{Auth, HttpClient};
use framework::tls::TlsConfig;
use framework::{Error, Source};
use http::{StatusCode, Uri};
use http_body_util::{BodyExt, Full};
use thiserror::Error;

// HAProxy 1.4
// # pxname,svname,qcur,qmax,scur,smax,slim,stot,bin,bout,dreq,dresp,ereq,econ,eresp,wretr,wredis,status,weight,act,bck,chkfail,chkdown,lastchg,downtime,qlimit,pid,iid,sid,throttle,lbtot,tracked,type,rate,rate_lim,rate_max,check_status,check_code,check_duration,hrsp_1xx,hrsp_2xx,hrsp_3xx,hrsp_4xx,hrsp_5xx,hrsp_other,hanafail,req_rate,req_rate_max,req_tot,cli_abrt,srv_abrt,
// HAProxy 1.5
// pxname,svname,qcur,qmax,scur,smax,slim,stot,bin,bout,dreq,dresp,ereq,econ,eresp,wretr,wredis,status,weight,act,bck,chkfail,chkdown,lastchg,downtime,qlimit,pid,iid,sid,throttle,lbtot,tracked,type,rate,rate_lim,rate_max,check_status,check_code,check_duration,hrsp_1xx,hrsp_2xx,hrsp_3xx,hrsp_4xx,hrsp_5xx,hrsp_other,hanafail,req_rate,req_rate_max,req_tot,cli_abrt,srv_abrt,comp_in,comp_out,comp_byp,comp_rsp,lastsess,
// HAProxy 1.5.19
// pxname,svname,qcur,qmax,scur,smax,slim,stot,bin,bout,dreq,dresp,ereq,econ,eresp,wretr,wredis,status,weight,act,bck,chkfail,chkdown,lastchg,downtime,qlimit,pid,iid,sid,throttle,lbtot,tracked,type,rate,rate_lim,rate_max,check_status,check_code,check_duration,hrsp_1xx,hrsp_2xx,hrsp_3xx,hrsp_4xx,hrsp_5xx,hrsp_other,hanafail,req_rate,req_rate_max,req_tot,cli_abrt,srv_abrt,comp_in,comp_out,comp_byp,comp_rsp,lastsess,last_chk,last_agt,qtime,ctime,rtime,ttime,
// HAProxy 1.7
// pxname,svname,qcur,qmax,scur,smax,slim,stot,bin,bout,dreq,dresp,ereq,econ,eresp,wretr,wredis,status,weight,act,bck,chkfail,chkdown,lastchg,downtime,qlimit,pid,iid,sid,throttle,lbtot,tracked,type,rate,rate_lim,rate_max,check_status,check_code,check_duration,hrsp_1xx,hrsp_2xx,hrsp_3xx,hrsp_4xx,hrsp_5xx,hrsp_other,hanafail,req_rate,req_rate_max,req_tot,cli_abrt,srv_abrt,comp_in,comp_out,comp_byp,comp_rsp,lastsess,last_chk,last_agt,qtime,ctime,rtime,ttime,agent_status,agent_code,agent_duration,check_desc,agent_desc,check_rise,check_fall,check_health,agent_rise,agent_fall,agent_health,addr,cookie,mode,algo,conn_rate,conn_rate_max,conn_tot,intercepted,dcon,dses
const MINIMUM_CSV_FIELD_COUNT: usize = 33;

const PXNAME_FIELD: usize = 0;
const SVNAME_FIELD: usize = 1;
const STATUS_FIELD: usize = 17;
const TYPE_FIELD: usize = 32;
const CHECK_DURATION_FIELD: usize = 38;
const QTIME_MS_FIELD: usize = 58;
const CTIME_MS_FIELD: usize = 59;
const RTIME_MS_FIELD: usize = 60;
const TTIME_MS_FIELD: usize = 61;

const BACKEND_KEY: Key = Key::from_static("backend");
const FRONTEND_KEY: Key = Key::from_static("frontend");
const INSTANCE_KEY: Key = Key::from_static("instance");
const SERVER_KEY: Key = Key::from_static("server");

/// This source scrapes HAProxy stats.
///
/// As of 2.0.0, HAProxy includes a Prometheus exporter module that can be
/// built into HAProxy binary during build time.
#[configurable_component(source, name = "haproxy")]
#[serde(deny_unknown_fields)]
struct Config {
    /// HTTP/HTTPS endpoint to Consul server.
    #[configurable(required)]
    endpoints: Vec<String>,

    /// Duration between each scrape.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    /// Configures the TLS options for outgoing connections.
    #[serde(default)]
    tls: Option<TlsConfig>,

    /// Configures the authentication strategy.
    #[serde(default)]
    auth: Option<Auth>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "haproxy")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let SourceContext {
            proxy,
            mut output,
            mut shutdown,
            ..
        } = cx;
        let endpoints = self
            .endpoints
            .iter()
            .flat_map(|f| f.parse::<Uri>())
            .collect::<Vec<_>>();

        let auth = self.auth.clone();
        let client = HttpClient::new(self.tls.as_ref(), &proxy)?;
        let mut ticker = tokio::time::interval(self.interval);

        Ok(Box::pin(async move {
            loop {
                tokio::select! {
                    biased;

                    _ = &mut shutdown => break,
                    _ = ticker.tick() => {}
                }

                let metrics = futures::future::join_all(
                    endpoints.iter().map(|uri| gather(&client, uri, &auth)),
                )
                .await;

                let now = Utc::now();
                let metrics = metrics
                    .into_iter()
                    .flatten()
                    .map(|mut m| {
                        m.timestamp = Some(now);
                        m
                    })
                    .collect::<Vec<_>>();

                if let Err(err) = output.send(metrics).await {
                    error!(
                        message = "Error sending haproxy metrics",
                        %err
                    );

                    return Err(());
                }
            }

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

async fn scrap(client: &HttpClient, uri: &Uri, auth: Option<Auth>) -> Result<Vec<Metric>, Error> {
    let mut req = http::Request::get(uri).body(Full::default())?;

    if let Some(auth) = &auth {
        auth.apply(&mut req);
    }

    let resp = client.send(req).await?;
    let (parts, incoming) = resp.into_parts();

    let metrics = match parts.status {
        StatusCode::OK => {
            let body = incoming.collect().await?.to_bytes();

            match parse_csv(&body) {
                Ok(metrics) => metrics,
                Err(err) => {
                    warn!(
                        message = "Parse haproxy response csv failed",
                        %err,
                        internal_log_rate_limit = true
                    );
                    vec![]
                }
            }
        }
        status => {
            warn!(
                message = "Fetch haproxy stats failed",
                code = status.as_str(),
            );

            vec![]
        }
    };

    Ok(metrics)
}

async fn gather(client: &HttpClient, uri: &Uri, auth: &Option<Auth>) -> Vec<Metric> {
    let start = std::time::Instant::now();
    let mut metrics = match scrap(client, uri, auth.clone()).await {
        Ok(ms) => ms,
        Err(err) => {
            warn!(
                message = "Scraping metrics failed",
                %err
            );

            vec![]
        }
    };
    let elapsed = start.elapsed();
    let up = i32::from(!metrics.is_empty());
    let instance = format!("{}:{}", uri.host().unwrap(), uri.port_u16().unwrap());
    metrics.extend([
        Metric::gauge(
            "haproxy_up",
            "Was the last scrape of HAProxy successful.",
            up,
        ),
        Metric::gauge("haproxy_scrape_duration_seconds", "", elapsed),
    ]);

    metrics.iter_mut().for_each(|m| {
        m.insert_tag(INSTANCE_KEY, &instance);
    });

    metrics
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("row is too short")]
    RowTooShort,

    #[error("unknown type of metrics, type: {0}")]
    UnknownTypeOfMetrics(String),
}

pub fn parse_csv(input: &[u8]) -> Result<Vec<Metric>, ParseError> {
    let mut metrics = vec![];
    let mut lines = input.lines();

    while let Some(Ok(line)) = lines.next() {
        let parts = line.split(',').collect::<Vec<_>>();
        if parts.len() < MINIMUM_CSV_FIELD_COUNT {
            return Err(ParseError::RowTooShort);
        }

        let pxname = parts[PXNAME_FIELD];
        let svname = parts[SVNAME_FIELD];

        let partial = match parts[TYPE_FIELD] {
            "0" => parse_row(
                parts,
                &FRONTEND_METRIC_INFOS,
                tags!(FRONTEND_KEY => pxname.to_string()),
            ),
            "1" => parse_row(
                parts,
                &BACKEND_METRIC_INFOS,
                tags!(BACKEND_KEY => pxname.to_string()),
            ),
            "2" => parse_row(
                parts,
                &SERVER_METRIC_INFOS,
                tags!(BACKEND_KEY => pxname.to_string(), SERVER_KEY => svname.to_string()),
            ),
            _ => continue,
        };

        metrics.extend(partial);
    }

    Ok(metrics)
}

// Available on unix only, see
// https://github.com/prometheus/haproxy_exporter/blob/master/haproxy_exporter.go#L267
//
// fn parse_info(reader: impl std::io::BufRead) -> Result<(String, String), Error> {
//     let lines = reader.lines();
//     let mut release_date = String::new();
//     let mut version = String::new();
//
//     for line in lines {
//         let line = match line {
//             Ok(line) => line,
//             Err(_) => continue,
//         };
//
//         match line.split_once(": ") {
//             Some((k, v)) => {
//                 if k == "Release_date" {
//                     release_date = v.to_string();
//                 } else if k == "Version" {
//                     version = v.to_string();
//                 }
//             }
//             _ => continue,
//         }
//     }
//
//     Ok((release_date, version))
// }

struct MetricInfo {
    index: usize,

    name: &'static str,
    desc: &'static str,
    kind: MetricKind,
    tags: &'static [(&'static str, &'static str)],
}

impl MetricInfo {
    fn build(&self, value: f64, mut tags: Tags) -> Metric {
        for (key, value) in self.tags {
            tags.insert(*key, *value);
        }

        match self.kind {
            MetricKind::Sum => Metric::sum_with_tags(self.name, self.desc, value, tags),
            MetricKind::Gauge => Metric::gauge_with_tags(self.name, self.desc, value, tags),
            _ => unreachable!(),
        }
    }
}

const SERVER_METRIC_INFOS: [MetricInfo; 32] = [
    MetricInfo {
        index: 2,
        name: "haproxy_server_current_queue",
        desc: "Current number of queued requests assigned to this server.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 3,
        name: "haproxy_server_max_queue",
        desc: "Maximum observed number of queued requests assigned to this server.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 4,
        name: "haproxy_server_current_sessions",
        desc: "Current number of active sessions.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 5,
        name: "haproxy_server_max_sessions",
        desc: "Maximum observed number of active sessions.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 6,
        name: "haproxy_server_limit_sessions",
        desc: "Configured session limit.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 7,
        name: "haproxy_server_sessions_total",
        desc: "Total number of sessions.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 8,
        name: "haproxy_server_bytes_in_total",
        desc: "Current total of incoming bytes.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 9,
        name: "haproxy_server_bytes_out_total",
        desc: "Current total of outgoing bytes.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 13,
        name: "haproxy_server_connection_errors_total",
        desc: "Total of connection errors.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 14,
        name: "haproxy_server_response_errors_total",
        desc: "Total of response errors.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 15,
        name: "haproxy_server_retry_warnings_total",
        desc: "Total of retry warnings.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 16,
        name: "haproxy_server_redispatch_warnings_total",
        desc: "Total of redispatch warnings.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 17,
        name: "haproxy_server_up",
        desc: "Current health status of the server (1 = UP, 0 = DOWN).",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 18,
        name: "haproxy_server_weight",
        desc: "Current weight of the server.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 21,
        name: "haproxy_server_check_failures_total",
        desc: "Total number of failed health checks.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 24,
        name: "haproxy_server_downtime_seconds_total",
        desc: "Total downtime in seconds.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 30,
        name: "haproxy_server_server_selected_total",
        desc: "Total number of times a server was selected, either for new sessions, or when re-dispatching.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 33,
        name: "haproxy_server_current_session_rate",
        desc: "Current number of sessions per second over last elapsed second.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 35,
        name: "haproxy_server_max_session_rate",
        desc: "Maximum observed number of sessions per second.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 38,
        name: "haproxy_server_check_duration_seconds",
        desc: "Previously run health check duration, in seconds",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 39,
        name: "haproxy_server_http_responses_total",
        desc: "Total of HTTP responses.",
        kind: MetricKind::Sum,
        tags: &[("code", "1xx")],
    },
    MetricInfo {
        index: 40,
        name: "haproxy_server_http_responses_total",
        desc: "Total of HTTP responses.",
        kind: MetricKind::Sum,
        tags: &[("code", "2xx")],
    },
    MetricInfo {
        index: 41,
        name: "haproxy_server_http_responses_total",
        desc: "Total of HTTP responses.",
        kind: MetricKind::Sum,
        tags: &[("code", "3xx")],
    },
    MetricInfo {
        index: 42,
        name: "haproxy_server_http_responses_total",
        desc: "Total of HTTP responses.",
        kind: MetricKind::Sum,
        tags: &[("code", "4xx")],
    },
    MetricInfo {
        index: 43,
        name: "haproxy_server_http_responses_total",
        desc: "Total of HTTP responses.",
        kind: MetricKind::Sum,
        tags: &[("code", "5xx")],
    },
    MetricInfo {
        index: 44,
        name: "haproxy_server_http_responses_total",
        desc: "Total of HTTP responses.",
        kind: MetricKind::Sum,
        tags: &[("code", "other")],
    },
    MetricInfo {
        index: 49,
        name: "haproxy_server_client_aborts_total",
        desc: "Total number of data transfers aborted by the client.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 50,
        name: "haproxy_server_server_aborts_total",
        desc: "Total number of data transfers aborted by the server.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 58,
        name: "haproxy_server_http_queue_time_average_seconds",
        desc: "Avg. HTTP queue time for last 1024 successful connections.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 59,
        name: "haproxy_server_http_connect_time_average_seconds",
        desc: "Avg. HTTP connect time for last 1024 successful connections.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 60,
        name: "haproxy_server_http_response_time_average_seconds",
        desc: "Avg. HTTP response time for last 1024 successful connections.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 61,
        name: "haproxy_server_http_total_time_average_seconds",
        desc: "Avg. HTTP total time for last 1024 successful connections.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
];

const BACKEND_METRIC_INFOS: [MetricInfo; 34] = [
    MetricInfo {
        index: 2,
        name: "haproxy_backend_current_queue",
        desc: "Current number of queued requests not assigned to any server.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 3,
        name: "haproxy_backend_max_queue",
        desc: "Maximum observed number of queued requests not assigned to any server.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 4,
        name: "haproxy_backend_current_sessions",
        desc: "Current number of active sessions.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 5,
        name: "haproxy_backend_max_sessions",
        desc: "Maximum observed number of active sessions.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 6,
        name: "haproxy_backend_limit_sessions",
        desc: "Configured session limit.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 7,
        name: "haproxy_backend_sessions_total",
        desc: "Total number of sessions.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 8,
        name: "haproxy_backend_bytes_in_total",
        desc: "Current total of incoming bytes.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 9,
        name: "haproxy_backend_bytes_out_total",
        desc: "Current total of outgoing bytes.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 13,
        name: "haproxy_backend_connection_errors_total",
        desc: "Total of connection errors.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 14,
        name: "haproxy_backend_response_errors_total",
        desc: "Total of response errors.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 15,
        name: "haproxy_backend_retry_warnings_total",
        desc: "Total of retry warnings.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 16,
        name: "haproxy_backend_redispatch_warnings_total",
        desc: "Total of redispatch warnings.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 17,
        name: "haproxy_backend_up",
        desc: "Current health status of the backend (1 = UP, 0 = DOWN).",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 18,
        name: "haproxy_backend_weight",
        desc: "Total weight of the servers in the backend.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 19,
        name: "haproxy_backend_current_server",
        desc: "Current number of active servers",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 30,
        name: "haproxy_backend_server_selected_total",
        desc: "Total number of times a server was selected, either for new sessions, or when re-dispatching.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 33,
        name: "haproxy_backend_current_session_rate",
        desc: "Current number of sessions per second over last elapsed second.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 35,
        name: "haproxy_backend_max_session_rate",
        desc: "Maximum number of sessions per second.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 39,
        name: "haproxy_backend_http_responses_total",
        desc: "Total of HTTP responses.",
        kind: MetricKind::Sum,
        tags: &[("code", "1xx")],
    },
    MetricInfo {
        index: 40,
        name: "haproxy_backend_http_responses_total",
        desc: "Total of HTTP responses.",
        kind: MetricKind::Sum,
        tags: &[("code", "2xx")],
    },
    MetricInfo {
        index: 41,
        name: "haproxy_backend_http_responses_total",
        desc: "Total of HTTP responses.",
        kind: MetricKind::Sum,
        tags: &[("code", "3xx")],
    },
    MetricInfo {
        index: 42,
        name: "haproxy_backend_http_responses_total",
        desc: "Total of HTTP responses.",
        kind: MetricKind::Sum,
        tags: &[("code", "4xx")],
    },
    MetricInfo {
        index: 43,
        name: "haproxy_backend_http_responses_total",
        desc: "Total of HTTP responses.",
        kind: MetricKind::Sum,
        tags: &[("code", "5xx")],
    },
    MetricInfo {
        index: 44,
        name: "haproxy_backend_http_responses_total",
        desc: "Total of HTTP responses.",
        kind: MetricKind::Sum,
        tags: &[("code", "other")],
    },
    MetricInfo {
        index: 49,
        name: "haproxy_backend_client_aborts_total",
        desc: "Total number of data transfers aborted by the client.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 50,
        name: "haproxy_backend_server_aborts_total",
        desc: "Total number of data transfers aborted by the server.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 51,
        name: "haproxy_backend_compressor_bytes_in_total",
        desc: "Number of HTTP response bytes fed to the compressor",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 52,
        name: "haproxy_backend_compressor_bytes_out_total",
        desc: "Number of HTTP response bytes emitted by the compressor",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 53,
        name: "haproxy_backend_compressor_bytes_bypassed_total",
        desc: "Number of bytes that bypassed the HTTP compressor",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 54,
        name: "haproxy_backend_http_responses_compressed_total",
        desc: "Number of HTTP responses that were compressed",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 58,
        name: "haproxy_backend_http_queue_time_average_seconds",
        desc: "Avg. HTTP queue time for last 1024 successful connections.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 59,
        name: "haproxy_backend_http_connect_time_average_seconds",
        desc: "Avg. HTTP connect time for last 1024 successful connections.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 60,
        name: "haproxy_backend_http_response_time_average_seconds",
        desc: "Avg. HTTP response time for last 1024 successful connections.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 61,
        name: "haproxy_backend_http_total_time_average_seconds",
        desc: "Avg. HTTP total time for last 1024 successful connections.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
];

const FRONTEND_METRIC_INFOS: [MetricInfo; 23] = [
    MetricInfo {
        index: 4,
        name: "haproxy_frontend_current_sessions",
        desc: "Current number of active sessions.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 5,
        name: "haproxy_frontend_max_sessions",
        desc: "Maximum observed number of active sessions.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 6,
        name: "haproxy_frontend_limit_sessions",
        desc: "Configured session limit.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 7,
        name: "haproxy_frontend_sessions_total",
        desc: "Total number of sessions.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 8,
        name: "haproxy_frontend_bytes_in_total",
        desc: "Current total of incoming bytes.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 9,
        name: "haproxy_frontend_bytes_out_total",
        desc: "Current total of outgoing bytes.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 10,
        name: "haproxy_frontend_requests_denied_total",
        desc: "Total of requests denied for security.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 12,
        name: "haproxy_frontend_request_errors_total",
        desc: "Total of request errors.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 33,
        name: "haproxy_frontend_current_session_rate",
        desc: "Current number of sessions per second over last elapsed second.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 34,
        name: "haproxy_frontend_limit_session_rate",
        desc: "Configured limit on new sessions per second.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 35,
        name: "haproxy_frontend_max_session_rate",
        desc: "Maximum observed number of sessions per second.",
        kind: MetricKind::Gauge,
        tags: &[],
    },
    MetricInfo {
        index: 39,
        name: "haproxy_frontend_http_responses_total",
        desc: "Total of HTTP responses.",
        kind: MetricKind::Sum,
        tags: &[("code", "1xx")],
    },
    MetricInfo {
        index: 40,
        name: "haproxy_frontend_http_responses_total",
        desc: "Total of HTTP responses.",
        kind: MetricKind::Sum,
        tags: &[("code", "2xx")],
    },
    MetricInfo {
        index: 41,
        name: "haproxy_frontend_http_responses_total",
        desc: "Total of HTTP responses.",
        kind: MetricKind::Sum,
        tags: &[("code", "3xx")],
    },
    MetricInfo {
        index: 42,
        name: "haproxy_frontend_http_responses_total",
        desc: "Total of HTTP responses.",
        kind: MetricKind::Sum,
        tags: &[("code", "4xx")],
    },
    MetricInfo {
        index: 43,
        name: "haproxy_frontend_http_responses_total",
        desc: "Total of HTTP responses.",
        kind: MetricKind::Sum,
        tags: &[("code", "5xx")],
    },
    MetricInfo {
        index: 44,
        name: "haproxy_frontend_http_responses_total",
        desc: "Total of HTTP responses.",
        kind: MetricKind::Sum,
        tags: &[("code", "other")],
    },
    MetricInfo {
        index: 48,
        name: "haproxy_frontend_http_requests_total",
        desc: "Total HTTP requests.",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 51,
        name: "haproxy_frontend_compressor_bytes_in_total",
        desc: "Number of HTTP response bytes fed to the compressor",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 52,
        name: "haproxy_frontend_compressor_bytes_out_total",
        desc: "Number of HTTP response bytes emitted by the compressor",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 53,
        name: "haproxy_frontend_compressor_bytes_bypassed_total",
        desc: "Number of bytes that bypassed the HTTP compressor",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 54,
        name: "haproxy_frontend_http_responses_compressed_total",
        desc: "Number of HTTP responses that were compressed",
        kind: MetricKind::Sum,
        tags: &[],
    },
    MetricInfo {
        index: 79,
        name: "haproxy_frontend_connections_total",
        desc: "Total number of connections",
        kind: MetricKind::Sum,
        tags: &[],
    },
];

fn parse_row(row: Vec<&str>, infos: &[MetricInfo], tags: Tags) -> Vec<Metric> {
    let mut metrics = Vec::with_capacity(infos.len());

    for info in infos {
        let field = match row.get(info.index) {
            Some(field) => {
                if field.is_empty() {
                    continue;
                }

                *field
            }
            None => break,
        };

        let value = match info.index {
            STATUS_FIELD => parse_status_field(field),
            CHECK_DURATION_FIELD | QTIME_MS_FIELD | CTIME_MS_FIELD | RTIME_MS_FIELD
            | TTIME_MS_FIELD => match field.parse::<f64>() {
                Ok(value) => value / 1000.0,
                Err(err) => {
                    warn!(
                        message = "parse csv field to f64 failed",
                        field,
                        %err,
                        internal_log_rate_limit = true,
                    );
                    continue;
                }
            },
            _ => match field.parse::<i64>() {
                Ok(value) => value as f64,
                Err(err) => {
                    warn!(
                        message = "parse csv field to i64 failed",
                        field,
                        %err,
                        internal_log_rate_limit = true,
                    );
                    continue;
                }
            },
        };

        metrics.push(info.build(value, tags.clone()));
    }

    metrics
}

#[inline]
fn parse_status_field(value: &str) -> f64 {
    match value {
        "UP" | "UP 1/3" | "UP 2/3" | "OPEN" | "no check" | "DRAIN" => 1.0,
        "DOWN" | "DOWN 1/2" | "NOLB" | "MAINT" | "MAINT(via)" | "MAINT(resolution)" => 0.0,
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }

    // Available on unix only, see
    // https://github.com/prometheus/haproxy_exporter/blob/master/haproxy_exporter.go#L267
    //
    // #[test]
    // fn test_parse_info() {
    //     let input = "Release_date: test date\nVersion: test version\n";
    //     let reader = BufReader::new(io::Cursor::new(input));
    //     let (release, version) = parse_info(reader).unwrap();
    //     assert_eq!(release, "test date");
    //     assert_eq!(version, "test version");
    // }

    #[test]
    fn parse() {
        let content = include_bytes!("../../tests/haproxy/stats.csv");
        let metrics = parse_csv(content).unwrap();

        assert!(!metrics.is_empty());
    }

    #[test]
    fn status_field() {
        let tests = [
            ("UP", 1),
            ("UP 1/3", 1),
            ("UP 2/3", 1),
            ("OPEN", 1),
            ("no check", 1),
            ("DOWN", 0),
            ("DOWN 1/2", 0),
            ("NOLB", 0),
            ("MAINT", 0), // prometheus/haproxy_exporter#35
            ("unknown", 0),
            ("", 0),
        ];

        for (input, want) in tests {
            assert_eq!(parse_status_field(input), want as f64);
        }
    }
}

#[cfg(all(test, feature = "haproxy-integration-tests"))]
mod integration_tests {
    use super::*;
    use crate::testing::trace_init;
    use framework::config::ProxyConfig;
    use testify::container::Container;
    use testify::next_addr;

    #[tokio::test]
    async fn test_gather() {
        trace_init();

        let pwd = std::env::current_dir().unwrap();
        let service_addr = next_addr();

        Container::new("haproxy", "2.4.7")
            .with_tcp(8404, service_addr.port())
            .with_volume(
                format!("{}/tests/haproxy/haproxy.cfg", pwd.to_string_lossy()),
                "/usr/local/etc/haproxy/haproxy.cfg",
            )
            .tail_logs(false, true)
            .run(async move {
                // test unhealth gather
                let uncorrect_port = 111; // dummy, but ok
                let uri = format!("http://127.0.0.1:{}/stats?stats;csv", uncorrect_port)
                    .parse()
                    .unwrap();
                let client = HttpClient::new(None, &ProxyConfig::default()).unwrap();
                let metrics = gather(&client, &uri, &None).await;
                assert_eq!(metrics.len(), 2);

                // test health gather
                let uri = format!("http://{}/stats?stats;csv", service_addr)
                    .parse()
                    .unwrap();
                let client = HttpClient::new(None, &ProxyConfig::default()).unwrap();
                let metrics = gather(&client, &uri, &None).await;
                assert_ne!(metrics.len(), 2);
            })
            .await;
    }
}
