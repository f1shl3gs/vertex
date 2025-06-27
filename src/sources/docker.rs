use std::collections::HashMap;
use std::fmt::Display;
use std::ops::Sub;
use std::path::PathBuf;
use std::time::Duration;

use bytes::Bytes;
use chrono::{DateTime, Utc};
use configurable::schema::{SchemaGenerator, SchemaObject};
use configurable::{Configurable, configurable_component};
use event::{Metric, tags};
use framework::Source;
use framework::config::{Output, SourceConfig, SourceContext};
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use glob::Pattern;
use http::{Method, Request};
use http_body_util::{BodyExt, Full};
use hyper_unix::UnixConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

fn default_endpoint() -> PathBuf {
    PathBuf::from("/var/run/docker.sock")
}

const fn default_interval() -> Duration {
    Duration::from_secs(15)
}

const fn default_timeout() -> Duration {
    Duration::from_secs(5)
}

#[derive(Clone, Debug)]
enum Matcher {
    Glob(Pattern),
    Literal(String),
}

impl Matcher {
    fn matches(&self, value: &str) -> bool {
        match self {
            Matcher::Glob(pattern) => pattern.matches(value),
            Matcher::Literal(s) => value == s,
        }
    }
}

impl Configurable for Matcher {
    fn generate_schema(generator: &mut SchemaGenerator) -> SchemaObject {
        String::generate_schema(generator)
    }
}

impl<'de> Deserialize<'de> for Matcher {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let matcher = match Pattern::new(s.as_str()) {
            Ok(pattern) => Matcher::Glob(pattern),
            Err(_err) => Matcher::Literal(s),
        };

        Ok(matcher)
    }
}

impl Serialize for Matcher {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Matcher::Glob(pattern) => serializer.serialize_str(pattern.as_str()),
            Matcher::Literal(s) => serializer.serialize_str(s),
        }
    }
}

#[configurable_component(source, name = "docker")]
struct Config {
    #[serde(default = "default_endpoint")]
    endpoint: PathBuf,

    /// A list of filters whose matching images are to be excluded.
    #[serde(default)]
    excluded_images: Vec<Matcher>,

    #[serde(default = "default_timeout", with = "humanize::duration::serde")]
    timeout: Duration,

    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "docker")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let path = self.endpoint.clone();
        let interval = self.interval;
        let mut shutdown = cx.shutdown;
        let mut output = cx.output;
        let excluded_images = self.excluded_images.clone();

        let connector = UnixConnector::new(path);
        let client = hyper_util::client::legacy::Builder::new(TokioExecutor::new())
            .build::<_, Full<Bytes>>(connector);

        Ok(Box::pin(async move {
            let mut ticker = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    _ = &mut shutdown => break,
                    _ = ticker.tick() => {}
                }

                // list containers
                let containers = match list_containers(&client).await {
                    Ok(containers) => containers,
                    Err(err) => {
                        warn!(message = "list running container failed", ?err);
                        continue;
                    }
                };

                let mut tasks = FuturesUnordered::new();
                for container in containers {
                    if excluded_images
                        .iter()
                        .any(|matcher| matcher.matches(container.image.as_str()))
                    {
                        continue;
                    }

                    tasks.push(get_container_stats(&client, container));
                }

                while let Some(result) = tasks.next().await {
                    match result {
                        Ok((container, inspect, stats)) => {
                            let metrics = build_metrics(container, inspect, stats);
                            if let Err(_err) = output.send(metrics).await {
                                return Ok(());
                            }
                        }
                        Err(err) => {
                            warn!(message = "failed to collect metrics for container", %err);
                            continue;
                        }
                    }
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

async fn list_containers(
    client: &Client<UnixConnector, Full<Bytes>>,
) -> Result<Vec<Container>, Error> {
    let req = Request::builder()
        .method(Method::GET)
        // filters={"status":["running"]}
        .uri("http://localhost/containers/json?filters=%7B%22status%22%3A%5B%22running%22%5D%7D")
        .body(Full::<Bytes>::default())
        .unwrap();

    let resp = client.request(req).await.map_err(Error::Client)?;
    let (parts, incoming) = resp.into_parts();
    if !parts.status.is_success() {
        return Err(Error::UnexpectedStatusCode(parts.status));
    }

    let body = incoming.collect().await.map_err(Error::Hyper)?.to_bytes();

    serde_json::from_slice(&body).map_err(Error::Deserialize)
}

async fn get_container_stats(
    client: &Client<UnixConnector, Full<Bytes>>,
    container: Container,
) -> Result<(Container, ContainerInspect, ContainerStats), Error> {
    let req = Request::builder()
        .method(Method::GET)
        .uri(format!("http://localhost/containers/{}/json", container.id))
        .body(Full::<Bytes>::default())
        .unwrap();

    let resp = client.request(req).await.map_err(Error::Client)?;
    let (parts, incoming) = resp.into_parts();
    if !parts.status.is_success() {
        return Err(Error::UnexpectedStatusCode(parts.status));
    }

    let body = incoming.collect().await.map_err(Error::Hyper)?.to_bytes();
    let inspect = serde_json::from_slice::<ContainerInspect>(&body).map_err(Error::Deserialize)?;

    let req = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "http://localhost/containers/{}/stats?stream=false",
            container.id
        ))
        .body(Full::<Bytes>::default())
        .unwrap();

    let resp = client.request(req).await.map_err(Error::Client)?;
    let (parts, incoming) = resp.into_parts();
    if !parts.status.is_success() {
        return Err(Error::UnexpectedStatusCode(parts.status));
    }

    let body = incoming.collect().await.map_err(Error::Hyper)?.to_bytes();
    let stats = serde_json::from_slice::<ContainerStats>(&body).map_err(Error::Deserialize)?;

    Ok((container, inspect, stats))
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct State {
    started_at: DateTime<Utc>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct HostConfig {
    /// relative weight vs. other containers
    cpu_shares: i64,
    // /// Limits in bytes
    // memory: i64,
    /// CPU quota in units of 10<sup>-9</sup> CPUs
    nano_cpus: i64,
    cpu_period: i64,
    cpu_quota: i64,
    /// CpusetCpus 0-2, 0,1
    cpuset_cpus: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ContainerConfig {
    image: String,
    hostname: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ContainerInspect {
    restart_count: u64,
    state: State,
    #[serde(default)]
    host_config: HostConfig,
    config: ContainerConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Container {
    id: String,
    image: String,
    #[serde(rename = "ImageID")]
    image_id: String,
}

/// CPU throttling stats of one running container.
///
/// Not used on Windows
#[derive(Deserialize)]
struct ThrottlingData {
    /// Number of periods with throttling active
    periods: i64,

    /// Number of periods when the container hits its throttling limit.
    throttled_periods: i64,

    /// Aggregate time the container was throttled for in nanoseconds
    throttled_time: i64,
}

/// All CPU stats aggregated since container inception
#[derive(Deserialize)]
struct CpuUsage {
    /// Total CPU time consumed per core (Linux). Not used on Windows
    ///
    /// Units: nanoseconds
    #[serde(default)]
    percpu_usage: Vec<i64>,

    /// Time spent by tasks of the cgroup in user mode (Linux)
    /// Time spent by all container processes in user mode (Windows)
    ///
    /// nanoseconds on Linux
    /// 100's of nanoseconds on Windows
    usage_in_usermode: i64,

    /// Total CPU time consumed
    ///
    /// nanoseconds on Linux
    /// 100's of nanoseconds on Windows
    total_usage: i64,

    /// Time spent by tasks of the cgroup in kernel mode (Linux).
    /// Time spent by all containers processes in kernel mode (Windows)
    ///
    /// nanoseconds on Linux
    /// 100's of nanoseconds on Windows
    usage_in_kernelmode: i64,
}

#[derive(Deserialize)]
struct CpuStats {
    /// CPU Usage
    cpu_usage: CpuUsage,
    /// System Usage, linux only
    #[cfg(target_os = "linux")]
    system_cpu_usage: i64,
    /// Online CPUs, linux only
    #[cfg(target_os = "linux")]
    online_cpus: i64,
    /// Throttling Data, linux only
    #[cfg(target_os = "linux")]
    throttling_data: ThrottlingData,
}

/// One small entity to store a piece of Blkio stats
#[derive(Deserialize)]
struct BlkioStatsEntry {
    major: u64,
    minor: u64,
    op: String,
    value: u64,
}

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

/// All IO service stats for data read and write. This is a Linux specific structure as
/// the differences between expressing block I/O on Windows and Linux are sufficiently
/// significant to make little sense attempting to morph into a combined structure.
#[derive(Deserialize)]
struct BlkioStats {
    #[serde(deserialize_with = "deserialize_null_default")]
    io_service_bytes_recursive: Vec<BlkioStatsEntry>,
    #[serde(deserialize_with = "deserialize_null_default")]
    io_serviced_recursive: Vec<BlkioStatsEntry>,
    #[serde(deserialize_with = "deserialize_null_default")]
    io_queue_recursive: Vec<BlkioStatsEntry>,
    #[serde(deserialize_with = "deserialize_null_default")]
    io_service_time_recursive: Vec<BlkioStatsEntry>,
    #[serde(deserialize_with = "deserialize_null_default")]
    io_wait_time_recursive: Vec<BlkioStatsEntry>,
    #[serde(deserialize_with = "deserialize_null_default")]
    io_merged_recursive: Vec<BlkioStatsEntry>,
    #[serde(deserialize_with = "deserialize_null_default")]
    io_time_recursive: Vec<BlkioStatsEntry>,
    #[serde(deserialize_with = "deserialize_null_default")]
    sectors_recursive: Vec<BlkioStatsEntry>,
}

#[derive(Deserialize)]
struct Stats {
    dirty: Option<u64>,
    total_dirty: Option<u64>,
    total_pgmajfault: Option<u64>,
    cache: Option<u64>,
    mapped_file: Option<u64>,
    total_inactive_file: Option<u64>,
    pgpgout: Option<u64>,
    rss: Option<u64>,
    total_mapped_file: Option<u64>,
    writeback: Option<u64>,
    unevictable: Option<u64>,
    pgpgin: Option<u64>,
    total_unevictable: Option<u64>,
    pgmajfault: Option<u64>,
    total_rss: Option<u64>,
    total_rss_huge: Option<u64>,
    total_writeback: Option<u64>,
    total_inactive_anon: Option<u64>,
    rss_huge: Option<u64>,
    hierarchical_memory_limit: Option<u64>,
    hierarchical_memswap_limit: Option<u64>,
    total_pgfault: Option<u64>,
    total_active_file: Option<u64>,
    active_anon: Option<u64>,
    total_active_anon: Option<u64>,
    total_pgpgout: Option<u64>,
    total_cache: Option<u64>,
    inactive_anon: Option<u64>,
    active_file: Option<u64>,
    pgfault: Option<u64>,
    inactive_file: Option<u64>,
    total_pgpgin: Option<u64>,
    anon: Option<u64>,
    file: Option<u64>,
}

#[derive(Deserialize)]
struct MemoryStats {
    /// Export these as stronger types, all the stats exported via memory.stat
    stats: Stats,
    /// maximum usage ever recorded.
    #[serde(default)]
    max_usage: i64,
    /// current res_counter usage for memory
    usage: u64,
    /// Number of times memory usage hits limits
    #[serde(default)]
    failcnt: i64,
    limit: u64,
}

/// The network stats of one container
#[derive(Deserialize)]
struct NetworkStats {
    /// Bytes received
    rx_bytes: i64,
    /// Incoming packets dropped.
    rx_dropped: i64,
    /// Received errors. Not used on Windows. Note that we don't `omitempty` this field
    /// as it is expected in the >= v1.21 API stats structure.
    rx_errors: i64,
    /// Packets received
    rx_packets: i64,
    /// Bytes sent
    tx_bytes: i64,
    /// Outgoing packets dropped
    tx_dropped: i64,
    /// Sent errors. Not used on Windows. Note that we don't `omitempty` this field as it
    /// is expected in the >= v1.21 API stats structure.
    tx_errors: i64,
    /// Packets sent.
    tx_packets: i64,
}

#[derive(Deserialize)]
struct PidsStats {
    /// The number of pids in the cgroup
    current: u64,
    /// The hard limit on the number of pids in the cgroup
    limit: u64,
}

// /// The disk I/O stats for read/write on Windows
// #[derive(Deserialize)]
// struct StorageStats {
//     read_count_normalized: u64,
//     read_size_bytes: u64,
//     write_count_normalized: u64,
//     write_size_bytes: u64,
// }

#[derive(Deserialize)]
struct ContainerStats {
    id: String,
    name: String,

    // Common stats
    // read: String,
    // preread: String,

    // Linux specific stats, not populated on Windows
    pids_stats: PidsStats,
    blkio_stats: BlkioStats,

    // Windows specific stats, not populated on Linux
    // num_procs: u32,
    // storage_stats: StorageStats,

    // Shared stats
    cpu_stats: CpuStats,
    precpu_stats: CpuStats,
    memory_stats: MemoryStats,
    networks: HashMap<String, NetworkStats>,
}

#[derive(Debug)]
enum Error {
    UnexpectedStatusCode(http::StatusCode),

    Hyper(hyper::Error),

    Client(hyper_util::client::legacy::Error),

    Deserialize(serde_json::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::UnexpectedStatusCode(code) => write!(f, "unexpected status code: {code}"),
            Error::Hyper(err) => err.fmt(f),
            Error::Client(err) => err.fmt(f),
            Error::Deserialize(err) => err.fmt(f),
        }
    }
}

fn build_metrics(
    container: Container,
    inspect: ContainerInspect,
    stats: ContainerStats,
) -> Vec<Metric> {
    let mut metrics = Vec::new();

    // base metric
    let elapsed = Utc::now().sub(inspect.state.started_at);
    metrics.extend([
        Metric::sum(
            "container_restarts",
            "Number of restarts for the container.",
            inspect.restart_count,
        ),
        Metric::gauge(
            "container_cpu_shares",
            "CPU shares set for the container",
            inspect.host_config.cpu_shares,
        ),
        Metric::sum(
            "container_uptime",
            "Time elapsed since container start time.",
            elapsed.num_seconds(),
        ),
    ]);
    if let Some(limit) = calculate_cpu_limit(&inspect.host_config) {
        if limit > 0.0 {
            metrics.push(Metric::gauge(
                "container_cpu_limit",
                "CPU limit set for the container.",
                limit,
            ));
        }
    } else {
        warn!(
            message = "calculate cpu limit failed",
            container = stats.id,
            container_name = stats.name
        );
    }

    // cpu usage
    metrics.extend([
        Metric::sum(
            "container_cpu_system_usage",
            "System CPU usage, as reported by docker",
            stats.cpu_stats.system_cpu_usage,
        ),
        Metric::sum(
            "container_cpu_usage_total",
            "Total CPU time consumed",
            stats.cpu_stats.cpu_usage.total_usage,
        ),
        Metric::sum(
            "container_cpu_kernel_mode_usage",
            "Time spent by tasks of the cgroup in kernel mode (Linux).  Time spent by all container processes in kernel mode (Windows).",
            stats.cpu_stats.cpu_usage.usage_in_kernelmode,
        ),
        Metric::sum(
            "container_cpu_usage_user_mode",
            "Time spent by tasks of the cgroup in user mode (Linux). Time spent by all container processes in user mode (Windows).",
            stats.cpu_stats.cpu_usage.usage_in_usermode,
        ),
        Metric::sum(
            "container_cpu_throttling_data_throttling_periods",
            "Number of periods when the container hits its throttling limit.",
            stats.cpu_stats.throttling_data.throttled_periods,
        ),
        Metric::sum(
            "container_cpu_throttling_data_periods",
            "Number of periods with throttling active.",
            stats.cpu_stats.throttling_data.periods,
        ),
        Metric::sum(
            "container_cpu_throttling_data_throttled_time",
            "Aggregate time the container was throttled.",
            stats.cpu_stats.throttling_data.throttled_time,
        ),
        Metric::sum(
            "container_cpu_utilization",
            "Percent of CPU used by the container",
            calculate_cpu_percent(&stats.precpu_stats, &stats.cpu_stats),
        ),
        Metric::gauge(
            "container_cpu_logical_count",
            "Number of cores available to the container",
            stats.cpu_stats.online_cpus,
        )
    ]);
    for (index, value) in stats.cpu_stats.cpu_usage.percpu_usage.iter().enumerate() {
        metrics.push(Metric::gauge_with_tags(
            "container_cpu_percpu_usage",
            "Per-core CPU usage by the container (Only available with cgroups v1).",
            *value,
            tags!(
                "id" => stats.id.clone(),
                "name" => stats.name.clone(),
                "cpu" => format!("cpu{index}")
            ),
        ));
    }

    // Memory
    let total_usage = calculate_memory_usage_no_cache(&stats.memory_stats);
    metrics.extend([
        Metric::sum(
            "container_memory_usage_total",
            "Memory usage of the container. This excludes the cache.",
            total_usage,
        ),
        Metric::sum(
            "container_memory_usage_limit",
            "Memory limit of the container.",
            stats.memory_stats.limit,
        ),
        Metric::gauge(
            "container_memory_percent",
            "Percentage of memory used",
            calculate_memory_percent(stats.memory_stats.limit, total_usage),
        ),
        Metric::gauge(
            "container_memory_max_usage",
            "Maximum memory usage",
            stats.memory_stats.max_usage,
        ),
        Metric::sum(
            "container_memory_fails",
            "Number of times the memory limit was hit",
            stats.memory_stats.failcnt,
        ),
    ]);
    if let Some(value) = stats.memory_stats.stats.cache {
        metrics.push(Metric::sum(
            "container_memory_cache",
            "The amount of memory used by the processes of this control group that can be associated precisely with a block on a block device (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.total_cache {
        metrics.push(Metric::sum(
            "container_memory_total_cache",
            "Total amount of memory used by the processes of this cgroup (and descendants) that can be associated with a block on a block device. Also accounts for memory used by tmpfs (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.rss {
        metrics.push(Metric::sum(
            "container_memory_rss",
            "The amount of memory that doesn’t correspond to anything on disk: stacks, heaps, and anonymous memory maps (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.total_rss {
        metrics.push(Metric::sum(
            "container_memory_total_rss",
            "The amount of memory that doesn’t correspond to anything on disk: stacks, heaps, and anonymous memory maps. Includes descendant cgroups (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.rss_huge {
        metrics.push(Metric::sum(
            "container_memory_rss_huge",
            "Number of bytes of anonymous transparent hugepages in this cgroup (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.total_rss_huge {
        metrics.push(Metric::sum(
            "container_memory_total_rss_huge",
            "Number of bytes of anonymous transparent hugepages in this cgroup and descendant cgroups (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.dirty {
        metrics.push(Metric::sum(
            "container_memory_dirty",
            "Bytes that are waiting to get written back to the disk, from this cgroup (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.total_dirty {
        metrics.push(Metric::sum(
            "container_memory_total_dirty",
            "Bytes that are waiting to get written back to the disk, from this cgroup and descendants (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.writeback {
        metrics.push(Metric::sum(
            "container_memory_writeback",
            "Number of bytes of file/anon cache that are queued for syncing to disk in this cgroup (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.total_writeback {
        metrics.push(Metric::sum(
            "container_memory_total_writeback",
            "Number of bytes of file/anon cache that are queued for syncing to disk in this cgroup and descendants (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.mapped_file {
        metrics.push(Metric::sum(
            "container_memory_mapped_file",
            "Indicates the amount of memory mapped by the processes in the control group (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.total_mapped_file {
        metrics.push(Metric::sum(
            "container_memory_total_mapped_file",
            "Indicates the amount of memory mapped by the processes in the control group and descendant groups (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.pgpgin {
        metrics.push(Metric::sum(
            "container_memory_pgpgin",
            "Number of pages read from disk by the cgroup (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.total_pgpgin {
        metrics.push(Metric::sum(
            "container_memory_total_pgpgin",
            "Number of pages read from disk by the cgroup and descendant groups (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.pgpgout {
        metrics.push(Metric::sum(
            "container_memory_pgpgout",
            "Number of pages written to disk by the cgroup (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.total_pgpgout {
        metrics.push(Metric::sum(
            "container_memory_total_pgpgout",
            "Number of pages written to disk by the cgroup and descendant groups (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.pgfault {
        metrics.push(Metric::sum(
            "container_memory_pgfault",
            "Indicate the number of times that a process of the cgroup triggered a page fault.",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.total_pgfault {
        metrics.push(Metric::sum(
            "container_memory_total_pgfault",
            "Indicate the number of times that a process of the cgroup (or descendant cgroups) triggered a page fault (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.pgmajfault {
        metrics.push(Metric::sum(
            "container_memory_pgmajfault",
            "Indicate the number of times that a process of the cgroup triggered a major fault.",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.total_pgmajfault {
        metrics.push(Metric::sum(
            "container_memory_total_pgmajfault",
            "Indicate the number of times that a process of the cgroup (or descendant cgroups) triggered a major fault (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.inactive_anon {
        metrics.push(Metric::sum(
            "container_memory_inactive_anon",
            "Indicate the number of times that a process of the cgroup (or descendant cgroups) triggered a major fault (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.total_inactive_anon {
        metrics.push(Metric::sum(
            "container_memory_total_inactive_anon",
            "The amount of anonymous memory that has been identified as inactive by the kernel. Includes descendant cgroups (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.active_anon {
        metrics.push(Metric::sum(
            "container_memory_active_anon",
            "The amount of anonymous memory that has been identified as active by the kernel.",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.total_active_anon {
        metrics.push(Metric::sum(
            "container_memory_total_active_anon",
            "The amount of anonymous memory that has been identified as active by the kernel. Includes descendant cgroups (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.inactive_file {
        metrics.push(Metric::sum(
            "container_memory_inactive_file",
            "Cache memory that has been identified as inactive by the kernel.",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.total_inactive_file {
        metrics.push(Metric::sum(
            "container_memory_total_inactive_file",
            "Cache memory that has been identified as inactive by the kernel. Includes descendant cgroups (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.active_file {
        metrics.push(Metric::sum(
            "container_memory_active_file",
            "Cache memory that has been identified as inactive by the kernel. Includes descendant cgroups (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.total_active_file {
        metrics.push(Metric::sum(
            "container_memory_total_active_file",
            "Cache memory that has been identified as active by the kernel. Includes descendant cgroups (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.unevictable {
        metrics.push(Metric::sum(
            "container_memory_unevictable",
            "The amount of memory that cannot be reclaimed.",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.total_unevictable {
        metrics.push(Metric::sum(
            "container_memory_total_unevictable",
            "The amount of memory that cannot be reclaimed. Includes descendant cgroups (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.hierarchical_memory_limit {
        metrics.push(Metric::sum(
            "container_memory_hierarchical_memory_limit",
            "The maximum amount of physical memory that can be used by the processes of this control group (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.hierarchical_memswap_limit {
        metrics.push(Metric::sum(
            "container_memory_hierarchical_memswap_limit",
            "The maximum amount of RAM + swap that can be used by the processes of this control group (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.anon {
        metrics.push(Metric::sum(
            "container_memory_anon",
            "The maximum amount of RAM + swap that can be used by the processes of this control group (Only available with cgroups v1).",
            value,
        ));
    }
    if let Some(value) = stats.memory_stats.stats.file {
        metrics.push(Metric::sum(
            "container_memory_file",
            "Amount of memory used to cache filesystem data, including tmpfs and shared memory (Only available with cgroups v2).",
            value,
        ));
    }

    // blkio
    metrics.extend(blkio_metrics(stats.blkio_stats));
    metrics.extend(pids_metrics(stats.pids_stats));

    // Network
    for (interface, network) in stats.networks {
        let tags = tags!(
            "interface" => interface,
        );

        metrics.extend([
            Metric::sum_with_tags(
                "container_network_rx_bytes",
                "Bytes received by the container.",
                network.rx_bytes,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "container_network_rx_dropped",
                "Incoming packets dropped.",
                network.rx_dropped,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "container_network_rx_errors",
                "Received errors.",
                network.rx_errors,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "container_network_rx_packets",
                "Received packets.",
                network.rx_packets,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "container_network_tx_bytes",
                "Bytes sent",
                network.tx_bytes,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "container_network_tx_dropped",
                "Outgoing packets dropped.",
                network.tx_dropped,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "container_network_tx_errors",
                "Sent errors.",
                network.tx_errors,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "container_network_tx_packets",
                "Sent packets.",
                network.tx_packets,
                tags,
            ),
        ]);
    }

    let now = Utc::now();
    metrics.iter_mut().for_each(|metric| {
        metric.timestamp = Some(now);

        let tags = metric.tags_mut();
        tags.insert("hostname", inspect.config.hostname.clone());
        tags.insert("id", stats.id.clone());
        tags.insert("name", stats.name.strip_prefix("/").unwrap_or(&stats.name));
        tags.insert("image_id", container.image_id.clone());

        match inspect.config.image.split_once('@') {
            Some((name, _id)) => {
                tags.insert("image_name", name);
            }
            None => {
                tags.insert("image_name", inspect.config.image.clone());
            }
        }
    });

    metrics
}

/// decompose -cpuset-cpus value into number os cpus
fn parse_cpuset(s: &str) -> Option<u64> {
    let mut cpus = 0;

    let line_slice = s.split(",");
    for part in line_slice {
        let parts = part.split("-").collect::<Vec<_>>();
        if parts.len() == 2 {
            let p0 = parts[0].parse::<u64>().ok()?;

            let p1 = parts[1].parse::<u64>().ok()?;

            cpus += p1 - p0 + 1;
        } else if parts.len() == 1 {
            cpus += 1;
        }
    }

    Some(cpus)
}

fn calculate_cpu_limit(config: &HostConfig) -> Option<f64> {
    let limit = if config.nano_cpus > 0 {
        config.nano_cpus as f64 / 1e9
    } else if config.cpuset_cpus.is_empty() {
        let limit = parse_cpuset(&config.cpuset_cpus)?;

        limit as f64
    } else if config.cpu_quota > 0 {
        let period = if config.cpu_period == 0 {
            config.cpu_period
        } else {
            100000
        };

        config.cpu_quota as f64 / period as f64
    } else {
        0.0
    };

    Some(limit)
}

fn calculate_cpu_percent(prev: &CpuStats, curr: &CpuStats) -> f64 {
    let mut cpu_percent = 0.0;
    let cpu_delta = curr.cpu_usage.total_usage - prev.cpu_usage.total_usage;
    let system_delta = curr.system_cpu_usage - prev.system_cpu_usage;
    let mut online_cpus = curr.online_cpus;

    if online_cpus == 0 {
        online_cpus = curr.cpu_usage.percpu_usage.len() as i64;
    }

    if system_delta > 0 && cpu_delta > 0 {
        cpu_percent = (cpu_delta as f64 / system_delta as f64) * online_cpus as f64 * 100.0;
    }

    cpu_percent
}

/// Calculate memory usage of the container. Cache is intentionally excluded to avoid
/// misinterpretation of the output.
///
/// On cgroup v1 host, the result is `mem.Usage - mem.Stats["total_inactive_file"]`
/// On cgroup v2 host, the result is `mem.Usage - mem.Stats["inactive_file"]`
fn calculate_memory_usage_no_cache(stats: &MemoryStats) -> u64 {
    // cgroup v1
    if let Some(value) = stats.stats.total_inactive_file {
        if value < stats.usage {
            return stats.usage - value;
        }
    }

    // cgroup v2
    if let Some(value) = stats.stats.inactive_file {
        return stats.usage - value;
    }

    stats.usage
}

fn calculate_memory_percent(limit: u64, used_no_cache: u64) -> f64 {
    // MemoryStats.Limit will never be 0 unless the container is not running and we
    // haven't got any data from cgroup
    if limit != 0 {
        return used_no_cache as f64 / limit as f64 * 100.0;
    }

    0.0
}

fn blkio_metrics(stats: BlkioStats) -> Vec<Metric> {
    let mut metrics = Vec::new();

    for entry in stats.io_merged_recursive {
        metrics.push(Metric::sum_with_tags(
            "container_blkio_merged_recursive",
            "Number of bios/requests merged into requests belonging to this cgroup and its descendant cgroups (Only available with cgroups v1).",
            entry.value,
            tags!(
                "major" => entry.major,
                "minor" => entry.minor,
                "op" => entry.op,
            ),
        ));
    }

    for entry in stats.io_queue_recursive {
        metrics.push(Metric::sum_with_tags(
            "container_blkio_queue_recursive",
            "Number of requests queued up for this cgroup and its descendant cgroups (Only available with cgroups v1).",
            entry.value,
            tags!(
                "major" => entry.major,
                "minor" => entry.minor,
                "op" => entry.op,
            )
        ))
    }

    for entry in stats.io_service_bytes_recursive {
        metrics.push(Metric::sum_with_tags(
            "container_blkio_service_bytes_recursive",
            "Number of bytes transferred to/from the disk by the group and descendant groups.",
            entry.value,
            tags!(
                "major" => entry.major,
                "minor" => entry.minor,
                "op" => entry.op,
            ),
        ));
    }

    for entry in stats.io_service_time_recursive {
        metrics.push(Metric::sum_with_tags(
            "container_blkio_service_time_recursive",
            "Total amount of time in nanoseconds between request dispatch and request completion for the IOs done by this cgroup and descendant cgroups (Only available with cgroups v1).",
            entry.value,
            tags!(
                "major" => entry.major,
                "minor" => entry.minor,
                "op" => entry.op,
            )
        ));
    }

    for entry in stats.io_serviced_recursive {
        metrics.push(Metric::sum_with_tags(
            "container_blkiod_serviced_recursive",
            "Number of IOs (bio) issued to the disk by the group and descendant groups (Only available with cgroups v1).",
            entry.value,
            tags!(
                "major" => entry.major,
                "minor" => entry.minor,
                "op" => entry.op,
            )
        ));
    }

    for entry in stats.io_time_recursive {
        metrics.push(Metric::sum_with_tags(
            "container_blkio_time_recursive",
            "Disk time allocated to cgroup (and descendant cgroups) per device in milliseconds (Only available with cgroups v1).",
            entry.value,
            tags!(
                "major" => entry.major,
                "minor" => entry.minor,
                "op" => entry.op,
            )
        ));
    }

    for entry in stats.io_wait_time_recursive {
        metrics.push(Metric::sum_with_tags(
            "container_blkio_wait_time_recursive",
            "Total amount of time the IOs for this cgroup (and descendant cgroups) spent waiting in the scheduler queues for service (Only available with cgroups v1).",
            entry.value,
            tags!(
                "major" => entry.major,
                "minor" => entry.minor,
                "op" => entry.op,
            )
        ));
    }

    for entry in stats.sectors_recursive {
        metrics.push(Metric::sum_with_tags(
            "container_blkio_sectors_recursive",
            "Number of sectors transferred to/from disk by the group and descendant groups (Only available with cgroups v1).",
            entry.value,
            tags!(
                "major" => entry.major,
                "minor" => entry.minor,
                "op" => entry.op,
            )
        ));
    }

    metrics
}

fn pids_metrics(stats: PidsStats) -> Vec<Metric> {
    let mut metrics = Vec::new();

    // pidsStats are available when kernel version is >= 4.3 and pids_cgroup is supported, it is
    // empty otherwise
    if stats.current != 0 {
        metrics.push(Metric::sum(
            "container_pids_count",
            "Number of pids in the container's cgroup.",
            stats.current,
        ));

        if stats.limit != 0 {
            metrics.push(Metric::sum(
                "container_pids_limit",
                "Maximum number of pids in the container's cgroup.",
                stats.limit,
            ));
        }
    }

    metrics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }
}
