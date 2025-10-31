use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Deserialize;

use super::{Client, Error, encode_filters};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Container {
    pub id: String,
    pub image: String,
    #[serde(rename = "ImageID")]
    pub image_id: String,
}

#[derive(Default)]
pub struct ListContainersOptions<T> {
    pub all: bool,
    pub limit: Option<usize>,
    pub filters: Option<HashMap<T, Vec<T>>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerState {
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerHostConfig {
    /// relative weight vs. other containers
    pub cpu_shares: i64,
    // /// Limits in bytes
    // memory: i64,
    /// CPU quota in units of 10<sup>-9</sup> CPUs
    pub nano_cpus: i64,
    pub cpu_period: i64,
    pub cpu_quota: i64,
    /// CpusetCpus 0-2, 0,1
    pub cpuset_cpus: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerConfig {
    pub image: String,
    pub hostname: String,

    #[serde(default)]
    pub cmd: Option<Vec<String>>,
    pub labels: HashMap<String, String>,
    pub exposed_ports: HashMap<String, HashMap<(), ()>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Network {
    #[serde(rename = "IPAddress")]
    pub ip_address: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NetworkSettings {
    pub networks: HashMap<String, Network>,
    // ports: HashMap<String, Option<Vec<Port>>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerInspect {
    pub name: String,
    pub restart_count: u64,
    pub state: ContainerState,
    pub host_config: ContainerHostConfig,
    pub config: ContainerConfig,
    pub network_settings: NetworkSettings,
}

#[derive(Debug, Deserialize)]
pub struct PidsStats {
    /// The number of pids in the cgroup
    pub current: u64,
    /// The hard limit on the number of pids in the cgroup
    pub limit: u64,
}

/// One small entity to store a piece of Blkio stats
#[derive(Debug, Deserialize)]
pub struct BlkioStatsEntry {
    pub major: u64,
    pub minor: u64,
    pub op: String,
    pub value: u64,
}

/// All IO service stats for data read and write. This is a Linux specific structure as
/// the differences between expressing block I/O on Windows and Linux are sufficiently
/// significant to make little sense attempting to morph into a combined structure.
#[derive(Debug, Deserialize)]
pub struct BlkioStats {
    pub io_service_bytes_recursive: Option<Vec<BlkioStatsEntry>>,
    pub io_serviced_recursive: Option<Vec<BlkioStatsEntry>>,
    pub io_queue_recursive: Option<Vec<BlkioStatsEntry>>,
    pub io_service_time_recursive: Option<Vec<BlkioStatsEntry>>,
    pub io_wait_time_recursive: Option<Vec<BlkioStatsEntry>>,
    pub io_merged_recursive: Option<Vec<BlkioStatsEntry>>,
    pub io_time_recursive: Option<Vec<BlkioStatsEntry>>,
    pub sectors_recursive: Option<Vec<BlkioStatsEntry>>,
}

/// All CPU stats aggregated since container inception
#[derive(Debug, Deserialize)]
pub struct CpuUsage {
    /// Total CPU time consumed per core (Linux). Not used on Windows
    ///
    /// Units: nanoseconds
    pub percpu_usage: Option<Vec<u64>>,

    /// Time spent by tasks of the cgroup in user mode (Linux)
    /// Time spent by all container processes in user mode (Windows)
    ///
    /// nanoseconds on Linux
    /// 100's of nanoseconds on Windows
    pub usage_in_usermode: u64,

    /// Total CPU time consumed
    ///
    /// nanoseconds on Linux
    /// 100's of nanoseconds on Windows
    pub total_usage: u64,

    /// Time spent by tasks of the cgroup in kernel mode (Linux).
    /// Time spent by all containers processes in kernel mode (Windows)
    ///
    /// nanoseconds on Linux
    /// 100's of nanoseconds on Windows
    pub usage_in_kernelmode: u64,
}

///
/// Not used on Windows
#[derive(Debug, Deserialize)]
pub struct ThrottlingData {
    /// Number of periods with throttling active
    pub periods: u64,

    /// Number of periods when the container hits its throttling limit.
    pub throttled_periods: u64,

    /// Aggregate time the container was throttled for in nanoseconds
    pub throttled_time: u64,
}

#[derive(Debug, Deserialize)]
pub struct CpuStats {
    /// CPU Usage
    pub cpu_usage: CpuUsage,
    /// System Usage, linux only
    pub system_cpu_usage: u64,
    /// Online CPUs, linux only
    pub online_cpus: u32,
    /// Throttling Data, linux only
    pub throttling_data: ThrottlingData,
}

#[derive(Debug, Deserialize)]
pub struct MemoryStats {
    /// Export these as stronger types, all the stats exported via memory.stat
    #[serde(default)]
    pub stats: HashMap<String, u64>,

    /// maximum usage ever recorded.
    pub max_usage: Option<u64>,

    /// current res_counter usage for memory
    pub usage: u64,

    /// Number of times memory usage hits limits
    pub failcnt: Option<u64>,

    /// This field is Linux-specific and omitted for Windows containers.
    pub limit: u64,
}

/// The network stats of one container
#[derive(Debug, Deserialize)]
pub struct NetworkStats {
    /// Bytes received
    pub rx_bytes: u64,

    /// Incoming packets dropped.
    pub rx_dropped: u64,

    /// Received errors. Not used on Windows. Note that we don't `omitempty` this field
    /// as it is expected in the >= v1.21 API stats structure.
    pub rx_errors: u64,

    /// Packets received
    pub rx_packets: u64,

    /// Bytes sent
    pub tx_bytes: u64,

    /// Outgoing packets dropped
    pub tx_dropped: u64,

    /// Sent errors. Not used on Windows. Note that we don't `omitempty` this field as it
    /// is expected in the >= v1.21 API stats structure.
    pub tx_errors: u64,

    /// Packets sent.
    pub tx_packets: u64,
}

#[derive(Debug, Deserialize)]
pub struct ContainerStats {
    /// ID of the container
    pub id: String,
    /// Name of the container
    pub name: String,

    pub pids_stats: PidsStats,
    pub blkio_stats: BlkioStats,

    pub cpu_stats: CpuStats,
    pub precpu_stats: CpuStats,
    pub memory_stats: Option<MemoryStats>,

    /// Network statistics for the container per interface.  This field is omitted if the container has no networking enabled.
    #[serde(default)]
    pub networks: HashMap<String, NetworkStats>,
}

impl Client {
    /// List Containers
    pub async fn list_containers<T: serde::Serialize>(
        &self,
        opts: ListContainersOptions<T>,
    ) -> Result<Vec<Container>, Error> {
        let mut params = Vec::new();
        if opts.all {
            params.push("all=true".to_string());
        }
        if let Some(limit) = opts.limit {
            params.push(format!("limit={}", limit));
        }
        if let Some(filters) = opts.filters
            && !filters.is_empty()
        {
            let encoded = encode_filters(&filters);
            params.push(format!("filters={}", encoded));
        }

        let uri = if params.is_empty() {
            "http://localhost/containers/json".to_string()
        } else {
            format!("http://localhost/containers/json?{}", params.join("&"))
        };

        self.fetch(uri).await
    }

    pub async fn inspect_container(&self, id: &str) -> Result<ContainerInspect, Error> {
        self.fetch(format!("http://localhost/containers/{id}/json"))
            .await
    }

    pub async fn stats(&self, id: &str) -> Result<ContainerStats, Error> {
        let uri = format!("http://localhost/containers/{id}/stats?stream=false");
        self.fetch(uri).await
    }
}
