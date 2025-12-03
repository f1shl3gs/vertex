use std::collections::HashMap;
use std::ops::Sub;
use std::path::PathBuf;
use std::time::Duration;

use chrono::Utc;
use configurable::schema::{SchemaGenerator, SchemaObject};
use configurable::{Configurable, configurable_component};
use docker::containers::{
    BlkioStats, ContainerHostConfig, ContainerInspect, ContainerStats, CpuStats,
    ListContainersOptions, MemoryStats, PidsStats,
};
use docker::{Client, Error};
use event::{Metric, tags};
use framework::Source;
use framework::config::{OutputType, SourceConfig, SourceContext};
use futures::StreamExt;
use futures::stream::FuturesOrdered;
use glob::Pattern;
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
        let interval = self.interval;
        let excluded_images = self.excluded_images.clone();
        let client = Client::new(self.endpoint.clone());
        let mut shutdown = cx.shutdown;
        let mut output = cx.output;

        Ok(Box::pin(async move {
            let mut ticker = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    _ = &mut shutdown => break,
                    _ = ticker.tick() => {}
                }

                // list containers
                let opts = ListContainersOptions {
                    filters: Some({
                        let mut filters = HashMap::new();
                        filters.insert("status", vec!["running"]);
                        filters
                    }),
                    ..Default::default()
                };
                let containers = match client.list_containers(opts).await {
                    Ok(containers) => containers
                        .into_iter()
                        .filter(|c| !excluded_images.iter().any(|m| m.matches(&c.image)))
                        .collect::<Vec<_>>(),
                    Err(err) => {
                        warn!(message = "list running container failed", ?err);
                        continue;
                    }
                };

                let inspect_tasks = FuturesOrdered::from_iter(
                    containers.iter().map(|c| client.inspect_container(&c.id)),
                );
                let stats_tasks =
                    FuturesOrdered::from_iter(containers.iter().map(|c| client.stats(&c.id)));

                let inspects = inspect_tasks
                    .collect::<Vec<Result<ContainerInspect, Error>>>()
                    .await;
                let stats = stats_tasks
                    .collect::<Vec<Result<ContainerStats, Error>>>()
                    .await;

                for result in inspects.into_iter().zip(stats.into_iter()) {
                    if let (Ok(inspect), Ok(stats)) = result {
                        let metrics = build_metrics(inspect, stats);

                        if let Err(_err) = output.send(metrics).await {
                            return Ok(());
                        }
                    }
                }
            }

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::metric()]
    }
}

fn build_metrics(inspect: ContainerInspect, stats: ContainerStats) -> Vec<Metric> {
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
    if let Some(values) = stats.cpu_stats.cpu_usage.percpu_usage {
        for (index, value) in values.into_iter().enumerate() {
            metrics.push(Metric::gauge_with_tags(
                "container_cpu_percpu_usage",
                "Per-core CPU usage by the container (Only available with cgroups v1).",
                value,
                tags!(
                    "id" => stats.id.clone(),
                    "name" => stats.name.clone(),
                    "cpu" => format!("cpu{index}")
                ),
            ));
        }
    }

    // Memory
    if let Some(memory_stats) = &stats.memory_stats {
        let total_usage = calculate_memory_usage_no_cache(memory_stats);
        metrics.extend([
            Metric::sum(
                "container_memory_usage_total",
                "Memory usage of the container. This excludes the cache.",
                total_usage,
            ),
            Metric::sum(
                "container_memory_usage_limit",
                "Memory limit of the container.",
                memory_stats.limit,
            ),
            Metric::gauge(
                "container_memory_percent",
                "Percentage of memory used",
                calculate_memory_percent(memory_stats.limit, total_usage),
            ),
        ]);

        if let Some(value) = memory_stats.failcnt {
            metrics.push(Metric::sum(
                "container_memory_fails",
                "Number of times the memory limit was hit",
                value,
            ));
        }
        if let Some(value) = memory_stats.max_usage {
            metrics.push(Metric::gauge(
                "container_memory_max_usage",
                "Maximum memory usage",
                value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("cache") {
            metrics.push(Metric::sum(
                "container_memory_cache",
                "The amount of memory used by the processes of this control group that can be associated precisely with a block on a block device (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("total_cache") {
            metrics.push(Metric::sum(
                "container_memory_total_cache",
                "Total amount of memory used by the processes of this cgroup (and descendants) that can be associated with a block on a block device. Also accounts for memory used by tmpfs (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("rss") {
            metrics.push(Metric::sum(
                "container_memory_rss",
                "The amount of memory that doesn’t correspond to anything on disk: stacks, heaps, and anonymous memory maps (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("total_rss") {
            metrics.push(Metric::sum(
                "container_memory_total_rss",
                "The amount of memory that doesn’t correspond to anything on disk: stacks, heaps, and anonymous memory maps. Includes descendant cgroups (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("rss_huge") {
            metrics.push(Metric::sum(
                "container_memory_rss_huge",
                "Number of bytes of anonymous transparent hugepages in this cgroup (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("total_rss_huge") {
            metrics.push(Metric::sum(
                "container_memory_total_rss_huge",
                "Number of bytes of anonymous transparent hugepages in this cgroup and descendant cgroups (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("dirty") {
            metrics.push(Metric::sum(
                "container_memory_dirty",
                "Bytes that are waiting to get written back to the disk, from this cgroup (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("total_dirty") {
            metrics.push(Metric::sum(
                "container_memory_total_dirty",
                "Bytes that are waiting to get written back to the disk, from this cgroup and descendants (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("writeback") {
            metrics.push(Metric::sum(
                "container_memory_writeback",
                "Number of bytes of file/anon cache that are queued for syncing to disk in this cgroup (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("total_writeback") {
            metrics.push(Metric::sum(
                "container_memory_total_writeback",
                "Number of bytes of file/anon cache that are queued for syncing to disk in this cgroup and descendants (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("mapped_file") {
            metrics.push(Metric::sum(
                "container_memory_mapped_file",
                "Indicates the amount of memory mapped by the processes in the control group (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("total_mapped_file") {
            metrics.push(Metric::sum(
                "container_memory_total_mapped_file",
                "Indicates the amount of memory mapped by the processes in the control group and descendant groups (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("pgpgin") {
            metrics.push(Metric::sum(
                "container_memory_pgpgin",
                "Number of pages read from disk by the cgroup (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("total_pgpgin") {
            metrics.push(Metric::sum(
                "container_memory_total_pgpgin",
                "Number of pages read from disk by the cgroup and descendant groups (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("pgpgout") {
            metrics.push(Metric::sum(
                "container_memory_pgpgout",
                "Number of pages written to disk by the cgroup (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("total_pgpgout") {
            metrics.push(Metric::sum(
                "container_memory_total_pgpgout",
                "Number of pages written to disk by the cgroup and descendant groups (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("pgfault") {
            metrics.push(Metric::sum(
                "container_memory_pgfault",
                "Indicate the number of times that a process of the cgroup triggered a page fault.",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("total_pgfault") {
            metrics.push(Metric::sum(
                "container_memory_total_pgfault",
                "Indicate the number of times that a process of the cgroup (or descendant cgroups) triggered a page fault (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("pgmajfault") {
            metrics.push(Metric::sum(
                "container_memory_pgmajfault",
                "Indicate the number of times that a process of the cgroup triggered a major fault.",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("total_pgmajfault") {
            metrics.push(Metric::sum(
                "container_memory_total_pgmajfault",
                "Indicate the number of times that a process of the cgroup (or descendant cgroups) triggered a major fault (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("inactive_anon") {
            metrics.push(Metric::sum(
                "container_memory_inactive_anon",
                "Indicate the number of times that a process of the cgroup (or descendant cgroups) triggered a major fault (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("total_inactive_anon") {
            metrics.push(Metric::sum(
                "container_memory_total_inactive_anon",
                "The amount of anonymous memory that has been identified as inactive by the kernel. Includes descendant cgroups (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("active_anon") {
            metrics.push(Metric::sum(
                "container_memory_active_anon",
                "The amount of anonymous memory that has been identified as active by the kernel.",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("total_active_anon") {
            metrics.push(Metric::sum(
                "container_memory_total_active_anon",
                "The amount of anonymous memory that has been identified as active by the kernel. Includes descendant cgroups (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("inactive_file") {
            metrics.push(Metric::sum(
                "container_memory_inactive_file",
                "Cache memory that has been identified as inactive by the kernel.",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("total_inactive_file") {
            metrics.push(Metric::sum(
                "container_memory_total_inactive_file",
                "Cache memory that has been identified as inactive by the kernel. Includes descendant cgroups (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("active_file") {
            metrics.push(Metric::sum(
                "container_memory_active_file",
                "Cache memory that has been identified as inactive by the kernel. Includes descendant cgroups (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("total_active_file") {
            metrics.push(Metric::sum(
                "container_memory_total_active_file",
                "Cache memory that has been identified as active by the kernel. Includes descendant cgroups (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("unevictable") {
            metrics.push(Metric::sum(
                "container_memory_unevictable",
                "The amount of memory that cannot be reclaimed.",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("total_unevictable") {
            metrics.push(Metric::sum(
                "container_memory_total_unevictable",
                "The amount of memory that cannot be reclaimed. Includes descendant cgroups (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("hierarchical_memory_limit") {
            metrics.push(Metric::sum(
                "container_memory_hierarchical_memory_limit",
                "The maximum amount of physical memory that can be used by the processes of this control group (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("hierarchical_memswap_limit") {
            metrics.push(Metric::sum(
                "container_memory_hierarchical_memswap_limit",
                "The maximum amount of RAM + swap that can be used by the processes of this control group (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("anon") {
            metrics.push(Metric::sum(
                "container_memory_anon",
                "The maximum amount of RAM + swap that can be used by the processes of this control group (Only available with cgroups v1).",
                *value,
            ));
        }
        if let Some(value) = memory_stats.stats.get("file") {
            metrics.push(Metric::sum(
                "container_memory_file",
                "Amount of memory used to cache filesystem data, including tmpfs and shared memory (Only available with cgroups v2).",
                *value,
            ));
        }
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
        tags.insert("image_id", inspect.config.image.clone());

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

fn calculate_cpu_limit(config: &ContainerHostConfig) -> Option<f64> {
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
        online_cpus = match &curr.cpu_usage.percpu_usage {
            None => 0,
            Some(arr) => arr.len() as u32,
        }
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
    if let Some(value) = stats.stats.get("total_inactive_file")
        && *value < stats.usage
    {
        return stats.usage - value;
    }

    // cgroup v2
    if let Some(value) = stats.stats.get("inactive_file") {
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

    for entry in stats.io_merged_recursive.unwrap_or_default() {
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

    for entry in stats.io_queue_recursive.unwrap_or_default() {
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

    for entry in stats.io_service_bytes_recursive.unwrap_or_default() {
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

    for entry in stats.io_service_time_recursive.unwrap_or_default() {
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

    for entry in stats.io_serviced_recursive.unwrap_or_default() {
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

    for entry in stats.io_time_recursive.unwrap_or_default() {
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

    for entry in stats.io_wait_time_recursive.unwrap_or_default() {
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

    for entry in stats.sectors_recursive.unwrap_or_default() {
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
