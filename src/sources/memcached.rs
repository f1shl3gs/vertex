use std::borrow::Cow;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use chrono::Utc;
use configurable::configurable_component;
use event::tags::Key;
use event::{tags, Metric};
use framework::config::{default_interval, DataType, Output, SourceConfig, SourceContext};
use framework::Source;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const CLIENT_ERROR_PREFIX: &str = "CLIENT_ERROR";
const STAT_PREFIX: &str = "STAT";

const SLAB_KEY: Key = Key::from_static_str("slab");
const INSTANCE_KEY: Key = Key::from_static_str("instance");

/// Collect metrics from memcached servers.
#[configurable_component(source, name = "memcached")]
struct Config {
    /// The endpoint to Memcached servers.
    #[configurable(required, format = "ip-address", example = "127.0.0.1:3000")]
    endpoints: Vec<String>,

    /// Duration between each scrape.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "memcached")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let mut ticker = tokio::time::interval(self.interval);
        let endpoints = self.endpoints.clone();
        let SourceContext {
            mut output,
            mut shutdown,
            ..
        } = cx;

        Ok(Box::pin(async move {
            loop {
                tokio::select! {
                    biased;

                    _ = &mut shutdown => break,
                    _ = ticker.tick() => {}
                }

                let metrics = futures::future::join_all(endpoints.iter().map(|addr| gather(addr)))
                    .await
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                if let Err(err) = output.send(metrics).await {
                    error!(
                        message = "Error sending memcached metrics",
                        %err
                    );

                    return Err(());
                }
            }

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }
}

macro_rules! get_value {
    ($map:expr, $key:expr) => {
        *$map.get($key).unwrap_or(&0.0)
    };
}

macro_rules! get_bool_value {
    ($map:expr, $key: expr) => {
        match $map.get($key) {
            None => 0.0,
            Some(v) => {
                if v == "yes" {
                    1.0
                } else {
                    0.0
                }
            }
        }
    };
}

macro_rules! get_value_from_string {
    ($map:expr, $key: expr) => {
        match $map.get($key) {
            None => 0.0,
            Some(v) => v.parse::<f64>().unwrap_or(0.0),
        }
    };
}

async fn fetch_stats_metrics(addr: &str) -> Result<Vec<Metric>, ParseError> {
    let mut metrics = vec![];

    match fetch_stats(addr).await {
        Ok(Stats {
            version,
            libevent,
            stats,
            slabs,
            items,
        }) => {
            metrics.extend_from_slice(&[Metric::gauge_with_tags(
                "memcached_version",
                "The version of this memcached server.",
                1,
                tags!(
                    "version" => version,
                    "libevent" => libevent
                ),
            )]);

            for op in ["get", "delete", "inc", "decr", "cas", "touch"] {
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
                let slab = Cow::from(slab);
                for op in ["get", "delete", "incr", "decr", "cas", "touch"] {
                    metrics.push(Metric::sum_with_tags(
                        "memcached_slab_commands_total",
                        "Total number of all requests broken down by command (get, set, etc.) and status per slab.",
                        get_value!(stats, (op.to_owned() + "_hits").as_str()),
                        tags!(
                            SLAB_KEY => slab.clone(),
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
                        SLAB_KEY => slab.clone(),
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
                        SLAB_KEY => slab.clone(),
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
                            SLAB_KEY => slab.clone()
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "memcached_slab_chunks_per_page",
                        "Number of chunks within a single page for this slab class.",
                        get_value!(stats, "chunks_per_page"),
                        tags!(
                            SLAB_KEY => slab.clone()
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "memcached_slab_current_pages",
                        "Number of pages allocated to this slab class.",
                        get_value!(stats, "total_pages"),
                        tags!(
                            SLAB_KEY => slab.clone()
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "memcached_slab_current_chunks",
                        "Number of chunks allocated to this slab class.",
                        get_value!(stats, "total_chunks"),
                        tags!(
                            SLAB_KEY => slab.clone()
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "memcached_slab_chunks_used",
                        "Number of chunks allocated to an item",
                        get_value!(stats, "used_chunks"),
                        tags!(
                            SLAB_KEY => slab.clone()
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "memcached_slab_chunks_free",
                        "Number of chunks not yet allocated items",
                        get_value!(stats, "free_chunks"),
                        tags!(
                            SLAB_KEY => slab.clone()
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "memcached_slab_chunks_free_end",
                        "Number of free chunks at the end of the last allocated page",
                        get_value!(stats, "free_chunks_end"),
                        tags!(
                            SLAB_KEY => slab.clone()
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "memcached_slab_mem_requested_bytes",
                        "Number of bytes of memory actual items take up within a slab",
                        get_value!(stats, "mem_requested"),
                        tags!(
                            SLAB_KEY => slab.clone()
                        ),
                    ),
                ]);
            }

            for (slab, stats) in items {
                let slab = Cow::from(slab);

                metrics.extend_from_slice(&[
                    Metric::gauge_with_tags(
                        "memcached_slab_current_items",
                        "Number of items currently stored in this slab class",
                        get_value!(stats, "number"),
                        tags!(
                            SLAB_KEY => slab.clone()
                        ),
                    ),
                    Metric::gauge_with_tags(
                        "memcached_slab_items_age_seconds",
                        "Number of seconds the oldest item has been in the slab class",
                        get_value!(stats, "age"),
                        tags!(
                            SLAB_KEY => slab.clone()
                        ),
                    ),
                    Metric::sum_with_tags(
                        "memcached_slab_lru_hits_total",
                        "Number of get_hits to the LRU",
                        get_value!(stats, "hits_to_hot"),
                        tags!(
                            SLAB_KEY => slab.clone(),
                            "lru" => "hot"
                        ),
                    ),
                    Metric::sum_with_tags(
                        "memcached_slab_lru_hits_total",
                        "Number of get_hits to the LRU",
                        get_value!(stats, "hits_to_warm"),
                        tags!(
                            SLAB_KEY => slab.clone(),
                            "lru" => "warm"
                        ),
                    ),
                    Metric::sum_with_tags(
                        "memcached_slab_lru_hits_total",
                        "Number of get_hits to the LRU",
                        get_value!(stats, "hits_to_cold"),
                        tags!(
                            SLAB_KEY => slab.clone(),
                            "lru" => "cold"
                        ),
                    ),
                    Metric::sum_with_tags(
                        "memcached_slab_lru_hits_total",
                        "Number of get_hits to the LRU",
                        get_value!(stats, "hits_to_temporary"),
                        tags!(
                            SLAB_KEY => slab.clone(),
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
                                SLAB_KEY => slab.clone()
                            ),
                        ));
                    }
                }

                for (key, name, desc) in [
                    (
                        "number_hot",
                        "memcached_slab_hot_items",
                        "Number of items presently stored in the HOT LRU",
                    ),
                    (
                        "number_warm",
                        "memcached_slab_warm_items",
                        "Number of items presently stored in the WARM LRU",
                    ),
                    (
                        "number_cold",
                        "memcached_slab_cold_items",
                        "Number of items presently stored in the COLD LRU",
                    ),
                    (
                        "number_temp",
                        "memcached_slab_temporary_items",
                        "Number of items presently stored in the TEMPORARY LRU",
                    ),
                    (
                        "age_hot",
                        "memcached_slab_hot_age_seconds",
                        "Age of the oldest item in HOT LRU",
                    ),
                    (
                        "age_warm",
                        "memcached_slab_warm_age_seconds",
                        "Age of the oldest item in HOT LRU",
                    ),
                ] {
                    if let Some(v) = stats.get(key) {
                        metrics.push(Metric::sum_with_tags(
                            name,
                            desc,
                            *v,
                            tags!(
                                SLAB_KEY => slab.clone()
                            ),
                        ))
                    }
                }
            }

            Ok(metrics)
        }
        Err(err) => {
            warn!(
                message = "Fetch stats failed",
                addr = addr,
                %err
            );

            Err(err)
        }
    }
}

async fn fetch_settings_metric(addr: &str) -> Result<Vec<Metric>, ParseError> {
    let mut metrics = vec![];

    match stats_settings(addr).await {
        Ok(stats) => {
            if let Some(v) = stats.get("maxconns") {
                if let Ok(v) = v.parse::<f64>() {
                    metrics.push(Metric::gauge(
                        "memcached_max_connections",
                        "Maximum number of clients allowed",
                        v,
                    ));
                }
            }

            if let Some(value) = stats.get("lru_crawler") {
                if value == "yes" {
                    metrics.extend_from_slice(&[
                        Metric::gauge(
                            "memcached_lru_crawler_enabled",
                            "Whether the LRU crawler is enabled",
                            get_bool_value!(stats, "lru_crawler"),
                        ),
                        Metric::gauge(
                            "memcached_lru_crawler_sleep",
                            "Microseconds to sleep between LRU crawls",
                            get_value_from_string!(stats, "lru_crawler_sleep"),
                        ),
                        Metric::gauge(
                            "memcached_lru_crawler_to_crawl",
                            "Max items to crawl per slab per run",
                            get_value_from_string!(stats, "lru_crawler_tocrawl"),
                        ),
                        Metric::gauge(
                            "memcached_lru_crawler_maintainer_thread",
                            "Split LRU mode and backgroud threads",
                            get_bool_value!(stats, "lru_maintainer_thread"),
                        ),
                        Metric::gauge(
                            "memcached_lru_crawler_hot_percent",
                            "Percent of slab memory reserved for HOT LRU",
                            get_value_from_string!(stats, "hot_lru_pct"),
                        ),
                        Metric::gauge(
                            "memcached_lru_crawler_warm_percent",
                            "Percent of slab memory reserved for WARM LRU",
                            get_value_from_string!(stats, "warm_lru_pct"),
                        ),
                        Metric::gauge(
                            "memcached_lru_crawler_hot_max_factor",
                            "Set idle age of HOT LRU to COLD age * this",
                            get_value_from_string!(stats, "hot_max_factor"),
                        ),
                        Metric::gauge(
                            "memcached_lru_crawler_warm_max_factor",
                            "Set idle age of WARM LRU to COLD age * this",
                            get_value_from_string!(stats, "warm_max_factor"),
                        ),
                    ])
                }
            }

            Ok(metrics)
        }
        Err(err) => {
            warn!(
                message = "Fetch stats settings failed",
                addr = addr,
                %err
            );

            Err(err)
        }
    }
}

async fn gather(addr: &str) -> Vec<Metric> {
    let start = Instant::now();

    let (stats, settings) =
        futures::future::join(fetch_stats_metrics(addr), fetch_settings_metric(addr)).await;

    let up = stats.is_ok() && settings.is_ok();
    let mut metrics = stats.unwrap_or_default();
    metrics.extend(settings.unwrap_or_default());

    metrics.extend_from_slice(&[
        Metric::gauge(
            "memcached_up",
            "Could the memcached server be reached.",
            if up { 1.0 } else { 0.0 },
        ),
        Metric::gauge(
            "memcached_scrape_duration_seconds",
            "",
            start.elapsed().as_secs_f64(),
        ),
    ]);

    let now = Utc::now();
    for metric in metrics.iter_mut() {
        metric.timestamp = Some(now);
        metric.insert_tag(INSTANCE_KEY, addr.to_string());
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

impl Stats {
    fn append(&mut self, input: &str) -> Result<(), ParseError> {
        input.lines().try_for_each(|line| {
            if line.starts_with(CLIENT_ERROR_PREFIX) {
                // TODO: more error context
                return Err(ParseError::ClientError);
            }

            if !line.starts_with(STAT_PREFIX) {
                return Ok(());
            }

            let parts = line.split_ascii_whitespace().collect::<Vec<_>>();
            if parts.len() != 3 {
                return Ok(());
            }

            if parts[1] == "version" {
                self.version = parts[2].to_string();
                return Ok(());
            } else if parts[1] == "libevent" {
                self.libevent = parts[2].to_string();
                return Ok(());
            }

            let v = parts[2].parse().map_err(ParseError::InvalidValue)?;

            let subs = parts[1].split(':').collect::<Vec<_>>();
            match subs.len() {
                1 => {
                    // Global stats
                    self.stats.insert(parts[1].to_string(), v);
                }

                2 => {
                    // Slab stats
                    let slab = match self.slabs.get_mut(subs[0]) {
                        Some(slab) => slab,
                        None => self
                            .slabs
                            .entry(subs[0].to_string())
                            .or_insert(Default::default()),
                    };

                    slab.insert(subs[1].to_string(), v);
                }

                3 => {
                    // Slab item stats
                    let item = match self.items.get_mut(subs[1]) {
                        Some(item) => item,
                        None => {
                            self.items.insert(subs[1].to_string(), Default::default());
                            self.items.get_mut(subs[1]).unwrap()
                        }
                    };

                    item.insert(subs[2].to_string(), v);
                }

                _ => {}
            }

            Ok(())
        })
    }
}

#[derive(Debug, Error)]
enum ParseError {
    #[error("invalid value found: {0}")]
    InvalidValue(std::num::ParseFloatError),
    #[error("command \"{cmd}\" execute failed: {err}")]
    CommandExecFailed { cmd: String, err: std::io::Error },
    #[error("client error")]
    ClientError,
}

async fn fetch_stats(addr: &str) -> Result<Stats, ParseError> {
    let mut stats = Stats::default();
    for cmd in ["stats\r\n", "stats slabs\r\n", "stats items\r\n"] {
        let resp = query(addr, cmd)
            .await
            .map_err(|err| ParseError::CommandExecFailed {
                cmd: cmd.to_string(),
                err,
            })?;

        stats.append(&resp)?;
    }

    Ok(stats)
}

async fn stats_settings(addr: &str) -> Result<HashMap<String, String>, ParseError> {
    let resp: String =
        query(addr, "stats settings\r\n")
            .await
            .map_err(|err| ParseError::CommandExecFailed {
                cmd: "stats settings".to_string(),
                err,
            })?;

    Ok(parse_stats_settings(&resp))
}

fn parse_stats_settings(input: &str) -> HashMap<String, String> {
    let mut stats = HashMap::with_capacity(96);

    input.lines().for_each(|line| {
        if !line.starts_with(STAT_PREFIX) {
            return;
        }

        let parts = line.split_ascii_whitespace().collect::<Vec<_>>();
        if parts.len() != 3 {
            return;
        }

        stats.insert(parts[1].to_string(), parts[2].to_string());
    });

    stats
}

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
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }

    #[tokio::test]
    async fn test_parse_stats() {
        let mut stats = Stats::default();
        let data = std::fs::read_to_string("tests/fixtures/memcached/stats.txt").unwrap();
        stats.append(&data).unwrap();
        let data = std::fs::read_to_string("tests/fixtures/memcached/slabs.txt").unwrap();
        stats.append(&data).unwrap();
        let data = std::fs::read_to_string("tests/fixtures/memcached/items.txt").unwrap();
        stats.append(&data).unwrap();

        assert_eq!(stats.version, "1.6.12");
        assert_eq!(stats.libevent, "2.1.12-stable");

        assert_eq!(*stats.stats.get("cmd_set").unwrap(), 100.0);
        assert_eq!(*stats.stats.get("limit_maxbytes").unwrap(), 67108864.0);
        assert_eq!(*stats.stats.get("lru_crawler_running").unwrap(), 0.0);
        assert_eq!(*stats.stats.get("active_slabs").unwrap(), 1.0);
        assert_eq!(*stats.stats.get("total_malloced").unwrap(), 1048576.0);

        assert_eq!(
            *stats.slabs.get("1").unwrap().get("free_chunks").unwrap(),
            10921.0
        );
        assert_eq!(
            *stats.slabs.get("1").unwrap().get("chunk_size").unwrap(),
            96.0
        );

        assert_eq!(
            *stats.items.get("1").unwrap().get("mem_requested").unwrap(),
            65.0
        );
        assert_eq!(*stats.items.get("1").unwrap().get("number").unwrap(), 1.0);
    }

    #[tokio::test]
    async fn test_parse_stats_settings() {
        let data = std::fs::read_to_string("tests/fixtures/memcached/settings.txt").unwrap();

        let stats = parse_stats_settings(&data);
        assert_eq!(stats.get("chunk_size").unwrap(), "48");
        assert_eq!(stats.get("umask").unwrap(), "700");
        assert_eq!(stats.get("binding_protocol").unwrap(), "auto-negotiate");
        assert_eq!(stats.get("warm_max_factor").unwrap(), "2.00");
        assert_eq!(stats.get("ssl_min_version").unwrap(), "tlsv1.2");
        assert_eq!(stats.get("memory_file").unwrap(), "(null)");
        assert_eq!(stats.get("stat_key_prefix").unwrap(), ":");
    }
}

#[cfg(all(test, feature = "integration-tests-memcached"))]
mod integration_tests {
    use super::*;
    use crate::testing::ContainerBuilder;

    async fn write_data(addr: &str, n: i32) {
        let mut socket = TcpStream::connect(addr).await.unwrap();
        let (mut reader, mut writer) = socket.split();

        for i in 0..n {
            let cmd = format!("set {} 0 0 5\nvalue\r\n", i + 1);

            writer.write_all(cmd.as_bytes()).await.unwrap();

            let mut buf = [0u8; 128];
            let _n = reader.read(&mut buf).await.unwrap();
        }
    }

    #[tokio::test]
    async fn query_stats() {
        let container = ContainerBuilder::new("memcached:1.6.12-alpine3.14")
            .port(11211)
            .run()
            .unwrap();

        let addr = container.get_host_port(11211).unwrap();

        write_data(&addr, 1000).await;

        let stats = fetch_stats(&addr).await.unwrap();
        assert_eq!(stats.stats.get("cmd_set").unwrap(), &1000.0);
        assert_eq!(stats.stats.get("cmd_get").unwrap(), &0.0);

        let stats = stats_settings(&addr).await.unwrap();
        assert_eq!(stats.get("temporary_ttl").unwrap(), "61");
        assert_eq!(stats.get("warm_max_factor").unwrap(), "2.00");
        assert_eq!(stats.get("binding_protocol").unwrap(), "auto-negotiate");
        assert_eq!(stats.get("ext_wbuf_size").unwrap(), "4194304");
    }
}
