use std::collections::HashMap;
use std::future::Future;
use std::io::BufRead;

use snafu::{OptionExt, ResultExt, Snafu};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::config::{DataType, default_interval, deserialize_duration, serialize_duration, SourceConfig, SourceContext, ticker_from_duration};
use crate::sources::Source;

/*
	crlf            = []byte("\r\n")
	space           = []byte(" ")
	resultOK        = []byte("OK\r\n")
	resultStored    = []byte("STORED\r\n")
	resultNotStored = []byte("NOT_STORED\r\n")
	resultExists    = []byte("EXISTS\r\n")
	resultNotFound  = []byte("NOT_FOUND\r\n")
	resultDeleted   = []byte("DELETED\r\n")
	resultEnd       = []byte("END\r\n")
	resultOk        = []byte("OK\r\n")
	resultTouched   = []byte("TOUCHED\r\n")
	resultReset     = []byte("RESET\r\n")

	resultClientErrorPrefix = []byte("CLIENT_ERROR ")
	resultStatPrefix        = []byte("STAT")
*/

const CLIENT_ERROR_PREFIX: &str = "CLIENT_ERROR";
const STAT_PREFIX: &str = "STAT";
const END_PREFIX: &str = "END";

#[derive(Debug, Deserialize, Serialize)]
struct MemcachedConfig {
    endpoints: Vec<String>,
    #[serde(default = "default_interval")]
    #[serde(deserialize_with = "deserialize_duration", serialize_with = "serialize_duration")]
    interval: chrono::Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "memcached")]
impl SourceConfig for MemcachedConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        todo!()
    }

    fn output_type(&self) -> DataType {
        DataType::Metric
    }

    fn source_type(&self) -> &'static str {
        "memcached1"
    }
}

/// Stats is a type for storing current statistics of a Memcached server
#[derive(Default)]
struct Stats {
    // Stats are the top level key = value metrics from memcached
    stats: HashMap<String, String>,
    // Slabs are indexed by slab ID. Each has a k/v store of metrics for
    // that slab
    slabs: HashMap<i32, HashMap<String, String>>,

    // Items are indexed by slab ID. Each ID has a k/v store of metrics for
    // items in that slab
    items: HashMap<i32, HashMap<String, String>>,
}

#[derive(Debug, Snafu)]
enum ParseError {
    #[snafu(display("invalid line"))]
    InvalidLine,
    #[snafu(display("invalid value found: {}", source))]
    InvalidValue { source: std::num::ParseFloatError },
    #[snafu(display("read line failed: {}", source))]
    ReadLine { source: std::io::Error },
    #[snafu(display("command {} execute failed: {}", cmd, source))]
    CommandExecFailed { cmd: String, source: std::io::Error },
    #[snafu(display("client error"))]
    ClientError,
}

fn parse_line(line: &str) -> Result<(String, f64), ParseError> {
    let parts = line.split_ascii_whitespace()
        .collect::<Vec<_>>();

    if parts.len() != 3 {
        return Err(ParseError::InvalidLine);
    }

    if parts[0] != "STAT" {
        return Err(ParseError::InvalidLine);
    }

    let v = parts[2].parse()
        .context(InvalidValue)?;

    Ok((parts[1].to_string(), v))
}

async fn stats<Fut>(
    addr: &str,
    query: impl FnOnce(&str, &str) -> Fut,
) -> Result<Stats, ParseError>
    where
        Fut: Future<Output=Result<String, std::io::Error>>
{
    let mut stats = Stats::default();
    for cmd in vec!["stats\r\n", "stats slabs\r\n", "stats items\r\n"] {
        let mut lines = query(addr, cmd)
            .await?
            .as_str()
            .lines();

        while let Some(line) = lines.next() {
            if line.starts_with(CLIENT_ERROR_PREFIX) {
                // TODO: more error context
                return Err(ParseError::ClientError);
            }

            if !line.starts_with(STAT_PREFIX) {
                continue;
            }

            let parts = line.split_ascii_whitespace()
                .collect::<Vec<_>>();
            if parts.len() != 3 {
                continue;
            }

            let subs = parts[1].split_ascii_whitespace()
                .collect::<Vec<_>>();
            match subs.len() {
                1 => {
                    // Global stats
                    stats.stats.insert(parts[1].to_string(), parts[2].to_string());
                }

                2 => {
                    // Slab stats
                    let index = subs[0].parse()?;
                    let mut slab = stats.slabs
                        .entry(index)
                        .or_insert(Default::default());
                    slab.insert(subs[1].to_string(), parts[2].to_string());
                }

                3 => {
                    // Slab item stats
                    let index = subs[1].parse()?;
                    let mut item = stats.items
                        .entry(index)
                        .or_insert(Default::default());
                    item.insert(subs[2].to_string(), parts[2].to_string());
                }

                _ => {}
            }
        }
    }

    Ok(stats)
}

async fn request(addr: &str, cmd: &str) -> Result<String, std::io::Error> {
    let socket = TcpStream::connect(addr).await?;
    let (mut reader, mut writer) = tokio::io::split(socket);

    writer.write_all(cmd.as_bytes()).await?;

    let mut buf = String::new();
    reader.read_to_string(&mut buf).await?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use std::io::BufReader;
    use super::*;

    #[test]
    fn test_parse_stats() {
        let input = include_str!("../../testdata/memcached/stats.txt");
        let stats = parse_stats(input).unwrap();
        assert_eq!(stats.len(), 90)
    }

    mod memcached {
        use std::collections::HashMap;
        use testcontainers::{Container, Docker, Image, WaitForMessage};

        const CONTAINER_IDENTIFIER: &str = "memcached";
        const DEFAULT_TAG: &str = "1.6.12-alpine3.14";

        #[derive(Debug, Clone, Default)]
        pub struct MemcachedArgs;

        impl IntoIterator for MemcachedArgs {
            type Item = String;
            type IntoIter = std::vec::IntoIter<String>;

            fn into_iter(self) -> Self::IntoIter {
                vec![].into_iter()
            }
        }

        pub struct Memcached {
            arguments: MemcachedArgs,
            tag: String,
        }

        impl Default for Memcached {
            fn default() -> Self {
                Self {
                    arguments: Default::default(),
                    tag: DEFAULT_TAG.into(),
                }
            }
        }

        impl Image for Memcached {
            type Args = MemcachedArgs;
            type EnvVars = HashMap<String, String>;
            type Volumes = HashMap<String, String>;
            type EntryPoint = std::convert::Infallible;

            fn descriptor(&self) -> String {
                format!("{}:{}", CONTAINER_IDENTIFIER, self.tag)
            }

            fn wait_until_ready<D: Docker>(&self, container: &Container<'_, D, Self>) {
                container
                    .logs()
                    .stdout
                    .wait_for_message("server listening")
                    .unwrap();
            }

            fn args(&self) -> Self::Args {
                self.arguments.clone()
            }

            fn env_vars(&self) -> Self::EnvVars {
                Default::default()
            }

            fn volumes(&self) -> Self::Volumes {
                Default::default()
            }

            fn with_args(self, arguments: Self::Args) -> Self {
                Memcached { arguments, ..self }
            }
        }
    }

    #[test]
    fn test_parse() {
        let tests = vec![
            ("STAT lru_bumps_dropped 0", "lru_bumps_dropped", 0.0),
            ("STAT limit_maxbytes 67108864", "limit_maxbytes", 67108864.0),
        ];

        for (input, want_key, want_value) in tests {
            let (key, value) = parse_line(input).unwrap();
            assert_eq!(key, want_key);
            assert_eq!(value, want_value)
        }
    }
}