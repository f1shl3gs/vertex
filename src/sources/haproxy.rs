use std::io::BufRead;

use snafu;
use futures::TryFutureExt;
use http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use event::Metric;

use crate::http::{Auth, HttpClient};
use crate::sources::Source;
use crate::tls::{MaybeTlsSettings, TlsConfig};
use crate::config::{deserialize_duration, serialize_duration, default_interval, SourceConfig, SourceContext, DataType, SourceDescription, GenerateConfig, ProxyConfig};
use crate::Error;


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
    UnknownTypeOfMetrics { typ: String }
}

fn parse_csv(reader: impl BufRead) -> Result<Vec<Metric>, ParseError> {
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

        match typ {
            "0" => {
                // frontend

            },
            "1" => {
                // backend
            },
            "2" => {
                // server
            },
            _ => {
                return Err(ParseError::UnknownTypeOfMetrics { typ: typ.into() })
            }
        }
    }
    todo!()
}

fn parse_info(reader: impl io::BufRead) -> Result<(String, String), Error> {
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
            },
            _ => continue
        }
    }

    Ok((release_date, version))
}

#[cfg(test)]
mod tests {
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
        let metrics = parse_csv_resp(content).unwrap();

    }
}