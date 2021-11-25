use std::collections::BTreeMap;
use std::io::BufRead;

use snafu::Snafu;
use futures::TryFutureExt;
use http::{StatusCode, Uri};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use event::{Metric, tags};

use crate::http::{Auth, HttpClient};
use crate::sources::Source;
use crate::tls::{MaybeTlsSettings, TlsConfig};
use crate::Error;
use crate::config::{
    deserialize_duration, serialize_duration, default_interval, SourceConfig,
    SourceContext, DataType, SourceDescription, GenerateConfig, ProxyConfig,
};


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

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct HaproxyConfig {
    #[serde(default = "default_interval")]
    #[serde(deserialize_with = "deserialize_duration", serialize_with = "serialize_duration")]
    interval: chrono::Duration,

    endpoints: Vec<String>,

    #[serde(default)]
    tls: Option<TlsConfig>,

    #[serde(default)]
    auth: Option<Auth>,
}

impl GenerateConfig for HaproxyConfig {
    fn generate_config() -> Value {
        serde_yaml::to_value(Self {
            interval: default_interval(),
            endpoints: vec![
                "http://127.0.0.1:1111/metrics".to_string(),
                "http://127.0.0.1:2222/metrics".to_string(),
            ],
            tls: None,
            auth: None,
        }).unwrap()
    }
}

inventory::submit! {
    SourceDescription::new::<HaproxyConfig>("haproxy")
}

#[async_trait::async_trait]
#[typetag::serde(name = "haproxy")]
impl SourceConfig for HaproxyConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        todo!()
    }

    fn output_type(&self) -> DataType {
        DataType::Metric
    }

    fn source_type(&self) -> &'static str {
        "haproxy"
    }
}

async fn gather(
    endpoints: Vec<String>,
    tls: Option<TlsConfig>,
    auth: Option<Auth>,
    proxy: &ProxyConfig,
) -> Result<(), Error> {
    /*    let tls = MaybeTlsSettings::from_config(&tls, false)?;
        let client = HttpClient::new(tls, &proxy)?;

        let mut tasks = vec![];
        for endpoint in endpoints.iter() {
            let uri = endpoint.parse()?;
            let mut req = http::Request::get(uri)
                .body(hyper::Body::empty())?;

            if let Some(auth) = &auth {
                auth.apply(&mut req);
            }

            tokio::spawn(async move {
                let resp = client.send(req).await?;
                let (parts, body) = resp.into_parts();
                match parts.status {
                    StatusCode::OK => {

                    },
                    status => {

                    }
                }
            })
        }

    */
    todo!()
}

#[derive(Debug, Snafu)]
enum ParseError {
    #[snafu(display("row is too short"))]
    RowTooShort,

    #[snafu(display("unknown type of metrics, type: {}", typ))]
    UnknownTypeOfMetrics { typ: String },
}

fn parse_csv(reader: impl BufRead) -> Result<Vec<Metric>, ParseError> {
    let mut metrics = vec![];
    let mut lines = reader.lines();

    while let Some(line) = lines.next() {
        let line = match line {
            Ok(line) => line,
            _ => continue
        };

        let parts = line.split(",")
            .collect::<Vec<_>>();
        if parts.len() < MINIMUM_CSV_FIELD_COUNT {
            return Err(ParseError::RowTooShort);
        }

        let pxname = parts[PXNAME_FIELD];
        let svname = parts[SVNAME_FIELD];
        let status = parts[STATUS_FIELD];
        let typ = parts[TYPE_FIELD];

        let partial = match typ {
            "0" => {
                // frontend
                parse_frontend(&line, pxname)
            }
            "1" => {
                // backend
                parse_backend(&line, pxname)
            }
            "2" => {
                // server
                parse_server(&line, pxname, svname)
            }
            _ => {
                return Err(ParseError::UnknownTypeOfMetrics { typ: typ.into() });
            }
        };

        metrics.extend(partial);
    }

    Ok(metrics)
}

fn parse_line(line: &str, list: Vec<(usize, &'static str, &'static str, &'static str, BTreeMap<String, String>)>) -> Result<Vec<Metric>, ParseError> {
    let mut metrics = vec![];
    let row = line.split(",")
        .map(|s| s.trim())
        .collect::<Vec<_>>();

    for (index, name, desc, typ, tags) in list.iter() {
        if *index > row.len() - 1 {
            continue;
        }

        let text = row[*index];
        let value = match *index {
            STATUS_FIELD => parse_status_field(text),
            CHECK_DURATION_FIELD | QTIME_MS_FIELD | CTIME_MS_FIELD | RTIME_MS_FIELD | TTIME_MS_FIELD => {
                let value = match text.parse::<f64>() {
                    Ok(v) => v,
                    Err(err) => {
                        warn!(
                            message = "Can't parse CSV field value",
                            value = text,
                            ?err
                        );

                        continue;
                    }
                };

                value / 1000.0
            }
            _ => match text.parse() {
                Ok(v) => v,
                Err(err) => {
                    warn!(
                        message = "Can't parse CSV failed value",
                        value = text,
                        ?err
                    );

                    continue;
                }
            }
        };

        match *typ {
            "gauge" => metrics.push(Metric::gauge_with_tags(
                *name,
                *desc,
                value,
                tags.clone(),
            )),
            "counter" => metrics.push(Metric::sum_with_tags(
                *name,
                *desc,
                value,
                tags.clone(),
            )),
            _ => unreachable!()
        }
    }

    Ok(metrics)
}

fn parse_info(reader: impl std::io::BufRead) -> Result<(String, String), Error> {
    let mut lines = reader.lines();
    let mut release_date = String::new();
    let mut version = String::new();

    while let Some(line) = lines.next() {
        let line = match line {
            Ok(line) => line,
            Err(_) => continue
        };

        match line.split_once(": ") {
            Some((k, v)) => {
                if k == "Release_date" {
                    release_date = v.to_string();
                } else if k == "Version" {
                    version = v.to_string();
                }
            }
            _ => continue
        }
    }

    Ok((release_date, version))
}

macro_rules! try_push_metric {
    ($metrics:expr, $row:expr, $index:expr, $name:expr, $desc:expr, $typ:expr) => {
        try_push_metric!($metrics, $row, $index, $name, $desc, $typ, tags!())
    };
    ($metrics:expr, $row:expr, $index:expr, $name:expr, $desc:expr, $typ:expr, $tags:expr) => {
        if $index <= $row.len() - 1 {
            let text = $row[$index];
            let value = match $index {
                STATUS_FIELD => Some(parse_status_field(text)),
                CHECK_DURATION_FIELD | QTIME_MS_FIELD | CTIME_MS_FIELD | RTIME_MS_FIELD | TTIME_MS_FIELD => match text.parse::<f64>() {
                    Ok(v) => Some( v / 1000.0 ),
                    Err(err) => {
                        warn!(
                            message = "Can't parse CSV field value",
                            value = text,
                            ?err
                        );

                        None
                    }
                }
                _ => match text.parse() {
                    Ok(v) => Some(v),
                    Err(err) => {
                        warn!(
                            message = "Can't parse CSV failed value",
                            value = text,
                            ?err
                        );

                        None
                    }
                }
            };

            if let Some(value) = value {
                match $typ {
                    "gauge" => $metrics.push(Metric::gauge_with_tags(
                        $name,
                        $desc,
                        value,
                        $tags
                    )),
                    "counter" => $metrics.push(Metric::sum_with_tags(
                        $name,
                        $desc,
                        value,
                        $tags
                    )),
                    _ => unreachable!()
                }
            }
        }
    };
}

fn parse_server(line: &str, pxname: &str, svname: &str) -> Vec<Metric> {
    let mut metrics = vec![];
    let row = line.split(",")
        .map(|s| s.trim())
        .collect::<Vec<_>>();

    try_push_metric!(metrics, row, 2,  "haproxy_server_current_queue", "Current number of queued requests assigned to this server.", "gauge");
    try_push_metric!(metrics, row, 3,  "haproxy_server_max_queue", "Maximum observed number of queued requests assigned to this server.", "gauge");
    try_push_metric!(metrics, row, 4,  "haproxy_server_current_sessions", "Current number of active sessions.", "gauge");
    try_push_metric!(metrics, row, 5,  "haproxy_server_max_sessions", "Maximum observed number of active sessions.", "gauge");
    try_push_metric!(metrics, row, 6,  "haproxy_server_limit_sessions", "Configured session limit.", "gauge");
    try_push_metric!(metrics, row, 7,  "haproxy_server_sessions_total", "Total number of sessions.", "counter");
    try_push_metric!(metrics, row, 8,  "haproxy_server_bytes_in_total", "Current total of incoming bytes.", "counter");
    try_push_metric!(metrics, row, 9,  "haproxy_server_bytes_out_total", "Current total of outgoing bytes.", "counter");
    try_push_metric!(metrics, row, 13, "haproxy_server_connection_errors_total", "Total of connection errors.", "counter");
    try_push_metric!(metrics, row, 14, "haproxy_server_response_errors_total", "Total of response errors.", "counter");
    try_push_metric!(metrics, row, 15, "haproxy_server_retry_warnings_total", "Total of retry warnings.", "counter");
    try_push_metric!(metrics, row, 16, "haproxy_server_redispatch_warnings_total", "Total of redispatch warnings.", "counter");
    try_push_metric!(metrics, row, 17, "haproxy_server_up", "Current health status of the server (1 = UP, 0 = DOWN).", "gauge");
    try_push_metric!(metrics, row, 18, "haproxy_server_weight", "Current weight of the server.", "gauge");
    try_push_metric!(metrics, row, 21, "haproxy_server_check_failures_total", "Total number of failed health checks.", "counter");
    try_push_metric!(metrics, row, 24, "haproxy_server_downtime_seconds_total", "Total downtime in seconds.", "counter");
    try_push_metric!(metrics, row, 30, "haproxy_server_server_selected_total", "Total number of times a server was selected, either for new sessions, or when re-dispatching.", "counter");
    try_push_metric!(metrics, row, 33, "haproxy_server_current_session_rate", "Current number of sessions per second over last elapsed second.", "gauge");
    try_push_metric!(metrics, row, 35, "haproxy_server_max_session_rate", "Maximum observed number of sessions per second.", "gauge");
    try_push_metric!(metrics, row, 38, "haproxy_server_check_duration_seconds", "Previously run health check duration, in seconds", "gauge");
    try_push_metric!(metrics, row, 39, "haproxy_server_http_responses_total", "Total of HTTP responses.", "counter", tags!("code" => "1xx"));
    try_push_metric!(metrics, row, 40, "haproxy_server_http_responses_total", "Total of HTTP responses.", "counter", tags!("code" => "2xx"));
    try_push_metric!(metrics, row, 41, "haproxy_server_http_responses_total", "Total of HTTP responses.", "counter", tags!("code" => "3xx"));
    try_push_metric!(metrics, row, 42, "haproxy_server_http_responses_total", "Total of HTTP responses.", "counter", tags!("code" => "4xx"));
    try_push_metric!(metrics, row, 43, "haproxy_server_http_responses_total", "Total of HTTP responses.", "counter", tags!("code" => "5xx"));
    try_push_metric!(metrics, row, 44, "haproxy_server_http_responses_total", "Total of HTTP responses.", "counter", tags!("code" => "other"));
    try_push_metric!(metrics, row, 49, "haproxy_server_client_aborts_total", "Total number of data transfers aborted by the client.", "counter");
    try_push_metric!(metrics, row, 50, "haproxy_server_server_aborts_total", "Total number of data transfers aborted by the server.", "counter");
    try_push_metric!(metrics, row, 58, "haproxy_server_http_queue_time_average_seconds", "Avg. HTTP queue time for last 1024 successful connections.", "gauge");
    try_push_metric!(metrics, row, 59, "haproxy_server_http_connect_time_average_seconds", "Avg. HTTP connect time for last 1024 successful connections.", "gauge");
    try_push_metric!(metrics, row, 60, "haproxy_server_http_response_time_average_seconds", "Avg. HTTP response time for last 1024 successful connections.", "gauge");
    try_push_metric!(metrics, row, 61, "haproxy_server_http_total_time_average_seconds", "Avg. HTTP total time for last 1024 successful connections.", "gauge");

    metrics.iter_mut()
        .map(|m| {
            m.insert_tag("backend", pxname);
            m.insert_tag("server", svname);

            m
        });

    metrics
}

fn parse_frontend(line: &str, pxname: &str) -> Vec<Metric> {
    let mut metrics = vec![];
    let row = line.split(",")
        .map(|s| s.trim())
        .collect::<Vec<_>>();

    try_push_metric!(metrics, row, 4,  "haproxy_frontend_current_sessions", "Current number of active sessions.", "gauge");
    try_push_metric!(metrics, row, 5,  "haproxy_frontend_max_sessions", "Maximum observed number of active sessions.", "gauge");
    try_push_metric!(metrics, row, 6,  "haproxy_frontend_limit_sessions", "Configured session limit.", "gauge");
    try_push_metric!(metrics, row, 7,  "haproxy_frontend_sessions_total", "Total number of sessions.", "counter");
    try_push_metric!(metrics, row, 8,  "haproxy_frontend_bytes_in_total", "Current total of incoming bytes.", "counter");
    try_push_metric!(metrics, row, 9,  "haproxy_frontend_bytes_out_total", "Current total of outgoing bytes.", "counter");
    try_push_metric!(metrics, row, 10, "haproxy_frontend_requests_denied_total", "Total of requests denied for security.", "counter");
    try_push_metric!(metrics, row, 12, "haproxy_frontend_request_errors_total", "Total of request errors.", "counter");
    try_push_metric!(metrics, row, 33, "haproxy_frontend_current_session_rate", "Current number of sessions per second over last elapsed second.", "gauge");
    try_push_metric!(metrics, row, 34, "haproxy_frontend_limit_session_rate", "Configured limit on new sessions per second.", "gauge");
    try_push_metric!(metrics, row, 35, "haproxy_frontend_max_session_rate", "Maximum observed number of sessions per second.", "gauge");
    try_push_metric!(metrics, row, 39, "haproxy_frontend_http_responses_total", "Total of HTTP responses.", "counter", tags!("code" => "1xx"));
    try_push_metric!(metrics, row, 40, "haproxy_frontend_http_responses_total", "Total of HTTP responses.", "counter", tags!("code" => "2xx"));
    try_push_metric!(metrics, row, 41, "haproxy_frontend_http_responses_total", "Total of HTTP responses.", "counter", tags!("code" => "3xx"));
    try_push_metric!(metrics, row, 42, "haproxy_frontend_http_responses_total", "Total of HTTP responses.", "counter", tags!("code" => "4xx"));
    try_push_metric!(metrics, row, 43, "haproxy_frontend_http_responses_total", "Total of HTTP responses.", "counter", tags!("code" => "5xx"));
    try_push_metric!(metrics, row, 44, "haproxy_frontend_http_responses_total", "Total of HTTP responses.", "counter", tags!("code" => "other"));
    try_push_metric!(metrics, row, 48, "haproxy_frontend_http_requests_total", "Total HTTP requests.", "counter");
    try_push_metric!(metrics, row, 51, "haproxy_frontend_compressor_bytes_in_total", "Number of HTTP response bytes fed to the compressor", "counter");
    try_push_metric!(metrics, row, 52, "haproxy_frontend_compressor_bytes_out_total", "Number of HTTP response bytes emitted by the compressor", "counter");
    try_push_metric!(metrics, row, 53, "haproxy_frontend_compressor_bytes_bypassed_total", "Number of bytes that bypassed the HTTP compressor", "counter");
    try_push_metric!(metrics, row, 54, "haproxy_frontend_http_responses_compressed_total", "Number of HTTP responses that were compressed", "counter");
    try_push_metric!(metrics, row, 79, "haproxy_frontend_connections_total", "Total number of connections", "counter");

    metrics.iter_mut()
        .map(|m| m.insert_tag("frontend", pxname));

    metrics
}

fn parse_backend(line: &str, pxname: &str) -> Vec<Metric> {
    let mut metrics = vec![];
    let row = line.split(",")
        .map(|s| s.trim())
        .collect::<Vec<_>>();

    try_push_metric!(metrics, row, 2,  "haproxy_backend_current_queue", "Current number of queued requests not assigned to any server.", "gauge");
    try_push_metric!(metrics, row, 3,  "haproxy_backend_max_queue", "Maximum observed number of queued requests not assigned to any server.", "gauge");
    try_push_metric!(metrics, row, 4,  "haproxy_backend_current_sessions", "Current number of active sessions.", "gauge");
    try_push_metric!(metrics, row, 5,  "haproxy_backend_max_sessions", "Maximum observed number of active sessions.", "gauge");
    try_push_metric!(metrics, row, 6,  "haproxy_backend_limit_sessions", "Configured session limit.", "gauge");
    try_push_metric!(metrics, row, 7,  "haproxy_backend_sessions_total", "Total number of sessions.", "counter");
    try_push_metric!(metrics, row, 8,  "haproxy_backend_bytes_in_total", "Current total of incoming bytes.", "counter");
    try_push_metric!(metrics, row, 9,  "haproxy_backend_bytes_out_total", "Current total of outgoing bytes.", "counter");
    try_push_metric!(metrics, row, 13, "haproxy_backend_connection_errors_total", "Total of connection errors.", "counter");
    try_push_metric!(metrics, row, 14, "haproxy_backend_response_errors_total", "Total of response errors.", "counter");
    try_push_metric!(metrics, row, 15, "haproxy_backend_retry_warnings_total", "Total of retry warnings.", "counter");
    try_push_metric!(metrics, row, 16, "haproxy_backend_redispatch_warnings_total", "Total of redispatch warnings.", "counter");
    try_push_metric!(metrics, row, 17, "haproxy_backend_up", "Current health status of the backend (1 = UP, 0 = DOWN).", "gauge");
    try_push_metric!(metrics, row, 18, "haproxy_backend_weight", "Total weight of the servers in the backend.", "gauge");
    try_push_metric!(metrics, row, 19, "haproxy_backend_current_server", "Current number of active servers", "gauge");
    try_push_metric!(metrics, row, 30, "haproxy_backend_server_selected_total", "Total number of times a server was selected, either for new sessions, or when re-dispatching.", "counter");
    try_push_metric!(metrics, row, 33, "haproxy_backend_current_session_rate", "Current number of sessions per second over last elapsed second.", "gauge");
    try_push_metric!(metrics, row, 35, "haproxy_backend_max_session_rate", "Maximum number of sessions per second.", "gauge");
    try_push_metric!(metrics, row, 39, "haproxy_backend_http_responses_total", "Total of HTTP responses.", "counter", tags!("code" => "1xx"));
    try_push_metric!(metrics, row, 40, "haproxy_backend_http_responses_total", "Total of HTTP responses.", "counter", tags!("code" => "2xx"));
    try_push_metric!(metrics, row, 41, "haproxy_backend_http_responses_total", "Total of HTTP responses.", "counter", tags!("code" => "3xx"));
    try_push_metric!(metrics, row, 42, "haproxy_backend_http_responses_total", "Total of HTTP responses.", "counter", tags!("code" => "4xx"));
    try_push_metric!(metrics, row, 43, "haproxy_backend_http_responses_total", "Total of HTTP responses.", "counter", tags!("code" => "5xx"));
    try_push_metric!(metrics, row, 44, "haproxy_backend_http_responses_total", "Total of HTTP responses.", "counter", tags!("code" => "other"));
    try_push_metric!(metrics, row, 49, "haproxy_backend_client_aborts_total", "Total number of data transfers aborted by the client.", "counter");
    try_push_metric!(metrics, row, 50, "haproxy_backend_server_aborts_total", "Total number of data transfers aborted by the server.", "counter");
    try_push_metric!(metrics, row, 51, "haproxy_backend_compressor_bytes_in_total", "Number of HTTP response bytes fed to the compressor", "counter");
    try_push_metric!(metrics, row, 52, "haproxy_backend_compressor_bytes_out_total", "Number of HTTP response bytes emitted by the compressor", "counter");
    try_push_metric!(metrics, row, 53, "haproxy_backend_compressor_bytes_bypassed_total", "Number of bytes that bypassed the HTTP compressor", "counter");
    try_push_metric!(metrics, row, 54, "haproxy_backend_http_responses_compressed_total", "Number of HTTP responses that were compressed", "counter");
    try_push_metric!(metrics, row, 58, "haproxy_backend_http_queue_time_average_seconds", "Avg. HTTP queue time for last 1024 successful connections.", "gauge");
    try_push_metric!(metrics, row, 59, "haproxy_backend_http_connect_time_average_seconds", "Avg. HTTP connect time for last 1024 successful connections.", "gauge");
    try_push_metric!(metrics, row, 60, "haproxy_backend_http_response_time_average_seconds", "Avg. HTTP response time for last 1024 successful connections.", "gauge");
    try_push_metric!(metrics, row, 61, "haproxy_backend_http_total_time_average_seconds", "Avg. HTTP total time for last 1024 successful connections.", "gauge");

    metrics.iter_mut()
        .map(|m| m.insert_tag("backend", pxname));

    metrics
}

#[inline]
fn parse_status_field(value: &str) -> f64 {
    match value {
        "UP" | "UP 1/3" | "UP 2/3" | "OPEN" | "no check" | "DRAIN" => 1.0,
        "DOWN" | "DOWN 1/2" | "NOLB" | "MAINT" | "MAINT(via)" | "MAINT(resolution)" => 0.0,
        _ => 0.0
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::io::BufReader;
    use super::*;

    #[test]
    fn test_parse_info() {
        let input = "Release_date: test date\nVersion: test version\n";
        let reader = BufReader::new(io::Cursor::new(input));

        let (release, version) = parse_info(reader).unwrap();
        assert_eq!(release, "test date");
        assert_eq!(version, "test version");
    }

    #[test]
    fn test_parse_csv_resp() {
        let content = include_str!("../../tests/fixtures/haproxy/stats.csv");
        let reader = BufReader::new(io::Cursor::new(content));
        let metrics = parse_csv(reader).unwrap();
    }

    #[test]
    fn test_parse_status_field() {
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
            ("", 0)
        ];

        for (input, want) in tests {
            assert_eq!(parse_status_field(input), want as f64);
        }
    }
}