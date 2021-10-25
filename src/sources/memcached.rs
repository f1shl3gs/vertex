use std::collections::HashMap;
use std::future::Future;
use std::io::{BufRead, Stdout};
use std::time::Instant;

use futures::{SinkExt, StreamExt};
use snafu::{OptionExt, ResultExt, Snafu};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use event::{Event, Metric, tags};

use crate::config::{DataType, default_interval, deserialize_duration, serialize_duration, SourceConfig, SourceContext, ticker_from_duration};
use crate::sources::Source;

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
        let mut ticker = ticker_from_duration(self.interval)
            .unwrap()
            .take_until(ctx.shutdown);

        let mut output = ctx.out
            .sink_map_err(|err| error!(
                message = "Error sending memcached metrics",
                %err
            ));

        let endpoints = self.endpoints.clone();
        Ok(Box::pin(async move {
            while ticker.next().await.is_some() {
                let metrics = futures::future::join_all(
                    endpoints
                        .iter()
                        .map(|addr| gather(addr))
                ).await;

                let mut stream = futures::stream::iter(metrics)
                    .map(futures::stream::iter)
                    .flatten()
                    .map(Event::Metric)
                    .map(Ok);

                output.send_all(&mut stream).await?
            }

            Ok(())
        }))
    }

    fn output_type(&self) -> DataType {
        DataType::Metric
    }

    fn source_type(&self) -> &'static str {
        "memcached1"
    }
}

macro_rules! get_value {
    ($map:expr, $key:expr) => {
        *$map.get($key).unwrap_or(&0.0)
    };
}

async fn gather(addr: &str) -> Vec<Metric> {
    let mut metrics = vec![];

    let start = Instant::now();
    let result = fetch_stats(addr, query).await;
    let elapsed = start.elapsed().as_secs_f64();
    let up = result.is_ok();

    match result {
        Ok(Stats { version, libevent, stats, slabs, items }) => {
            metrics.extend_from_slice(&[
                Metric::gauge_with_tags(
                    "memcached_version",
                    "The version of this memcached server.",
                    1,
                    tags!(
                "version" => version,
                "libevent" => libevent
            ),
                )
            ]);

            for op in vec!["get", "delete", "inc", "decr", "cas", "touch"] {
                let hits = get_value!(stats, (op.to_owned() + "_hits").as_str());
                let misses = get_value!(stats, (op.to_owned() + "_misses").as_str());

                metrics.extend_from_slice(&[
                    Metric::sum_with_tags(
                        "memcached_commands_total",
                        "Total number of all requests broken down by command (get, set, etc.) and status.",
                        hits,
                        tags!(
                    "command" => op,
                    "status" => "hit"
                ),
                    ),
                    Metric::sum_with_tags(
                        "memcached_commands_total",
                        "Total number of all requests broken down by command (get, set, etc.) and status.",
                        misses,
                        tags!(
                    "command" => op,
                    "status" => "miss"
                ),
                    )
                ])
            }


            metrics.extend_from_slice(&[
                Metric::sum(
                    "memcached_uptime_seconds",
                    "Number of seconds since the server started.",
                    get_value!(stats, "uptime"),
                ),
                Metric::gauge(
                    "memcached_time_seconds",
                    "current UNIX time according to the server.",
                    get_value!(stats, "time"),
                ),
                Metric::sum_with_tags(
                    "memcached_commands_total",
                    "Total number of all requests broken down by command (get, set, etc.) and status.",
                    get_value!(stats, "cas_badval"),
                    tags!(
                        "command" => "cas",
                        "status" => "badval"
                    ),
                ),
                Metric::sum_with_tags(
                    "memcached_commands_total",
                    "Total number of all requests broken down by command (get, set, etc.) and status.",
                    get_value!(stats, "cmd_flush"),
                    tags!(
                        "command" => "flush",
                        "status" => "hit"
                    ),
                )
            ]);

            // memcached includes cas operations again in cmd_set
            let sets = get_value!(stats, "cmd_set");
            let cas = get_value!(stats, "cas_misses")
                + get_value!(stats, "cas_hits")
                + get_value!(stats, "cas_badval");
            metrics.push(Metric::sum_with_tags(
                "memcached_commands_total",
                "Total number of all requests broken down by command (get, set, etc.) and status.",
                sets - cas,
                tags!(
            "command" => "set",
            "status" => "hit"
        ),
            ));

            metrics.extend_from_slice(&[
                Metric::sum(
                    "memcached_process_user_cpu_seconds_total",
                    "Accumulated user time for this process",
                    get_value!(stats, "rusage_user"),
                ),
                Metric::sum(
                    "memcached_process_system_cpu_seconds_total",
                    "Accumulated system time for this process",
                    get_value!(stats, "rusage_system"),
                ),
                Metric::gauge(
                    "memcached_current_bytes",
                    "Current number of bytes used to store items.",
                    get_value!(stats, "bytes"),
                ),
                Metric::gauge(
                    "memcached_limit_bytes",
                    "Number of bytes this server is allowed to use for storage.",
                    get_value!(stats, "limit_maxbytes"),
                ),
                Metric::gauge(
                    "memcached_current_items",
                    "Current number of items stored by this instance.",
                    get_value!(stats, "curr_items"),
                ),
                Metric::sum(
                    "memcached_items_total",
                    "Total number of items stored during the life of this instance.",
                    get_value!(stats, "total_items"),
                ),
                Metric::sum(
                    "memcached_read_bytes_total",
                    "Total number of bytes read by this server from network.",
                    get_value!(stats, "bytes_read"),
                ),
                Metric::sum(
                    "memcached_written_bytes_total",
                    "Total number of bytes sent by this server to network.",
                    get_value!(stats, "bytes_written"),
                ),
                Metric::gauge(
                    "memcached_current_connections",
                    "Current number of open connections.",
                    get_value!(stats, "curr_connections"),
                ),
                Metric::sum(
                    "memcached_connections_total",
                    "Total number of connections opened since the server started running.",
                    get_value!(stats, "total_connections"),
                ),
                Metric::sum(
                    "memcached_connections_rejected_total",
                    "Total number of connections rejected due to hitting the memcached's -c limit in maxconns_fast mode.",
                    get_value!(stats, "rejected_connections"),
                ),
                Metric::sum(
                    "memcached_connections_yielded_total",
                    "Total number of connections yielded running due to hitting the memcached's -R limit.",
                    get_value!(stats, "conn_yields"),
                ),
                Metric::sum(
                    "memcached_connections_listener_disabled_total",
                    "Number of times that memcached has hit its connections limit and disabled its listener.",
                    get_value!(stats, "listen_disabled_num"),
                ),
                Metric::sum(
                    "memcached_items_evicted_total",
                    "Total number of valid items removed from cache to free memory for new items.",
                    get_value!(stats, "evictions"),
                ),
                Metric::sum(
                    "memcached_items_reclaimed_total",
                    "Total number of times an entry was stored using memory from an expired entry.",
                    get_value!(stats, "reclaimed"),
                ),
                Metric::sum(
                    "memcached_lru_crawler_starts_total",
                    "Times an LRU crawler was started.",
                    get_value!(stats, "lru_crawler_starts"),
                ),
                Metric::sum(
                    "memcached_lru_crawler_items_checked_total",
                    "Total items examined by LRU Crawler.",
                    get_value!(stats, "crawler_items_checked"),
                ),
                Metric::sum(
                    "memcached_lru_crawler_reclaimed_total",
                    "Total items freed by LRU Crawler.",
                    get_value!(stats, "crawler_reclaimed"),
                ),
                Metric::sum(
                    "memcached_lru_crawler_moves_to_cold_total",
                    "Total number of items moved from HOT/WARM to COLD LRU's.",
                    get_value!(stats, "moves_to_cold"),
                ),
                Metric::sum(
                    "memcached_lru_crawler_moves_to_warm_total",
                    "Total number of items moved from COLD to WARM LRU.",
                    get_value!(stats, "moves_to_warm"),
                ),
                Metric::sum(
                    "memcached_lru_crawler_moves_within_lru_total",
                    "Total number of items reshuffled within HOT or WARM LRU's.",
                    get_value!(stats, "moves_within_lru"),
                ),
                Metric::gauge(
                    "memcached_malloced_bytes",
                    "Number of bytes of memory allocated to slab pages",
                    get_value!(stats, "total_malloced"),
                )
            ]);

            for (slab, stats) in slabs {
                let slab = slab.as_str();
                for op in vec!["get", "delete", "incr", "decr", "cas", "touch"] {
                    metrics.push(Metric::sum_with_tags(
                        "memcached_slab_commands_total",
                        "Total number of all requests broken down by command (get, set, etc.) and status per slab.",
                        get_value!(stats, (op.to_owned() + "_hits").as_str()),
                        tags!(
                    "slab" => slab,
                    "command" => op,
                    "status" => "hit"
                ),
                    ));
                }

                metrics.push(Metric::sum_with_tags(
                    "memcached_slab_commands_total",
                    "Total number of all requests broken down by command (get, set, etc.) and status per slab.",
                    get_value!(stats, "cas_badval"),
                    tags!(
                "slab" => slab,
                "command" => "cas",
                "status" => "badval"
            ),
                ));

                let sets = get_value!(stats, "cmd_set");
                let cases = get_value!(stats, "cas_hits") + get_value!(stats, "cas_badval");
                metrics.push(Metric::sum_with_tags(
                    "memcached_slab_commands_total",
                    "Total number of all requests broken down by command (get, set, etc.) and status per slab.",
                    sets - cases,
                    tags!(
                "slab" => slab,
                "command" => "set",
                "status" => "hit"
            ),
                ));

                metrics.extend_from_slice(&[
                    Metric::gauge_with_tags(
                        "memcached_slab_chunk_size_bytes",
                        "Number of bytes allocated to each chunk within this slab class.",
                        get_value!(stats, "chunk_size"),
                        tags!(
                    "slab" => slab
                ),
                    ),
                    Metric::gauge_with_tags(
                        "memcached_slab_chunks_per_page",
                        "Number of chunks within a single page for this slab class.",
                        get_value!(stats, "chunks_per_page"),
                        tags!(
                    "slab" => slab
                ),
                    ),
                    Metric::gauge_with_tags(
                        "memcached_slab_current_pages",
                        "Number of pages allocated to this slab class.",
                        get_value!(stats, "total_pages"),
                        tags!(
                    "slab" => slab
                ),
                    ),
                    Metric::gauge_with_tags(
                        "memcached_slab_current_chunks",
                        "Number of chunks allocated to this slab class.",
                        get_value!(stats, "total_chunks"),
                        tags!(
                    "slab" => slab
                ),
                    ),
                    Metric::gauge_with_tags(
                        "memcached_slab_chunks_used",
                        "Number of chunks allocated to an item",
                        get_value!(stats, "used_chunks"),
                        tags!(
                    "slab" => slab
                ),
                    ),
                    Metric::gauge_with_tags(
                        "memcached_slab_chunks_free",
                        "Number of chunks not yet allocated items",
                        get_value!(stats, "free_chunks"),
                        tags!(
                    "slab" => slab
                ),
                    ),
                    Metric::gauge_with_tags(
                        "memcached_slab_chunks_free_end",
                        "Number of free chunks at the end of the last allocated page",
                        get_value!(stats, "free_chunks_end"),
                        tags!(
                    "slab" => slab
                ),
                    ),
                    Metric::gauge_with_tags(
                        "memcached_slab_mem_requested_bytes",
                        "Number of bytes of memory actual items take up within a slab",
                        get_value!(stats, "mem_requested"),
                        tags!(
                    "slab" => slab
                ),
                    )
                ]);
            }

            for (slab, stats) in items {
                let slab = slab.as_str();

                metrics.extend_from_slice(&[
                    Metric::gauge_with_tags(
                        "memcached_slab_current_items",
                        "Number of items currently stored in this slab class",
                        get_value!(stats, "number"),
                        tags!(
                    "slab" => slab
                ),
                    ),
                    Metric::gauge_with_tags(
                        "memcached_slab_items_age_seconds",
                        "Number of seconds the oldest item has been in the slab class",
                        get_value!(stats, "age"),
                        tags!(
                    "slab" => slab
                ),
                    ),
                    Metric::sum_with_tags(
                        "memcached_slab_lru_hits_total",
                        "Number of get_hits to the LRU",
                        get_value!(stats, "hits_to_hot"),
                        tags!(
                    "slab" => slab,
                    "lru" => "hot"
                ),
                    ),
                    Metric::sum_with_tags(
                        "memcached_slab_lru_hits_total",
                        "Number of get_hits to the LRU",
                        get_value!(stats, "hits_to_warm"),
                        tags!(
                    "slab" => slab,
                    "lru" => "warm"
                ),
                    ),
                    Metric::sum_with_tags(
                        "memcached_slab_lru_hits_total",
                        "Number of get_hits to the LRU",
                        get_value!(stats, "hits_to_cold"),
                        tags!(
                    "slab" => slab,
                    "lru" => "cold"
                ),
                    ),
                    Metric::sum_with_tags(
                        "memcached_slab_lru_hits_total",
                        "Number of get_hits to the LRU",
                        get_value!(stats, "hits_to_temporary"),
                        tags!(
                    "slab" => slab,
                    "lru" => "temporary"
                ),
                    ),
                ]);

                for (key, name, desc) in [
                    (
                        "crawler_reclaimed",
                        "memcached_slab_items_crawler_reclaimed_total",
                        "Number of items freed by the LRU Crawler."
                    ),
                    (
                        "evicted",
                        "memcached_slab_items_evicted_total",
                        "Total number of times an item had to be evicted from the LRU before it expired.",
                    ),
                    (
                        "evicted_nonzero",
                        "memcached_slab_items_evicted_nonzero_total",
                        "Total number of times an item which had an explicit expire time set had to be evicted from the LRU before it expired.",
                    ),
                    (
                        "evicted_time",
                        "memcached_slab_items_evicted_time_seconds",
                        "Seconds since the last access for the most recent item evicted from this class.",
                    ),
                    (
                        "evicted_unfetched",
                        "memcached_items_evicted_unfetched_total",
                        "Total number of items evicted and never fetched.",
                    ),
                    (
                        "expired_unfetched",
                        "memcached_slab_items_expired_unfetched_total",
                        "Total number of valid items evicted from the LRU which were never touched after being set.",
                    ),
                    (
                        "outofmemory",
                        "memcached_slab_items_outofmemory_total",
                        "Total number of items for this slab class that have triggered an out of memory error.",
                    ),
                    (
                        "reclaimed",
                        "memcached_slab_items_reclaimed_total",
                        "Total number of items reclaimed.",
                    ),
                    (
                        "tailrepairs",
                        "memcached_slab_items_tailrepairs_total",
                        "Total number of times the entries for a particular ID need repairing.",
                    ),
                    (
                        "mem_requested",
                        "memcached_slab_mem_requested_bytes",
                        "Number of bytes of memory actual items take up within a slab.",
                    ),
                    (
                        "moves_to_cold",
                        "memcached_slab_items_moves_to_cold",
                        "Number of items moved from HOT or WARM into COLD.",
                    ),
                    (
                        "moves_to_warm",
                        "memcached_slab_items_moves_to_warm",
                        "Number of items moves from COLD into WARM.",
                    ),
                    (
                        "moves_within_lru",
                        "memcached_slab_items_moves_within_lru",
                        "Number of times active items were bumped within HOT or WARM.",
                    ),
                ] {
                    if let Some(v) = stats.get(key) {
                        metrics.push(Metric::sum_with_tags(
                            name,
                            desc,
                            *v,
                            tags!(
                                "slab" => slab
                            ),
                        ));
                    }
                }

                for (key, name, desc) in [
                    (
                        "number_hot",
                        "memcached_slab_hot_items",
                        "Number of items presently stored in the HOT LRU"
                    ),
                    (
                        "number_warm",
                        "memcached_slab_warm_items",
                        "Number of items presently stored in the WARM LRU"
                    ),
                    (
                        "number_cold",
                        "memcached_slab_cold_items",
                        "Number of items presently stored in the COLD LRU"
                    ),
                    (
                        "number_temp",
                        "memcached_slab_temporary_items",
                        "Number of items presently stored in the TEMPORARY LRU"
                    ),
                    (
                        "age_hot",
                        "memcached_slab_hot_age_seconds",
                        "Age of the oldest item in HOT LRU"
                    ),
                    (
                        "age_warm",
                        "memcached_slab_warm_age_seconds",
                        "Age of the oldest item in HOT LRU"
                    ),
                ] {
                    if let Some(v) = stats.get(key) {
                        metrics.push(Metric::sum_with_tags(
                            name,
                            desc,
                            *v,
                            tags!(
                                "slab" => slab
                            ),
                        ))
                    }
                }
            }
        }
        Err(ref err) => {}
    }

    metrics.extend_from_slice(&[
        Metric::gauge(
            "memcached_up",
            "Could the memcached server be reached.",
            if up { 1.0 } else { 0.0 },
        ),
        Metric::gauge(
            "memcached_scrape_duration",
            "",
            elapsed,
        ),
    ]);

    for metric in metrics.iter_mut() {
        metric.tags.insert("instance".to_string(), addr.to_string());
    }

    metrics
}

/// Stats is a type for storing current statistics of a Memcached server
#[derive(Default, Debug)]
struct Stats {
    version: String,
    libevent: String,
    // Stats are the top level key = value metrics from memcached
    stats: HashMap<String, f64>,
    // Slabs are indexed by slab ID. Each has a k/v store of metrics for
    // that slab
    slabs: HashMap<String, HashMap<String, f64>>,

    // Items are indexed by slab ID. Each ID has a k/v store of metrics for
    // items in that slab
    items: HashMap<String, HashMap<String, f64>>,
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
    #[snafu(display("parse slab failed {}", source))]
    ParseSlabFailed { source: std::num::ParseIntError },
}

async fn fetch_stats<'a, F, Fut>(
    addr: &'a str,
    query: F,
) -> Result<Stats, ParseError>
    where
        F: Fn(&'a str, &'a str) -> Fut,
        Fut: Future<Output=Result<String, std::io::Error>>
{
    let mut stats = Stats::default();
    for cmd in vec!["stats\r\n", "stats slabs\r\n", "stats items\r\n"] {
        let resp = query(addr, cmd).await
            .with_context(|| CommandExecFailed { cmd: cmd.to_string() })?;
        let mut lines = resp.as_str().lines();

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

            if parts[1] == "version" {
                stats.version = parts[2].to_string();
                continue;
            } else if parts[1] == "libevent" {
                stats.libevent = parts[2].to_string();
                continue;
            }

            let v = parts[2].parse()
                .context(InvalidValue)?;

            let subs = parts[1].split(':')
                .collect::<Vec<_>>();
            match subs.len() {
                1 => {
                    // Global stats
                    stats.stats.insert(parts[1].to_string(), v);
                }

                2 => {
                    // Slab stats
                    let mut slab = match stats.slabs.get_mut(subs[0]) {
                        Some(slab) => slab,
                        None => {
                            stats.slabs.insert(subs[0].to_string(), Default::default());
                            stats.slabs.get_mut(subs[0]).unwrap()
                        }
                    };

                    slab.insert(subs[1].to_string(), v);
                }

                3 => {
                    // Slab item stats
                    let mut item = match stats.items.get_mut(subs[1]) {
                        Some(item) => item,
                        None => {
                            stats.items.insert(subs[1].to_string(), Default::default());
                            stats.items.get_mut(subs[1]).unwrap()
                        }
                    };

                    item.insert(subs[2].to_string(), v);
                }

                _ => {}
            }
        }
    }

    Ok(stats)
}

// TODO: this implement is stupid, refactor should be done as soon as possible
async fn query(addr: &str, cmd: &str) -> Result<String, std::io::Error> {
    let socket = TcpStream::connect(addr).await?;
    let (mut reader, mut writer) = tokio::io::split(socket);

    writer.write_all(cmd.as_bytes()).await?;

    let mut resp = Vec::with_capacity(8 * 1024);
    let mut buf = [0u8; 4096];
    loop {
        let n = reader.read(&mut buf).await?;

        resp.extend_from_slice(&buf[..n]);

        if n == 0 || n < 4096 {
            break;
        }
    }

    Ok(String::from_utf8_lossy(resp.as_slice()).to_string())
}

#[cfg(test)]
mod tests {
    use std::io::BufReader;
    use testcontainers::Docker;
    use crate::sources::memcached::tests::memcached::Memcached;
    use super::*;

    async fn mock_query<'a>(_addr: &str, cmd: &str) -> Result<String, std::io::Error> {
        let path = match cmd {
            "stats\r\n" => "testdata/memcached/stats.txt",
            "stats slabs\r\n" => "testdata/memcached/slabs.txt",
            "stats items\r\n" => "testdata/memcached/items.txt",
            _ => panic!("unknown commands")
        };

        std::fs::read_to_string(path)
    }

    #[tokio::test]
    async fn test_parse() {
        let stats = fetch_stats("dummy", mock_query).await.unwrap();
        assert_eq!(stats.version, "1.6.12");
        assert_eq!(stats.libevent, "2.1.12-stable");

        assert_eq!(stats.stats.len(), 90);
        assert_eq!(*stats.stats.get("limit_maxbytes").unwrap(), 67108864.0);
        assert_eq!(*stats.stats.get("lru_crawler_running").unwrap(), 0.0);
        assert_eq!(*stats.stats.get("active_slabs").unwrap(), 1.0);
        assert_eq!(*stats.stats.get("total_malloced").unwrap(), 1048576.0);

        assert_eq!(*stats.slabs.get("1").unwrap().get("free_chunks").unwrap(), 10921.0);
        assert_eq!(*stats.slabs.get("1").unwrap().get("chunk_size").unwrap(), 96.0);

        assert_eq!(*stats.items.get("1").unwrap().get("mem_requested").unwrap(), 65.0);
        assert_eq!(*stats.items.get("1").unwrap().get("number").unwrap(), 1.0);
    }

    mod memcached {
        use std::collections::HashMap;
        use testcontainers::{Container, Docker, Image, WaitForMessage};

        const CONTAINER_IDENTIFIER: &str = "memcached";
        const DEFAULT_TAG: &str = "1.6.12-alpine3.14";

        #[derive(Debug, Clone, Default)]
        pub struct MemcachedArgs(Vec<String>);

        impl IntoIterator for MemcachedArgs {
            type Item = String;
            type IntoIter = std::vec::IntoIter<String>;

            fn into_iter(self) -> Self::IntoIter {
                self.0.into_iter()
            }
        }

        pub struct Memcached {
            arguments: MemcachedArgs,
            tag: String,
        }

        impl Default for Memcached {
            fn default() -> Self {
                Self {
                    arguments: MemcachedArgs(vec!["-vv".to_string()]),
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
                    .stderr
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

    async fn write_data(addr: &str, n: i32) {
        let mut socket = TcpStream::connect(addr).await.unwrap();
        let (mut reader, mut writer) = socket.split();

        for i in 0..n {
            let cmd = format!("set {} 0 0 5\nvalue\r\n", i + 1);

            writer.write_all(cmd.as_bytes()).await.unwrap();

            let mut buf = [0u8; 128];
            reader.read(&mut buf).await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_query() {
        let docker = testcontainers::clients::Cli::default();
        let image = Memcached::default();
        let service = docker.run(image);
        let host_port = service.get_host_port(11211).unwrap();
        let addr = format!("127.0.0.1:{}", host_port);

        write_data(&addr, 1000).await;

        let stats = fetch_stats(&addr, query).await.unwrap();

        assert_eq!(stats.stats.get("cmd_set").unwrap(), &1000.0);
    }
}