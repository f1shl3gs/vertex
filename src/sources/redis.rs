use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use event::{tags, Metric};
use framework::config::{
    default_interval, deserialize_duration, serialize_duration, DataType, GenerateConfig, Output,
    SourceConfig, SourceContext, SourceDescription,
};
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::Source;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use tokio_stream::wrappers::IntervalStream;

lazy_static!(
    static ref GAUGE_METRICS: BTreeMap<String, &'static str> = {
        let mut m = BTreeMap::new();
        // # Server
        m.insert("uptime_in_seconds".to_string(), "uptime_in_seconds");
        m.insert("process_id".to_string(),        "process_id");

        // # Clients
        m.insert("connected_clients".to_string(), "connected_clients");
        m.insert("blocked_clients".to_string(),   "blocked_clients");
        m.insert("tracking_clients".to_string(),  "tracking_clients");

        // redis 2,3,4.x
        m.insert("client_longest_output_list".to_string(), "client_longest_output_list");
        m.insert("client_biggest_input_buf".to_string(),   "client_biggest_input_buf");

        // the above two metrics were renamed in redis 5.x
        m.insert("client_recent_max_output_buffer".to_string(), "client_recent_max_output_buffer_bytes");
        m.insert("client_recent_max_input_buffer".to_string(),  "client_recent_max_input_buffer_bytes");

        // # Memory
        m.insert("allocator_active".to_string(),     "allocator_active_bytes");
        m.insert("allocator_allocated".to_string(),  "allocator_allocated_bytes");
        m.insert("allocator_resident".to_string(),   "allocator_resident_bytes");
        m.insert("allocator_frag_ratio".to_string(), "allocator_frag_ratio");
        m.insert("allocator_frag_bytes".to_string(), "allocator_frag_bytes");
        m.insert("allocator_rss_ratio".to_string(),  "allocator_rss_ratio");
        m.insert("allocator_rss_bytes".to_string(),  "allocator_rss_bytes");

        m.insert("used_memory".to_string(),          "memory_used_bytes");
        m.insert("used_memory_rss".to_string(),      "memory_used_rss_bytes");
        m.insert("used_memory_peak".to_string(),     "memory_used_peak_bytes");
        m.insert("used_memory_lua".to_string(),      "memory_used_lua_bytes");
        m.insert("used_memory_overhead".to_string(), "memory_used_overhead_bytes");
        m.insert("used_memory_startup".to_string(),  "memory_used_startup_bytes");
        m.insert("used_memory_dataset".to_string(),  "memory_used_dataset_bytes");
        m.insert("used_memory_scripts".to_string(),  "memory_used_scripts_bytes");
        m.insert("maxmemory".to_string(),            "memory_max_bytes");

        m.insert("mem_fragmentation_ratio".to_string(), "mem_fragmentation_ratio");
        m.insert("mem_fragmentation_bytes".to_string(), "mem_fragmentation_bytes");
        m.insert("mem_clients_slaves".to_string(),      "mem_clients_slaves");
        m.insert("mem_clients_normal".to_string(),      "mem_clients_normal");

        // https://github.com/antirez/redis/blob/17bf0b25c1171486e3a1b089f3181fff2bc0d4f0/src/evict.c#L349-L352
        // ... the sum of AOF and slaves buffer ....
        m.insert("mem_not_counted_for_evict".to_string(), "mem_not_counted_for_eviction_bytes");

        m.insert("lazyfree_pending_objects".to_string(), "lazyfree_pending_objects");
        m.insert("active_defrag_running".to_string(),    "active_defrag_running");

        m.insert("migrate_cached_sockets".to_string(), "migrate_cached_sockets_total");

        m.insert("active_defrag_hits".to_string(),       "defrag_hits");
        m.insert("active_defrag_misses".to_string(),     "defrag_misses");
        m.insert("active_defrag_key_hits".to_string(),   "defrag_key_hits");
        m.insert("active_defrag_key_misses".to_string(), "defrag_key_misses");

        // https://github.com/antirez/redis/blob/0af467d18f9d12b137af3b709c0af579c29d8414/src/expire.c#L297-L299
        m.insert("expired_time_cap_reached_count".to_string(), "expired_time_cap_reached_total");

        // # Persistence
        m.insert("loading".to_string(),                      "loading_dump_file");
        m.insert("rdb_changes_since_last_save".to_string(),  "rdb_changes_since_last_save");
        m.insert("rdb_bgsave_in_progress".to_string(),       "rdb_bgsave_in_progress");
        m.insert("rdb_last_save_time".to_string(),           "rdb_last_save_timestamp_seconds");
        m.insert("rdb_last_bgsave_status".to_string(),       "rdb_last_bgsave_status");
        m.insert("rdb_last_bgsave_time_sec".to_string(),     "rdb_last_bgsave_duration_sec");
        m.insert("rdb_current_bgsave_time_sec".to_string(),  "rdb_current_bgsave_duration_sec");
        m.insert("rdb_last_cow_size".to_string(),            "rdb_last_cow_size_bytes");
        m.insert("aof_enabled".to_string(),                  "aof_enabled");
        m.insert("aof_rewrite_in_progress".to_string(),      "aof_rewrite_in_progress");
        m.insert("aof_rewrite_scheduled".to_string(),        "aof_rewrite_scheduled");
        m.insert("aof_last_rewrite_time_sec".to_string(),    "aof_last_rewrite_duration_sec");
        m.insert("aof_current_rewrite_time_sec".to_string(), "aof_current_rewrite_duration_sec");
        m.insert("aof_last_cow_size".to_string(),            "aof_last_cow_size_bytes");
        m.insert("aof_current_size".to_string(),             "aof_current_size_bytes");
        m.insert("aof_base_size".to_string(),                "aof_base_size_bytes");
        m.insert("aof_pending_rewrite".to_string(),          "aof_pending_rewrite");
        m.insert("aof_buffer_length".to_string(),            "aof_buffer_length");
        m.insert("aof_rewrite_buffer_length".to_string(),    "aof_rewrite_buffer_length");
        m.insert("aof_pending_bio_fsync".to_string(),        "aof_pending_bio_fsync");
        m.insert("aof_delayed_fsync".to_string(),            "aof_delayed_fsync");
        m.insert("aof_last_bgrewrite_status".to_string(),    "aof_last_bgrewrite_status");
        m.insert("aof_last_write_status".to_string(),        "aof_last_write_status");
        m.insert("module_fork_in_progress".to_string(),      "module_fork_in_progress");
        m.insert("module_fork_last_cow_size".to_string(),    "module_fork_last_cow_size");

        // # Stats
        m.insert("pubsub_channels".to_string(),           "pubsub_channels");
        m.insert("pubsub_patterns".to_string(),           "pubsub_patterns");
        m.insert("latest_fork_usec".to_string(),          "latest_fork_usec");
        m.insert("instantaneous_ops_per_sec".to_string(), "instantaneous_ops");

        // # Replication
        m.insert("connected_slaves".to_string(),               "connected_slaves");
        m.insert("repl_backlog_size".to_string(),              "replication_backlog_bytes");
        m.insert("repl_backlog_active".to_string(),            "repl_backlog_is_active");
        m.insert("repl_backlog_first_byte_offset".to_string(), "repl_backlog_first_byte_offset");
        m.insert("repl_backlog_histlen".to_string(),           "repl_backlog_history_bytes");
        m.insert("master_repl_offset".to_string(),             "master_repl_offset");
        m.insert("second_repl_offset".to_string(),             "second_repl_offset");
        m.insert("slave_expires_tracked_keys".to_string(),     "slave_expires_tracked_keys");
        m.insert("slave_priority".to_string(),                 "slave_priority");
        m.insert("sync_full".to_string(),                      "replica_resyncs_full");
        m.insert("sync_partial_ok".to_string(),                "replica_partial_resync_accepted");
        m.insert("sync_partial_err".to_string(),               "replica_partial_resync_denied");

        // # Cluster
        m.insert("cluster_stats_messages_sent".to_string(),     "cluster_messages_sent_total");
        m.insert("cluster_stats_messages_received".to_string(), "cluster_messages_received_total");

        // # Tile38
        // based on https://tile38.com/commands/server/
        m.insert("tile38_aof_size".to_string(),        "tile38_aof_size_bytes");
        m.insert("tile38_avg_item_size".to_string(),   "tile38_avg_item_size_bytes");
        m.insert("tile38_cpus".to_string(),            "tile38_cpus_total");
        m.insert("tile38_heap_released".to_string(),   "tile38_heap_released_bytes");
        m.insert("tile38_heap_size".to_string(),       "tile38_heap_size_bytes");
        m.insert("tile38_http_transport".to_string(),  "tile38_http_transport");
        m.insert("tile38_in_memory_size".to_string(),  "tile38_in_memory_size_bytes");
        m.insert("tile38_max_heap_size".to_string(),   "tile38_max_heap_size_bytes");
        m.insert("tile38_mem_alloc".to_string(),       "tile38_mem_alloc_bytes");
        m.insert("tile38_num_collections".to_string(), "tile38_num_collections_total");
        m.insert("tile38_num_hooks".to_string(),       "tile38_num_hooks_total");
        m.insert("tile38_num_objects".to_string(),     "tile38_num_objects_total");
        m.insert("tile38_num_points".to_string(),      "tile38_num_points_total");
        m.insert("tile38_pointer_size".to_string(),    "tile38_pointer_size_bytes");
        m.insert("tile38_read_only".to_string(),       "tile38_read_only");
        m.insert("tile38_threads".to_string(),         "tile38_threads_total");

        // addtl. KeyDB metrics
        m.insert("server_threads".to_string(),        "server_threads_total");
        m.insert("long_lock_waits".to_string(),       "long_lock_waits_total");
        m.insert("current_client_thread".to_string(), "current_client_thread");

        m
    };

    static ref COUNTER_METRICS: BTreeMap<String, &'static str> = {
        let mut m = BTreeMap::new();

        m.insert("total_connections_received".to_string(), "connections_received_total");
        m.insert("total_commands_processed".to_string(),   "commands_processed_total");

        m.insert("rejected_connections".to_string(),   "rejected_connections_total");
        m.insert("total_net_input_bytes".to_string(),  "net_input_bytes_total");
        m.insert("total_net_output_bytes".to_string(), "net_output_bytes_total");

        m.insert("expired_keys".to_string(),    "expired_keys_total");
        m.insert("evicted_keys".to_string(),    "evicted_keys_total");
        m.insert("keyspace_hits".to_string(),   "keyspace_hits_total");
        m.insert("keyspace_misses".to_string(), "keyspace_misses_total");

        m.insert("used_cpu_sys".to_string(),           "cpu_sys_seconds_total");
        m.insert("used_cpu_user".to_string(),          "cpu_user_seconds_total");
        m.insert("used_cpu_sys_children".to_string(), "cpu_sys_children_seconds_total");
        m.insert("used_cpu_user_children".to_string(), "cpu_user_children_seconds_total");

        m
    };
);

#[derive(Debug, Snafu)]
enum ParseError {
    #[snafu(display("Parse integer failed, {}", source))]
    Int { source: std::num::ParseIntError },

    #[snafu(display("Parse float failed, {}", source))]
    Float { source: std::num::ParseFloatError },
}

#[derive(Debug, Snafu)]
enum Error {
    #[snafu(display("Invalid data: {}", desc))]
    InvalidData { desc: String },

    #[snafu(display("Invalid slave line"))]
    InvalidSlaveLine,

    #[snafu(display("Invalid command stats"))]
    InvalidCommandStats,

    #[snafu(display("Invalid keyspace line"))]
    InvalidKeyspaceLine,

    #[snafu(display("Redis error: {}", source))]
    Redis { source: redis::RedisError },

    #[snafu(display("Parse error: {}", source))]
    Parse { source: ParseError },
}

impl From<redis::RedisError> for Error {
    fn from(source: redis::RedisError) -> Self {
        Self::Redis { source }
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(source: std::num::ParseIntError) -> Self {
        Self::Parse {
            source: ParseError::Int { source },
        }
    }
}

impl From<std::num::ParseFloatError> for Error {
    fn from(source: std::num::ParseFloatError) -> Self {
        Self::Parse {
            source: ParseError::Float { source },
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RedisSourceConfig {
    // something looks like this, e.g. redis://host:port/db
    url: String,

    #[serde(default = "default_interval")]
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
    interval: Duration,

    #[serde(default = "default_namespace")]
    namespace: Option<String>,

    #[serde(default)]
    user: Option<String>,

    #[serde(default)]
    password: Option<String>,
}

impl GenerateConfig for RedisSourceConfig {
    fn generate_config() -> String {
        r#"
# The endpoints to connect to redis
#
endpoint: redis://localhost:6379

# The interval between scrapes.
#
# interval: 15s

# TODO: example for configuring "user" and password
"#
        .into()
    }
}

inventory::submit! {
    SourceDescription::new::<RedisSourceConfig>("redis")
}

fn default_namespace() -> Option<String> {
    Some("redis".to_string())
}

#[async_trait::async_trait]
#[typetag::serde(name = "redis")]
impl SourceConfig for RedisSourceConfig {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let src = RedisSource::from(self);

        Ok(Box::pin(src.run(cx.output, cx.shutdown)))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }

    fn source_type(&self) -> &'static str {
        "redis"
    }
}

struct RedisSource {
    // user: Option<String>,
    // password: Option<String>,
    url: String,
    namespace: Option<String>,
    interval: std::time::Duration,

    client_name: Option<String>,
    // TODO: add TLS and timeouts
}

impl RedisSource {
    fn from(conf: &RedisSourceConfig) -> Self {
        Self {
            url: conf.url.clone(),
            namespace: conf.namespace.clone(),
            interval: conf.interval,
            client_name: None,
        }
    }

    async fn run(self, mut output: Pipeline, shutdown: ShutdownSignal) -> Result<(), ()> {
        let mut ticker =
            IntervalStream::new(tokio::time::interval(self.interval)).take_until(shutdown);

        while ticker.next().await.is_some() {
            let mut metrics = match self.gather().await {
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

    async fn gather(&self) -> Result<Vec<Metric>, Error> {
        let mut metrics = vec![];
        let cli = redis::Client::open(self.url.as_str())?;
        let mut conn = cli.get_async_connection().await?;

        if let Some(ref client_name) = self.client_name {
            redis::cmd("CLIENT")
                .arg("SETNAME")
                .arg(client_name)
                .query_async(&mut conn)
                .await?;
        }

        let mut db_count = match query_databases(&mut conn).await {
            Ok(n) => n,
            Err(err) => {
                debug!(message = "redis config get failed", ?err);
                0
            }
        };

        let infos = query_infos(&mut conn).await?;

        if infos.contains("cluster_enabled:1") {
            match cluster_info(&mut conn).await {
                Ok(info) => {
                    if let Ok(ms) = extract_cluster_info_metrics(info) {
                        metrics.extend(ms);
                    }

                    // in cluster mode Redis only supports one database so no extra DB number padding needed
                    db_count = 1;
                }
                Err(err) => {
                    error!(
                        message = "Redis CLUSTER INFO failed",
                        ?err,
                        internal_log_rate_secs = 30
                    );
                }
            }
        } else if db_count == 0 {
            // in non-cluster mode, if db_count is zero then "CONFIG" failed to retrieve a
            // valid number of databases and we use the Redis config default which is 16
            db_count = 16
        }

        // info metrics
        if let Ok(ms) = extract_info_metrics(infos.as_str(), db_count) {
            metrics.extend(ms);
        }

        // latency
        if let Ok(ms) = extract_latency_metrics(&mut conn).await {
            metrics.extend(ms);
        }

        if let Ok(ms) = extract_slowlog_metrics(&mut conn).await {
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

async fn extract_slowlog_metrics<C: redis::aio::ConnectionLike>(
    conn: &mut C,
) -> Result<Vec<Metric>, Error> {
    let mut metrics = vec![];
    match redis::cmd("SLOWLOG")
        .arg("LEN")
        .query_async::<C, f64>(conn)
        .await
    {
        Ok(length) => {
            metrics.push(Metric::gauge("slowlog_length", "Total slowlog", length));
        }
        Err(err) => {
            warn!(message = "slowlog length query failed", ?err);
        }
    }

    let values: Vec<i64> = redis::cmd("SLOWLOG")
        .arg("GET")
        .arg("1")
        .query_async(conn)
        .await?;

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
            last_slow_execution_second as f64,
        ),
    ]);

    Ok(metrics)
}

// https://redis.io/commands/latency-latest
async fn extract_latency_metrics<C: redis::aio::ConnectionLike>(
    conn: &mut C,
) -> Result<Vec<Metric>, Error> {
    let mut metrics = vec![];
    let values: Vec<Vec<String>> = redis::cmd("LATENCY")
        .arg("LATEST")
        .query_async(conn)
        .await?;

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

fn extract_cluster_info_metrics(info: String) -> Result<Vec<Metric>, Error> {
    let mut metrics = vec![];
    info.split("\r\n").for_each(|line| {
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
}

fn extract_info_metrics(infos: &str, dbcount: i64) -> Result<Vec<Metric>, std::io::Error> {
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

async fn query_databases<C: redis::aio::ConnectionLike>(conn: &mut C) -> Result<i64, Error> {
    let resp: Vec<String> = redis::cmd("CONFIG")
        .arg("GET")
        .arg("*")
        .query_async(conn)
        .await?;

    if resp.len() % 2 != 0 {
        return Err(Error::InvalidData {
            desc: "config response".to_string(),
        });
    }

    for pos in 0..resp.len() / 2 {
        let key = resp[2 * pos].as_str();
        let value = resp[2 * pos + 1].as_str();

        if key == "databases" {
            return value.parse().map_err(|_err| Error::InvalidData {
                desc: value.to_string(),
            });
        }
    }

    Ok(0)
}

async fn cluster_info<C: redis::aio::ConnectionLike>(conn: &mut C) -> Result<String, Error> {
    let resp = redis::cmd("CLUSTER").arg("INFO").query_async(conn).await?;

    Ok(resp)
}

async fn query_infos<C: redis::aio::ConnectionLike>(conn: &mut C) -> Result<String, Error> {
    let resp = redis::cmd("INFO").arg("ALL").query_async(conn).await?;

    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    use redis::ToRedisArgs;
    use testcontainers::{images::redis::Redis, Docker};

    const REDIS_PORT: u16 = 6379;

    async fn write_testdata<C: redis::aio::ConnectionLike>(conn: &mut C) {
        for i in 0..100 {
            let key = format!("key_{}", i).to_redis_args();
            let value = format!("value_{}", i).to_redis_args();

            let _resp: () = redis::cmd("SET")
                .arg(key)
                .arg(value)
                .query_async(conn)
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    async fn dump_config() {
        let docker = testcontainers::clients::Cli::default();
        let service = docker.run(Redis::default());
        let host_port = service.get_host_port(REDIS_PORT).unwrap();
        let url = format!("redis://localhost:{}", host_port);

        let cli = redis::Client::open(url).unwrap();
        let mut conn = cli.get_async_connection().await.unwrap();

        let resp: Vec<String> = redis::cmd("CONFIG")
            .arg("GET")
            .arg("*")
            .query_async(&mut conn)
            .await
            .unwrap();

        assert_ne!(resp.len(), 0);
    }

    #[tokio::test]
    async fn test_slowlog() {
        let docker = testcontainers::clients::Cli::default();
        let service = docker.run(Redis::default());
        let host_port = service.get_host_port(REDIS_PORT).unwrap();
        let url = format!("redis://localhost:{}", host_port);
        let cli = redis::Client::open(url).unwrap();
        let mut conn = cli.get_multiplexed_tokio_connection().await.unwrap();

        write_testdata(&mut conn).await;

        let v = extract_slowlog_metrics(&mut conn).await.unwrap();
        assert_eq!(v.len(), 3);
    }

    #[tokio::test]
    async fn test_query_databases() {
        let docker = testcontainers::clients::Cli::default();
        let service = docker.run(Redis::default());
        let host_port = service.get_host_port(REDIS_PORT).unwrap();
        let url = format!("redis://localhost:{}", host_port);
        let cli = redis::Client::open(url).unwrap();
        let mut conn = cli.get_multiplexed_tokio_connection().await.unwrap();
        let n = query_databases(&mut conn).await.unwrap();
        assert_eq!(n, 16)
    }

    #[tokio::test]
    async fn test_latency_latest() {
        let docker = testcontainers::clients::Cli::default();
        let service = docker.run(Redis::default());
        let host_port = service.get_host_port(REDIS_PORT).unwrap();
        let url = format!("redis://localhost:{}", host_port);
        let cli = redis::Client::open(url).unwrap();
        let mut conn = cli.get_multiplexed_tokio_connection().await.unwrap();

        write_testdata(&mut conn).await;

        extract_latency_metrics(&mut conn).await.unwrap();
    }
}
