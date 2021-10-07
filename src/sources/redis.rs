use std::collections::BTreeMap;
use std::num::ParseIntError;
use futures::{SinkExt, StreamExt};
use nom::InputIter;
use redis::{InfoDict, ToRedisArgs};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncBufReadExt;
use tokio_stream::wrappers::IntervalStream;
use event::{Event, Metric};
use snafu::Snafu;
use crate::config::{DataType, SourceConfig, SourceContext, deserialize_duration, serialize_duration, default_interval};
use crate::pipeline::Pipeline;
use crate::shutdown::ShutdownSignal;
use crate::sources::Source;
use event::{tags};

#[derive(Debug, Snafu)]
enum Error {
    #[snafu(display("Invalid slave line"))]
    InvalidSlaveLine,

    #[snafu(display("Redis error: {}", source))]
    RedisError {
        source: redis::RedisError
    },

    #[snafu(display("Parse error: {}", source))]
    ParseError {
        source: ParseIntError,
    },
}

impl From<redis::RedisError> for Error {
    fn from(source: redis::RedisError) -> Self {
        Self::RedisError {
            source,
        }
    }
}

impl From<ParseIntError> for Error {
    fn from(source: ParseIntError) -> Self {
        Self::ParseError {
            source,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RedisSourceConfig {
    // something looks like this, e.g. redis://host:port/db
    url: String,

    #[serde(default = "default_interval")]
    #[serde(deserialize_with = "deserialize_duration", serialize_with = "serialize_duration")]
    interval: chrono::Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "redis_info")]
impl SourceConfig for RedisSourceConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        let params = self.url.clone();
        let mut output = ctx.out;
        let mut ticker = IntervalStream::new(tokio::time::interval(self.interval.to_std().unwrap()))
            .take_until(ctx.shutdown);

        Ok(Box::pin(async move {
            while let Some(_) = ticker.next().await {
                match scrap(params.as_ref()).await {
                    Ok(infos) => {
                        println!("{:#?}", infos);
                    }
                    Err(_) => {
                        output.send(Metric::gauge(
                            "redis_up",
                            "redis status",
                            0,
                        ).into());
                    }
                }
            }

            Ok(())
        }))
    }

    fn output_type(&self) -> DataType {
        DataType::Metric
    }

    fn source_type(&self) -> &'static str {
        "redis_info"
    }
}

async fn scrap(url: &str) -> Result<String, Error> {
    let cli = redis::Client::open(url)?;
    let mut conn = cli.get_async_connection().await?;

    let resp = redis::cmd("INFO")
        .arg("ALL")
        .query_async(&mut conn)
        .await?;

    Ok(resp)
}

fn extract_info_metrics(infos: &str) -> Result<Vec<Event>, std::io::Error> {
    let mut events = vec![];
    let mut kvs = BTreeMap::new();
    let mut master_host = String::new();
    let mut master_port = String::new();
    let mut field_class = String::new();

    for line in infos.lines() {
        let line = line.trim();
        if line.len() == 0 {
            continue;
        }

        if line.starts_with("# ") {
            field_class = line[2..].to_string();
            continue;
        }

        if line.len() < 2 || !line.contains(':') {
            continue;
        }

        let mut fields = line.splitn(2, ':');
        let key = fields.next().unwrap();
        let value = fields.next().unwrap();

        kvs.insert(key.to_string(), value.to_string());
        if key == "master_host" {
            master_host = value.to_string();
        }

        if key == "master_port" {
            master_port = value.to_string();
        }

        match field_class.as_ref() {
            "Replication" => {
                if let Ok(evs) = handle_replication_metrics(master_host, master_port, key, value) {
                    events.extend(evs);
                }
            }

            "Server" => {

            }
        }
    }

    Ok(events)
}

fn handle_replication_metrics(host: &str, port: &str, key: &str, value: &str) -> Result<Vec<Event>, Error> {
    // only slaves have this field
    if key == "master_link_status" {
        let v = match value {
            "up" => 1,
            _ => 0
        };

        return Ok(vec![Metric::gauge_with_tags(
            "master_link_up",
            "",
            v,
            tags!(
                    "master_host" => host,
                    "master_port" => port
                ),
        ).into()]);
    }

    match key {
        "master_last_io_seconds_ago" | "slave_repl_offset" | "master_sync_in_progress" => {
            let v = value.parse::<i32>()?;
            return Ok(Metric::gauge_with_tags(
                key,
                "",
                v,
                tags!(
                    "master_host" => host,
                    "master_port" => port
                ),
            ).into());
        }

        _ => {}
    }

    // not a slave, try extracting master metrics
    if let Ok((offset, ip, port, state, lag)) = parse_connected_slave_string(key, value) {
        let mut events = vec![];
        events.push(Metric::gauge_with_tags(
            "connected_slave_offset_bytes",
            "Offset of connected slave",
            offset,
            tags!(
                "slave_ip" => ip,
                "slave_port" => port,
                "slave_state" => state
            ),
        ).into());

        if lag > -1.0 {
            events.push(Metric::gauge_with_tags(
                "connected_slave_lag_seconds",
                "Lag of connected slave",
                lag,
                tags!(
                    "slave_ip" => ip,
                    "slave_port" => port,
                    "slave_state" => state
                ),
            ).into())
        }

        return Ok(events);
    }

    Ok(vec![])
}

/// the slave line looks like
///
/// ```text
/// slave0:ip=10.254.11.1,port=6379,state=online,offset=1751844676,lag=0
///    slave1:ip=10.254.11.2,port=6379,state=online,offset=1751844222,lag=0
/// ```
fn parse_connected_slave_string(slave: &str, kvs: &str) -> Result<(f64, String, String, String, f64), Error> {
    let mut lag = 0.0;
    let mut connected_kvs = BTreeMap::new();

    if !validate_slave_line(slave) {
        return Err(Error::InvalidSlaveLine);
    }

    for part in kvs.split(',') {
        let kv = part.split(b'=')
            .collect::<Vec<_>>();
        if kv.len() != 2 {
            return Err(Error::InvalidSlaveLine);
        }

        connected_kvs.insert(kv[0].to_string(), kv[1].to_string());
    }

    let offset = connected_kvs.get("offset")
        .unwrap_or("0".as_ref())
        .parse::<f64>()
        .map_err(|_| Error::InvalidSlaveLine)?;

    if let Some(text) = connected_kvs.get("lag") {
        lag = text.parse()?;
    } else {
        lag = -1.0;
    }

    let ip = connected_kvs.get("ip").unwrap_or("".as_ref());
    let port = connected_kvs.get("port").unwrap_or("".as_ref());
    let state = connected_kvs.get("state").unwrap_or("".as_ref());

    Ok((offset, ip.to_string(), port.to_string(), state.to_string(), lag))
}

fn validate_slave_line(line: &str) -> bool {
    if !line.starts_with("slave") {
        return false;
    }

    if line.len() <= 5 {
        return false;
    }

    let c = line[5];
    return c >= b'0' && c <= b'9';
}

#[cfg(test)]
mod tests {
    use testcontainers::{
        Docker,
        images::redis::Redis,
    };
    use redis::Client;
    use super::*;

    #[tokio::test]
    async fn dump_info() {
        let docker = testcontainers::clients::Cli::default();
        let service = docker.run(Redis::default());
        let host_port = service.get_host_port(6379).unwrap();
        let url = format!("redis://localhost:{}", host_port);

        let infos = scrap(url.as_ref()).await.unwrap();
        println!("{:#?}", infos);
    }

    #[test]
    fn test_info() {
        let cli = Client::open("redis://localhost:6379").unwrap();
        let mut conn = cli.get_connection().unwrap();
        let info: redis::InfoDict = redis::cmd("INFO").query(&mut conn).unwrap();
        println!("{:#?}", info);
    }
}