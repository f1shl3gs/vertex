mod client;

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::io::BufRead;
use std::time::Duration;

use bytes::{Buf, Bytes};
use client::{Client, RespErr};
use configurable::configurable_component;
use event::{tags, Metric};
use framework::config::{default_interval, DataType, Output, SourceConfig, SourceContext};
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::Source;
use once_cell::sync::Lazy;
use thiserror::Error;

static GAUGE_METRICS: Lazy<BTreeMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = BTreeMap::new();
    // # Server
    m.insert("uptime_in_seconds", "uptime_in_seconds");
    m.insert("process_id", "process_id");

    // # Clients
    m.insert("connected_clients", "connected_clients");
    m.insert("blocked_clients", "blocked_clients");
    m.insert("tracking_clients", "tracking_clients");

    // redis 2,3,4.x
    m.insert("client_longest_output_list", "client_longest_output_list");
    m.insert("client_biggest_input_buf", "client_biggest_input_buf");

    // the above two metrics were renamed in redis 5.x
    m.insert(
        "client_recent_max_output_buffer",
        "client_recent_max_output_buffer_bytes",
    );
    m.insert(
        "client_recent_max_input_buffer",
        "client_recent_max_input_buffer_bytes",
    );

    // # Memory
    m.insert("allocator_active", "allocator_active_bytes");
    m.insert("allocator_allocated", "allocator_allocated_bytes");
    m.insert("allocator_resident", "allocator_resident_bytes");
    m.insert("allocator_frag_ratio", "allocator_frag_ratio");
    m.insert("allocator_frag_bytes", "allocator_frag_bytes");
    m.insert("allocator_rss_ratio", "allocator_rss_ratio");
    m.insert("allocator_rss_bytes", "allocator_rss_bytes");

    m.insert("used_memory", "memory_used_bytes");
    m.insert("used_memory_rss", "memory_used_rss_bytes");
    m.insert("used_memory_peak", "memory_used_peak_bytes");
    m.insert("used_memory_lua", "memory_used_lua_bytes");
    m.insert("used_memory_overhead", "memory_used_overhead_bytes");
    m.insert("used_memory_startup", "memory_used_startup_bytes");
    m.insert("used_memory_dataset", "memory_used_dataset_bytes");
    m.insert("used_memory_scripts", "memory_used_scripts_bytes");
    m.insert("maxmemory", "memory_max_bytes");

    m.insert("mem_fragmentation_ratio", "mem_fragmentation_ratio");
    m.insert("mem_fragmentation_bytes", "mem_fragmentation_bytes");
    m.insert("mem_clients_slaves", "mem_clients_slaves");
    m.insert("mem_clients_normal", "mem_clients_normal");

    // https://github.com/antirez/redis/blob/17bf0b25c1171486e3a1b089f3181fff2bc0d4f0/src/evict.c#L349-L352
    // ... the sum of AOF and slaves buffer ....
    m.insert(
        "mem_not_counted_for_evict",
        "mem_not_counted_for_eviction_bytes",
    );

    m.insert("lazyfree_pending_objects", "lazyfree_pending_objects");
    m.insert("active_defrag_running", "active_defrag_running");

    m.insert("migrate_cached_sockets", "migrate_cached_sockets_total");

    m.insert("active_defrag_hits", "defrag_hits");
    m.insert("active_defrag_misses", "defrag_misses");
    m.insert("active_defrag_key_hits", "defrag_key_hits");
    m.insert("active_defrag_key_misses", "defrag_key_misses");

    // https://github.com/antirez/redis/blob/0af467d18f9d12b137af3b709c0af579c29d8414/src/expire.c#L297-L299
    m.insert(
        "expired_time_cap_reached_count",
        "expired_time_cap_reached_total",
    );

    // # Persistence
    m.insert("loading", "loading_dump_file");
    m.insert("rdb_changes_since_last_save", "rdb_changes_since_last_save");
    m.insert("rdb_bgsave_in_progress", "rdb_bgsave_in_progress");
    m.insert("rdb_last_save_time", "rdb_last_save_timestamp_seconds");
    m.insert("rdb_last_bgsave_status", "rdb_last_bgsave_status");
    m.insert("rdb_last_bgsave_time_sec", "rdb_last_bgsave_duration_sec");
    m.insert(
        "rdb_current_bgsave_time_sec",
        "rdb_current_bgsave_duration_sec",
    );
    m.insert("rdb_last_cow_size", "rdb_last_cow_size_bytes");
    m.insert("aof_enabled", "aof_enabled");
    m.insert("aof_rewrite_in_progress", "aof_rewrite_in_progress");
    m.insert("aof_rewrite_scheduled", "aof_rewrite_scheduled");
    m.insert("aof_last_rewrite_time_sec", "aof_last_rewrite_duration_sec");
    m.insert(
        "aof_current_rewrite_time_sec",
        "aof_current_rewrite_duration_sec",
    );
    m.insert("aof_last_cow_size", "aof_last_cow_size_bytes");
    m.insert("aof_current_size", "aof_current_size_bytes");
    m.insert("aof_base_size", "aof_base_size_bytes");
    m.insert("aof_pending_rewrite", "aof_pending_rewrite");
    m.insert("aof_buffer_length", "aof_buffer_length");
    m.insert("aof_rewrite_buffer_length", "aof_rewrite_buffer_length");
    m.insert("aof_pending_bio_fsync", "aof_pending_bio_fsync");
    m.insert("aof_delayed_fsync", "aof_delayed_fsync");
    m.insert("aof_last_bgrewrite_status", "aof_last_bgrewrite_status");
    m.insert("aof_last_write_status", "aof_last_write_status");
    m.insert("module_fork_in_progress", "module_fork_in_progress");
    m.insert("module_fork_last_cow_size", "module_fork_last_cow_size");

    // # Stats
    m.insert("pubsub_channels", "pubsub_channels");
    m.insert("pubsub_patterns", "pubsub_patterns");
    m.insert("latest_fork_usec", "latest_fork_usec");
    m.insert("instantaneous_ops_per_sec", "instantaneous_ops");

    // # Replication
    m.insert("connected_slaves", "connected_slaves");
    m.insert("repl_backlog_size", "replication_backlog_bytes");
    m.insert("repl_backlog_active", "repl_backlog_is_active");
    m.insert(
        "repl_backlog_first_byte_offset",
        "repl_backlog_first_byte_offset",
    );
    m.insert("repl_backlog_histlen", "repl_backlog_history_bytes");
    m.insert("master_repl_offset", "master_repl_offset");
    m.insert("second_repl_offset", "second_repl_offset");
    m.insert("slave_expires_tracked_keys", "slave_expires_tracked_keys");
    m.insert("slave_priority", "slave_priority");
    m.insert("sync_full", "replica_resyncs_full");
    m.insert("sync_partial_ok", "replica_partial_resync_accepted");
    m.insert("sync_partial_err", "replica_partial_resync_denied");

    // # Cluster
    m.insert("cluster_stats_messages_sent", "cluster_messages_sent_total");
    m.insert(
        "cluster_stats_messages_received",
        "cluster_messages_received_total",
    );

    // # Tile38
    // based on https://tile38.com/commands/server/
    m.insert("tile38_aof_size", "tile38_aof_size_bytes");
    m.insert("tile38_avg_item_size", "tile38_avg_item_size_bytes");
    m.insert("tile38_cpus", "tile38_cpus_total");
    m.insert("tile38_heap_released", "tile38_heap_released_bytes");
    m.insert("tile38_heap_size", "tile38_heap_size_bytes");
    m.insert("tile38_http_transport", "tile38_http_transport");
    m.insert("tile38_in_memory_size", "tile38_in_memory_size_bytes");
    m.insert("tile38_max_heap_size", "tile38_max_heap_size_bytes");
    m.insert("tile38_mem_alloc", "tile38_mem_alloc_bytes");
    m.insert("tile38_num_collections", "tile38_num_collections_total");
    m.insert("tile38_num_hooks", "tile38_num_hooks_total");
    m.insert("tile38_num_objects", "tile38_num_objects_total");
    m.insert("tile38_num_points", "tile38_num_points_total");
    m.insert("tile38_pointer_size", "tile38_pointer_size_bytes");
    m.insert("tile38_read_only", "tile38_read_only");
    m.insert("tile38_threads", "tile38_threads_total");

    // addtl. KeyDB metrics
    m.insert("server_threads", "server_threads_total");
    m.insert("long_lock_waits", "long_lock_waits_total");
    m.insert("current_client_thread", "current_client_thread");

    m
});

static COUNTER_METRICS: Lazy<BTreeMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = BTreeMap::new();

    m.insert("total_connections_received", "connections_received_total");
    m.insert("total_commands_processed", "commands_processed_total");

    m.insert("rejected_connections", "rejected_connections_total");
    m.insert("total_net_input_bytes", "net_input_bytes_total");
    m.insert("total_net_output_bytes", "net_output_bytes_total");

    m.insert("expired_keys", "expired_keys_total");
    m.insert("evicted_keys", "evicted_keys_total");
    m.insert("keyspace_hits", "keyspace_hits_total");
    m.insert("keyspace_misses", "keyspace_misses_total");

    m.insert("used_cpu_sys", "cpu_sys_seconds_total");
    m.insert("used_cpu_user", "cpu_user_seconds_total");
    m.insert("used_cpu_sys_children", "cpu_sys_children_seconds_total");
    m.insert("used_cpu_user_children", "cpu_user_children_seconds_total");

    m
});

#[derive(Debug, Error)]
enum ParseError {
    #[error("Parse integer failed, {0}")]
    Integer(#[from] std::num::ParseIntError),

    #[error("Parse float failed, {0}")]
    Float(#[from] std::num::ParseFloatError),
}

#[derive(Debug, Error)]
enum Error {
    #[error("Invalid slave line")]
    InvalidSlaveLine,

    #[error("Invalid command stats")]
    InvalidCommandStats,

    #[error("Invalid keyspace line")]
    InvalidKeyspaceLine,

    #[error("Parse error: {0}")]
    Parse(ParseError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("Redis error: {0}")]
    Resp(#[from] RespErr),
}

impl From<std::num::ParseIntError> for Error {
    fn from(err: std::num::ParseIntError) -> Self {
        Self::Parse(ParseError::Integer(err))
    }
}

impl From<std::num::ParseFloatError> for Error {
    fn from(err: std::num::ParseFloatError) -> Self {
        Self::Parse(ParseError::Float(err))
    }
}

#[configurable_component(source, name = "redis")]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Redis address
    #[configurable(required, format = "ip-address")]
    endpoint: String,

    /// Duration between each scrape.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    #[serde(default = "default_namespace")]
    namespace: Option<String>,

    #[serde(default)]
    user: Option<String>,

    #[serde(default)]
    password: Option<String>,
}

fn default_namespace() -> Option<String> {
    Some("redis".to_string())
}

#[async_trait::async_trait]
#[typetag::serde(name = "redis")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let src = RedisSource {
            url: self.endpoint.clone(),
            namespace: self.namespace.clone(),
            interval: self.interval,
            client_name: None,
        };

        Ok(Box::pin(src.run(cx.output, cx.shutdown)))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }
}

struct RedisSource {
    // user: Option<String>,
    // password: Option<String>,
    url: String,
    namespace: Option<String>,
    interval: Duration,

    client_name: Option<String>,
    // TODO: add TLS and timeouts
}

impl RedisSource {
    async fn run(self, mut output: Pipeline, mut shutdown: ShutdownSignal) -> Result<(), ()> {
        let mut ticker = tokio::time::interval(self.interval);

        loop {
            tokio::select! {
                biased;

                _ = &mut shutdown => break,
                _ = ticker.tick() => {}
            }

            let mut metrics = match self.collect().await {
                Err(err) => {
                    warn!(
                        message = "collect redis metrics failed",
                        %err
                    );

                    vec![Metric::gauge(
                        "up",
                        "Information about the Redis instance",
                        0,
                    )]
                }
                Ok(mut metrics) => {
                    metrics.push(Metric::gauge(
                        "up",
                        "Information about the Redis instance",
                        1,
                    ));

                    metrics
                }
            };

            let timestamp = chrono::Utc::now();
            metrics.iter_mut().for_each(|m| {
                m.timestamp = Some(timestamp);
                m.insert_tag("instance", &self.url);
                if let Some(ref namespace) = self.namespace {
                    m.set_name(format!("{}_{}", namespace, m.name()));
                }
            });

            if let Err(err) = output.send(metrics).await {
                error!(
                    message = "Error sending redis metrics",
                    %err,
                );

                return Err(());
            }
        }

        Ok(())
    }

    async fn collect(&self) -> Result<Vec<Metric>, Error> {
        let mut metrics = vec![];
        let mut cli = Client::connect(&self.url).await?;

        if let Some(ref name) = self.client_name {
            cli.query::<String>(&["client", "setname", name]).await?;
        }

        let mut db_count = match databases(&mut cli).await {
            Ok(n) => n,
            Err(err) => {
                debug!(message = "redis config get failed", ?err);
                0
            }
        };

        let infos = cli.query::<Bytes>(&["info", "all"]).await?;
        let infos = std::str::from_utf8(&infos).unwrap();

        if infos.contains("cluster_enabled:1") {
            match cluster_info(&mut cli).await {
                Ok(ms) => {
                    if !metrics.is_empty() {
                        metrics.extend(ms);
                    }

                    // in cluster mode Redis only supports one database so no extra DB
                    // number padding needed
                    db_count = 1;
                }
                Err(err) => {
                    warn!(
                        message = "Redis CLUSTER INFO failed",
                        ?err,
                        internal_log_rate_limit = true
                    );
                }
            }
        } else if db_count == 0 {
            // in non-cluster mode, if db_count is zero then "CONFIG" failed to retrieve a
            // valid number of databases and we use the Redis config default which is 16
            db_count = 16
        }

        // info metrics
        if let Ok(ms) = extract_info_metrics(infos, db_count) {
            metrics.extend(ms);
        }

        // latency
        if let Ok(ms) = latency_metrics(&mut cli).await {
            metrics.extend(ms);
        }

        if let Ok(ms) = slowlog_metrics(&mut cli).await {
            metrics.extend(ms);
        }

        //      Redis exporter provide this feature, but
        //      do we need it too? Under the hood, SELECT is used, which might
        //      hurt the performance of redis
        //
        //      if let Ok(ms) = extract_count_keys_metrics(&mut conn).await {
        //          metrics.extend(ms);
        //      }
        //

        // TODO： implement this
        // if infos.contains("# Sentinel") {
        //     if let Ok(ms) = extract_sentinel_metrics(&mut conn).await {
        //         metrics.extend(ms);
        //     }
        // }

        Ok(metrics)
    }
}

async fn databases(cli: &mut Client) -> Result<u64, Error> {
    let parts = cli.query::<Vec<String>>(&["config", "get", "*"]).await?;

    for pos in 0..parts.len() / 2 {
        let key = &parts[2 * pos];
        if key == "databases" {
            let value = &parts[2 * pos + 1];

            return value.parse::<u64>().map_err(|_err| {
                RespErr::ServerErr("invalid `databases` value".to_string()).into()
            });
        }
    }

    Ok(0)
}

async fn slowlog_metrics(cli: &mut Client) -> Result<Vec<Metric>, Error> {
    let mut metrics = vec![];

    match cli.query::<u64>(&["slowlog", "len"]).await {
        Ok(length) => {
            metrics.push(Metric::gauge("slowlog_length", "Total slowlog", length));
        }
        Err(err) => {
            warn!(message = "slowlog length query failed", ?err);
        }
    }

    let values: Vec<i64> = cli.query(&["slowlog", "get", "1"]).await?;

    let mut last_id: i64 = 0;
    let mut last_slow_execution_second: f64 = 0.0;
    if !values.is_empty() {
        last_id = values[0];
        if values.len() > 2 {
            last_slow_execution_second = values[2] as f64 / 1e6
        }
    }

    metrics.extend_from_slice(&[
        Metric::gauge("slowlog_last_id", "Last id of slowlog", last_id as f64),
        Metric::gauge(
            "last_slow_execution_duration_seconds",
            "The amount of time needed for last slow execution, in seconds",
            last_slow_execution_second,
        ),
    ]);

    Ok(metrics)
}

// https://redis.io/commands/latency-latest
async fn latency_metrics(cli: &mut Client) -> Result<Vec<Metric>, Error> {
    let mut metrics = vec![];
    let values: Vec<Vec<String>> = cli.query(&["latency", "latest"]).await?;

    for parts in values {
        let event = Cow::from(parts[0].clone());
        let spike_last = parts[1].parse::<f64>()?;
        let spike_duration = parts[2].parse::<f64>()?;

        metrics.extend_from_slice(&[
            Metric::gauge_with_tags(
                "latency_spike_last",
                "When the latency spike last occurred",
                spike_last,
                tags!(
                    "event_name" => event.clone()
                ),
            ),
            Metric::gauge_with_tags(
                "latency_spike_duration_seconds",
                "Length of the last latency spike in seconds",
                spike_duration / 1e3,
                tags!(
                    "event_name" => event
                ),
            ),
        ]);
    }

    Ok(metrics)
}

async fn cluster_info(cli: &mut Client) -> Result<Vec<Metric>, Error> {
    let keyword = "cluster_enabled:1".as_bytes();
    let infos = cli.query::<Bytes>(&["cluster", "info"]).await?;

    if (infos[..]).windows(keyword.len()).any(|p| p == keyword) {
        let mut metrics = vec![];

        infos
            .reader()
            .lines()
            .map_while(Result::ok)
            .for_each(|line| {
                let part = line.split(':').collect::<Vec<_>>();

                if part.len() != 2 {
                    return;
                }

                if !include_metric(part[0]) {
                    return;
                }

                if let Ok(m) = parse_and_generate(part[0], part[1]) {
                    metrics.push(m);
                }
            });

        Ok(metrics)
    } else {
        Ok(vec![])
    }
}

fn extract_info_metrics(infos: &str, dbcount: u64) -> Result<Vec<Metric>, std::io::Error> {
    let mut metrics = vec![];
    let mut kvs = BTreeMap::new();
    let mut handled_dbs = BTreeSet::new();
    let mut instance_infos = BTreeMap::new();
    let mut slave_infos: BTreeMap<String, String> = BTreeMap::new();
    let mut master_host = String::new();
    let mut master_port = String::new();
    let mut field_class = String::new();
    let instance_info_fields = [
        "role",
        "redis_version",
        "redis_build_id",
        "redis_mode",
        "os",
    ];
    let slave_info_fields = ["master_host", "master_port", "slave_read_only"];

    for line in infos.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(stripped) = line.strip_prefix("# ") {
            field_class = stripped.to_string();
            continue;
        }

        if line.len() < 2 || !line.contains(':') {
            continue;
        }

        let (key, value) = line.split_once(':').unwrap();

        kvs.insert(key.to_string(), value.to_string());
        if key == "master_host" {
            master_host = value.to_string();
        }

        if key == "master_port" {
            master_port = value.to_string();
        }

        if instance_info_fields.contains(&key) {
            instance_infos.insert(key.to_string(), value.to_string());
            continue;
        }

        if slave_info_fields.contains(&key) {
            slave_infos.insert(key.to_string(), value.to_string());
            continue;
        }

        match field_class.as_ref() {
            "Replication" => {
                if let Ok(ms) = handle_replication_metrics(&master_host, &master_port, key, value) {
                    metrics.extend(ms);
                }

                continue;
            }

            "Server" => {
                if let Ok(ms) = handle_server_metrics(key, value) {
                    metrics.extend(ms);
                }
            }

            "Commandstats" => {
                if let Ok(ms) = handle_command_stats(key, value) {
                    metrics.extend(ms);
                }

                continue;
            }

            "Keyspace" => {
                if let Ok((keys, expired, avg_ttl)) = parse_db_keyspace(key, value) {
                    let dbname = key.to_string();
                    let key = Cow::from(key.to_string());

                    metrics.extend_from_slice(&[
                        Metric::gauge_with_tags(
                            "db_keys",
                            "Total number of keys by DB",
                            keys,
                            tags!(
                                "db" => key.clone()
                            ),
                        ),
                        Metric::gauge_with_tags(
                            "db_keys_expiring",
                            "Total number of expiring keys by DB",
                            expired,
                            tags!(
                                "db" => key.clone()
                            ),
                        ),
                    ]);

                    if avg_ttl > -1.0 {
                        metrics.push(Metric::gauge_with_tags(
                            "db_avg_ttl_seconds",
                            "Avg TTL in seconds",
                            avg_ttl,
                            tags!(
                                "db" => key
                            ),
                        ));
                    }

                    handled_dbs.insert(dbname.clone());
                    continue;
                }
            }

            _ => {}
        }

        if !include_metric(key) {
            continue;
        }

        if let Ok(m) = parse_and_generate(key, value) {
            metrics.push(m);
        }
    }

    for i in 0..dbcount {
        let name = format!("db{}", i);
        if handled_dbs.get(name.as_str()).is_none() {
            let name = Cow::from(name);

            metrics.extend_from_slice(&[
                Metric::gauge_with_tags(
                    "db_keys",
                    "Total number of keys by DB",
                    0,
                    tags!(
                        "db" => name.clone()
                    ),
                ),
                Metric::gauge_with_tags(
                    "db_keys_expiring",
                    "Total number of expiring keys by DB",
                    0,
                    tags!(
                        "db" => name
                    ),
                ),
            ])
        }
    }

    let role = instance_infos.get("slave_info").map_or("", |v| v);

    metrics.push(Metric::gauge_with_tags(
        "instance_info",
        "Information about the Redis instance",
        1,
        tags!(
            "role" => role.to_string(),
            "redis_version" => instance_infos.get("redis_version").map_or("", |v| v).to_string(),
            "redis_build_id" => instance_infos.get("redis_mode").map_or("", |v| v).to_string(),
            "os" => instance_infos.get("os").map_or("", |v| v).to_string()
        ),
    ));

    if role == "slave" {
        metrics.push(Metric::gauge_with_tags(
            "slave_info",
            "Information about the Redis slave",
            1,
            slave_infos,
        ))
    }

    Ok(metrics)
}

// TODO: this function looks wired, we need to re-implement it
fn parse_and_generate(key: &str, value: &str) -> Result<Metric, Error> {
    let mut name = sanitize_metric_name(key);
    if let Some(new_name) = GAUGE_METRICS.get(name.as_str()) {
        name = new_name.to_string();
    }

    if let Some(new_name) = COUNTER_METRICS.get(name.as_str()) {
        name = new_name.to_string();
    }

    let mut val = match value {
        "ok" | "true" => 1.0,
        "err" | "fail" | "false" => 0.0,
        _ => value.parse().unwrap_or(0.0),
    };

    if name == "latest_fork_usec" {
        name = "latest_fork_seconds".to_string();
        val /= 1e6;
    }

    let metric = if let Some(name) = GAUGE_METRICS.get(name.as_str()) {
        Metric::gauge(name.to_string(), "", val)
    } else {
        Metric::sum(name.to_string(), "", val)
    };

    Ok(metric)
}

fn sanitize_metric_name(name: &str) -> String {
    let mut bytes = name.as_bytes().to_vec();
    for b in &mut bytes {
        if b.is_ascii_alphanumeric() {
            continue;
        }

        if *b == b'_' {
            continue;
        }

        *b = b'_';
    }

    String::from_utf8(bytes).unwrap_or_default()
}

fn handle_replication_metrics(
    host: &str,
    port: &str,
    key: &str,
    value: &str,
) -> Result<Vec<Metric>, Error> {
    // only slaves have this field
    if key == "master_link_status" {
        let v = match value {
            "up" => 1,
            _ => 0,
        };

        return Ok(vec![Metric::gauge_with_tags(
            "master_link_up",
            "",
            v,
            tags!(
                "master_host" => host.to_string(),
                "master_port" => port.to_string()
            ),
        )]);
    }

    match key {
        "master_last_io_seconds_ago" | "slave_repl_offset" | "master_sync_in_progress" => {
            let v = value.parse::<i32>()?;
            return Ok(vec![Metric::gauge_with_tags(
                key,
                "",
                v,
                tags!(
                    "master_host" => host.to_string(),
                    "master_port" => port.to_string()
                ),
            )]);
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
                "slave_ip" => ip.to_string(),
                "slave_port" => port.to_string(),
                "slave_state" => state.to_string()
            ),
        ));

        if lag > -1.0 {
            events.push(Metric::gauge_with_tags(
                "connected_slave_lag_seconds",
                "Lag of connected slave",
                lag,
                tags!(
                    "slave_ip" => ip.to_string(),
                    "slave_port" => port.to_string(),
                    "slave_state" => state.to_string()
                ),
            ))
        }

        return Ok(events);
    }

    Ok(vec![])
}

/// the slave line looks like
///
/// ```text
/// slave0:ip=10.254.11.1,port=6379,state=online,offset=1751844676,lag=0
/// slave1:ip=10.254.11.2,port=6379,state=online,offset=1751844222,lag=0
/// ```
fn parse_connected_slave_string<'a>(
    slave: &'a str,
    kvs: &'a str,
) -> Result<(f64, &'a str, &'a str, &'a str, f64), Error> {
    let mut connected_kvs = BTreeMap::new();

    if !validate_slave_line(slave) {
        return Err(Error::InvalidSlaveLine);
    }

    for part in kvs.split(',') {
        let kv = part.split('=').collect::<Vec<_>>();
        if kv.len() != 2 {
            return Err(Error::InvalidSlaveLine);
        }

        connected_kvs.insert(kv[0].to_string(), kv[1]);
    }

    let offset = connected_kvs
        .get("offset")
        .map(|v| v.parse::<f64>().unwrap_or(0.0))
        .unwrap();

    let lag = match connected_kvs.get("lag") {
        Some(text) => text.parse()?,
        _ => -1.0,
    };

    let ip = connected_kvs.get("ip").unwrap_or(&"");
    let port = connected_kvs.get("port").unwrap_or(&"");
    let state = connected_kvs.get("state").unwrap_or(&"");

    Ok((offset, ip, port, state, lag))
}

fn validate_slave_line(line: &str) -> bool {
    if !line.starts_with("slave") {
        return false;
    }

    if line.len() <= 5 {
        return false;
    }

    let c = line.as_bytes()[5];
    c.is_ascii_digit()
}

fn handle_server_metrics(key: &str, value: &str) -> Result<Vec<Metric>, Error> {
    if key == "uptime_in_seconds" {
        return Ok(vec![]);
    }

    let uptime = value.parse::<f64>()?;
    let now = std::time::SystemTime::now();
    let elapsed = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();

    Ok(vec![Metric::gauge(
        "start_time_seconds",
        "Start time of the Redis instance since unix epoch in seconds.",
        elapsed - uptime,
    )])
}

/*
    Format:
    cmdstat_get:calls=21,usec=175,usec_per_call=8.33
    cmdstat_set:calls=61,usec=3139,usec_per_call=51.46
    cmdstat_setex:calls=75,usec=1260,usec_per_call=16.80
*/
fn handle_command_stats(key: &str, value: &str) -> Result<Vec<Metric>, Error> {
    if !key.starts_with("cmdstat_") {
        return Err(Error::InvalidCommandStats);
    }

    let values = value.split(',').collect::<Vec<_>>();
    if values.len() < 3 {
        return Err(Error::InvalidCommandStats);
    }

    let calls = values[0]
        .strip_prefix("calls=")
        .ok_or(Error::InvalidCommandStats)?
        .parse::<f64>()?;
    let usec = values[1]
        .strip_prefix("usec=")
        .ok_or(Error::InvalidCommandStats)?
        .parse::<f64>()?;

    let cmd = Cow::from(key.strip_prefix("cmdstat_").unwrap().to_string());

    Ok(vec![
        Metric::sum_with_tags(
            "commands_total",
            "Total number of calls per command",
            calls,
            tags!(
                "cmd" => cmd.clone()
            ),
        ),
        Metric::sum_with_tags(
            "commands_duration_seconds_total",
            "Total amount of time in seconds spent per command",
            usec / 1e6,
            tags!(
                "cmd" => cmd
            ),
        ),
    ])
}

/*
    valid example: db0:keys=1,expires=0,avg_ttl=0
*/
fn parse_db_keyspace(key: &str, value: &str) -> Result<(f64, f64, f64), Error> {
    if !key.starts_with("db") {
        return Err(Error::InvalidKeyspaceLine);
    }

    let kvs = value.split(',').collect::<Vec<_>>();
    if kvs.len() != 3 && kvs.len() != 2 {
        return Err(Error::InvalidKeyspaceLine);
    }

    let keys = kvs[0]
        .strip_prefix("keys=")
        .ok_or(Error::InvalidKeyspaceLine)?
        .parse::<f64>()?;
    let expires = kvs[1]
        .strip_prefix("expires=")
        .ok_or(Error::InvalidKeyspaceLine)?
        .parse::<f64>()?;
    let mut avg_ttl = -1.0;
    if kvs.len() > 2 {
        avg_ttl = kvs[2]
            .strip_prefix("avg_ttl=")
            .ok_or(Error::InvalidKeyspaceLine)?
            .parse::<f64>()?;

        avg_ttl /= 1000.0
    }

    Ok((keys, expires, avg_ttl))
}

fn include_metric(s: &str) -> bool {
    if s.starts_with("db") || s.starts_with("cmdstat_") || s.starts_with("cluster_") {
        return true;
    }

    if GAUGE_METRICS.contains_key(s) {
        return true;
    }

    COUNTER_METRICS.contains_key(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }

    #[test]
    fn test_parse_db_keyspace() {
        let key = "db0";
        let value = "keys=100,expires=50,avg_ttl=5";
        let (keys, expires, avg_ttl) = parse_db_keyspace(key, value).unwrap();
        assert_eq!(keys, 100.0);
        assert_eq!(expires, 50.0);
        assert_eq!(avg_ttl, 5.0 / 1000.0);
    }

    #[test]
    fn parse_db_keyspace_without_avg_ttl() {
        let key = "db1";
        let value = "keys=100,expires=50";
        let (keys, expires, avg_ttl) = parse_db_keyspace(key, value).unwrap();
        assert_eq!(keys, 100.0);
        assert_eq!(expires, 50.0);
        assert_eq!(avg_ttl, -1.0);
    }
}

#[cfg(all(test, feature = "integration-tests-redis"))]
mod integration_tests {
    use super::*;
    use crate::testing::{ContainerBuilder, WaitFor};

    const REDIS_PORT: u16 = 6379;

    async fn write_testdata(cli: &mut Client) {
        for i in 0..100 {
            let key = format!("key_{}", i);
            let value = format!("value_{}", i);
            let resp = cli.query::<String>(&["set", &key, &value]).await.unwrap();
            assert_eq!(resp, "OK")
        }
    }

    #[tokio::test]
    async fn dump_config() {
        let container = ContainerBuilder::new("redis:5.0")
            .port(REDIS_PORT)
            .run()
            .unwrap();
        container
            .wait(WaitFor::Stdout("Ready to accept connections"))
            .unwrap();
        let url = container.get_host_port(REDIS_PORT).unwrap();

        let mut cli = Client::connect(url).await.unwrap();
        let resp: Vec<String> = cli.query(&["config", "get", "*"]).await.unwrap();

        assert_ne!(resp.len(), 0);
    }

    #[tokio::test]
    async fn test_slowlog() {
        let container = ContainerBuilder::new("redis:5.0")
            .port(REDIS_PORT)
            .run()
            .unwrap();
        container
            .wait(WaitFor::Stdout("Ready to accept connections"))
            .unwrap();
        let url = container.get_host_port(REDIS_PORT).unwrap();
        let mut cli = Client::connect(url).await.unwrap();

        write_testdata(&mut cli).await;

        let v = slowlog_metrics(&mut cli).await.unwrap();
        assert_eq!(v.len(), 3);
    }

    #[tokio::test]
    async fn test_latency_latest() {
        let container = ContainerBuilder::new("redis:5.0")
            .port(REDIS_PORT)
            .run()
            .unwrap();
        container
            .wait(WaitFor::Stdout("Ready to accept connections"))
            .unwrap();
        let url = container.get_host_port(REDIS_PORT).unwrap();
        let mut cli = Client::connect(url).await.unwrap();

        write_testdata(&mut cli).await;

        latency_metrics(&mut cli).await.unwrap();
    }
}
