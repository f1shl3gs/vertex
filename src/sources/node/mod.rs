#![allow(dead_code)]

mod arp;
mod bcache;
mod bonding;
#[cfg(target_os = "macos")]
mod boot_time;
mod btrfs;
mod conntrack;
mod cpu;
mod cpufreq;
mod diskstats;
mod dmi;
mod drm;
mod edac;
mod entropy;
mod error;
mod fibrechannel;
mod filefd;
mod filesystem;
pub mod hwmon;
mod infiniband;
mod ipvs;
mod lnstat;
mod loadavg;
mod mdadm;
mod meminfo;
mod netclass;
mod netdev;
mod netstat;
mod network_route;
mod nfs;
mod nfsd;
mod nvme;
mod os_release;
mod powersupplyclass;
mod pressure;
mod processes;
mod protocols;
mod rapl;
mod schedstat;
mod selinux;
mod sockstat;
mod softirqs;
mod softnet;
mod stat;
mod tapestats;
mod tcpstat;
mod thermal_zone;
mod time;
mod timex;
mod udp_queues;
mod uname;
mod vmstat;
mod wifi;
#[cfg(target_os = "linux")]
mod xfs;
mod zfs;

use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use configurable::configurable_component;
use error::Error;
use event::{tags, tags::Key, Metric};
use framework::config::{
    default_interval, default_true, DataType, Output, SourceConfig, SourceContext,
};
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::Source;
use futures::stream::FuturesUnordered;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

fn default_cpu_config() -> Option<cpu::CPUConfig> {
    Some(cpu::CPUConfig::default())
}

fn default_bcache_config() -> Option<bcache::Config> {
    Some(bcache::Config::default())
}

fn default_diskstats_config() -> Option<diskstats::DiskStatsConfig> {
    Some(diskstats::DiskStatsConfig::default())
}

fn default_filesystem_config() -> Option<filesystem::FileSystemConfig> {
    Some(filesystem::FileSystemConfig::default())
}

fn default_ipvs_config() -> Option<ipvs::IPVSConfig> {
    Some(ipvs::IPVSConfig::default())
}

fn default_netclass_config() -> Option<netclass::NetClassConfig> {
    Some(netclass::NetClassConfig::default())
}

fn default_netdev_config() -> Option<netdev::NetdevConfig> {
    Some(netdev::NetdevConfig::default())
}

fn default_netstat_config() -> Option<netstat::NetstatConfig> {
    Some(netstat::NetstatConfig::default())
}

fn default_powersupply_config() -> Option<powersupplyclass::PowerSupplyConfig> {
    Some(powersupplyclass::PowerSupplyConfig::default())
}

fn default_vmstat_config() -> Option<vmstat::VMStatConfig> {
    Some(vmstat::VMStatConfig::default())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Collectors {
    #[serde(default = "default_true")]
    arp: bool,

    #[serde(default = "default_bcache_config")]
    bcache: Option<bcache::Config>,

    #[serde(default = "default_true")]
    bonding: bool,

    #[serde(default = "default_true")]
    btrfs: bool,

    #[serde(default = "default_true")]
    conntrack: bool,

    #[serde(default = "default_cpu_config")]
    cpu: Option<cpu::CPUConfig>,

    #[serde(default = "default_true")]
    cpufreq: bool,

    #[serde(default = "default_diskstats_config")]
    diskstats: Option<diskstats::DiskStatsConfig>,

    #[serde(default = "default_true")]
    dmi: bool,

    #[serde(default)]
    drm: bool,

    #[serde(default = "default_true")]
    edac: bool,

    #[serde(default = "default_true")]
    entropy: bool,

    #[serde(default = "default_true")]
    fibrechannel: bool,

    #[serde(default = "default_true")]
    filefd: bool,

    #[serde(default = "default_filesystem_config")]
    filesystem: Option<filesystem::FileSystemConfig>,

    #[serde(default = "default_true")]
    hwmon: bool,

    #[serde(default = "default_true")]
    infiniband: bool,

    #[serde(default = "default_ipvs_config")]
    ipvs: Option<ipvs::IPVSConfig>,

    #[serde(default = "default_true")]
    loadavg: bool,

    #[serde(default = "default_true")]
    mdadm: bool,

    #[serde(default = "default_true")]
    memory: bool,

    #[serde(default = "default_netclass_config")]
    netclass: Option<netclass::NetClassConfig>,

    #[serde(
        default = "default_netdev_config",
        with = "serde_yaml::with::singleton_map"
    )]
    netdev: Option<netdev::NetdevConfig>,

    #[serde(default = "default_netstat_config")]
    netstat: Option<netstat::NetstatConfig>,

    #[serde(default = "default_true")]
    nfs: bool,

    #[serde(default = "default_true")]
    nfsd: bool,

    #[serde(default = "default_true")]
    nvme: bool,

    #[serde(default = "default_true")]
    os_release: bool,

    #[serde(default = "default_powersupply_config")]
    power_supply: Option<powersupplyclass::PowerSupplyConfig>,

    #[serde(default = "default_true")]
    pressure: bool,

    #[serde(default)]
    processes: bool,

    #[serde(default = "default_true")]
    rapl: bool,

    #[serde(default = "default_true")]
    schedstat: bool,

    #[serde(default = "default_true")]
    selinux: bool,

    #[serde(default = "default_true")]
    sockstat: bool,

    #[serde(default = "default_true")]
    softnet: bool,

    #[serde(default)]
    softirqs: bool,

    #[serde(default = "default_true")]
    stat: bool,

    #[serde(default = "default_true")]
    tcpstat: bool,

    #[serde(default = "default_true")]
    thermal_zone: bool,

    #[serde(default = "default_true")]
    time: bool,

    #[serde(default = "default_true")]
    timex: bool,

    #[serde(default = "default_true")]
    udp_queues: bool,

    #[serde(default = "default_true")]
    uname: bool,

    #[serde(default = "default_vmstat_config")]
    vmstat: Option<vmstat::VMStatConfig>,

    #[cfg(target_os = "linux")]
    #[serde(default = "default_true")]
    xfs: bool,

    #[serde(default = "default_true")]
    zfs: bool,

    // MacOS
    #[cfg(target_os = "macos")]
    #[serde(default = "default_true")]
    boot_time: bool,
}

impl Default for Collectors {
    fn default() -> Self {
        Self {
            arp: default_true(),
            bcache: default_bcache_config(),
            btrfs: default_true(),
            bonding: default_true(),
            conntrack: default_true(),
            cpu: default_cpu_config(),
            cpufreq: true,
            diskstats: default_diskstats_config(),
            dmi: default_true(),
            drm: default_true(),
            edac: default_true(),
            entropy: default_true(),
            fibrechannel: default_true(),
            filefd: default_true(),
            filesystem: default_filesystem_config(),
            hwmon: default_true(),
            infiniband: default_true(),
            ipvs: default_ipvs_config(),
            loadavg: default_true(),
            mdadm: default_true(),
            memory: default_true(),
            netclass: default_netclass_config(),
            netdev: default_netdev_config(),
            netstat: default_netstat_config(),
            nfs: default_true(),
            nfsd: default_true(),
            nvme: default_true(),
            os_release: default_true(),
            power_supply: default_powersupply_config(),
            pressure: default_true(),
            processes: false,
            rapl: default_true(),
            schedstat: default_true(),
            selinux: default_true(),
            sockstat: default_true(),
            softnet: default_true(),
            softirqs: false,
            stat: default_true(),
            time: default_true(),
            timex: default_true(),
            tcpstat: default_true(),
            thermal_zone: default_true(),
            udp_queues: default_true(),
            uname: default_true(),
            vmstat: default_vmstat_config(),
            xfs: default_true(),
            zfs: default_true(),

            // MacOS
            #[cfg(target_os = "macos")]
            boot_time: default_true(),
        }
    }
}

fn default_proc_path() -> PathBuf {
    "/proc".into()
}

fn default_sys_path() -> PathBuf {
    "/sys".into()
}

/// The Node source generates metrics about the host system scraped
/// from various sources. This is intended to be used when the collector is
/// deployed as an agent, and replace `node_exporter`.
#[configurable_component(source, name = "node")]
#[serde(deny_unknown_fields)]
struct Config {
    /// procfs mountpoint.
    #[serde(default = "default_proc_path")]
    proc_path: PathBuf,

    /// sysfs mountpoint.
    #[serde(default = "default_sys_path")]
    sys_path: PathBuf,

    /// Duration between each scrape.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    #[serde(default, flatten)]
    #[configurable(skip)]
    collectors: Collectors,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            proc_path: default_proc_path(),
            sys_path: default_sys_path(),
            interval: default_interval(),
            collectors: Collectors::default(),
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct NodeMetrics {
    interval: Duration,
    proc_path: PathBuf,
    sys_path: PathBuf,

    collectors: Collectors,
}

/// `read_to_string` should be a real async function, but the implement of
/// `tokio::fs::read_to_string` will spawn a thread for reading files, which actually
/// increase cpu and memory usage. The `tokio-uring` might be help, and it should be
/// introduced once it's ready.
///
/// The files this function will(should) be reading is under `/sys` and `/proc` which is
/// relative small and the filesystem is kind of `tmpfs`, so the performance should never
/// be a problem.
pub fn read_to_string<P: AsRef<Path>>(path: P) -> Result<String, std::io::Error> {
    let mut content = std::fs::read_to_string(path)?;

    while content.ends_with('\n') || content.ends_with('\t') || content.ends_with(' ') {
        content.pop();
    }

    Ok(content)
}

pub fn read_into<P, T, E>(path: P) -> Result<T, Error>
where
    P: AsRef<Path>,
    T: FromStr<Err = E>,
    Error: From<E>,
{
    let content = read_to_string(path)?;
    Ok(<T as FromStr>::from_str(content.as_str())?)
}

macro_rules! record_gather {
    ($name: expr, $future: expr) => ({
        let start = std::time::SystemTime::now();
        let r = $future.await;
        let duration = std::time::SystemTime::now()
            .duration_since(start)
            .unwrap()
            .as_secs_f64();
        let (mut metrics, success) = match r {
            Ok(ms) => (ms, 1.0),
            Err(err) => {
                debug!("gather metrics failed, {}: {}", $name, err);
                (vec![], 0.0)
            },
        };

        metrics.extend([
            Metric::gauge_with_tags(
                "node_scrape_collector_duration_seconds",
                "Duration of a collector scrape.",
                duration,
                tags! (
                    Key::from_static("collector") => $name
                )
            ),
            Metric::gauge_with_tags(
                "node_scrape_collector_success",
                "Whether a collector succeeded.",
                success,
                tags! (
                    Key::from_static("collector") => $name
                )
            )
        ]);

        metrics
    })
}

impl NodeMetrics {
    async fn run(self, mut shutdown: ShutdownSignal, mut out: Pipeline) -> Result<(), ()> {
        let mut ticker = tokio::time::interval(self.interval);

        loop {
            tokio::select! {
                biased;

                _ = &mut shutdown => break,
                _ = ticker.tick() => {}
            }

            let mut tasks = FuturesUnordered::new();

            if self.collectors.arp {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("arp", arp::gather(proc_path))
                }));
            }

            if let Some(conf) = &self.collectors.bcache {
                let sys_path = self.sys_path.clone();
                let conf = conf.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("bcache", bcache::gather(conf, sys_path))
                }))
            }

            if self.collectors.bonding {
                let sys_path = self.sys_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("bonding", bonding::gather(sys_path))
                }));
            }

            if self.collectors.btrfs {
                let sys_path = self.sys_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("btrfs", btrfs::gather(sys_path))
                }))
            }

            if self.collectors.conntrack {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("conntrack", conntrack::gather(proc_path))
                }))
            }

            if let Some(conf) = &self.collectors.cpu {
                let proc_path = self.proc_path.clone();
                let conf = conf.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("cpu", cpu::gather(conf, proc_path))
                }));
            }

            if self.collectors.cpufreq {
                let sys_path = self.sys_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("cpufreq", cpufreq::gather(sys_path))
                }))
            }

            if let Some(conf) = &self.collectors.diskstats {
                let proc_path = self.proc_path.clone();
                let conf = conf.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("diskstats", diskstats::gather(conf, proc_path))
                }))
            }

            if self.collectors.dmi {
                let sys_path = self.sys_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("dmi", dmi::gather(sys_path))
                }))
            }

            if self.collectors.drm {
                let sys_path = self.sys_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("drm", drm::gather(sys_path))
                }))
            }

            if self.collectors.edac {
                let sys_path = self.sys_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("edac", edac::gather(sys_path))
                }))
            }

            if self.collectors.entropy {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("entropy", entropy::gather(proc_path))
                }))
            }

            if self.collectors.fibrechannel {
                let sys_path = self.sys_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("fibrechannel", fibrechannel::gather(sys_path))
                }))
            }

            if self.collectors.filefd {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("filefd", filefd::gather(proc_path))
                }))
            }

            if let Some(conf) = &self.collectors.filesystem {
                let proc_path = self.proc_path.clone();
                let conf = conf.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("filesystem", filesystem::gather(conf, proc_path))
                }))
            }

            if self.collectors.hwmon {
                let sys_path = self.sys_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("hwmon", hwmon::gather(sys_path))
                }))
            }

            if self.collectors.infiniband {
                let sys_path = self.sys_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("infiniband", infiniband::gather(sys_path))
                }))
            }

            if let Some(conf) = &self.collectors.ipvs {
                let proc_path = self.proc_path.clone();
                let conf = conf.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("ipvs", ipvs::gather(conf, proc_path))
                }))
            }

            if self.collectors.loadavg {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("loadavg", loadavg::gather(proc_path))
                }))
            }

            if self.collectors.mdadm {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("mdadm", mdadm::gather(proc_path))
                }))
            }

            if self.collectors.memory {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("meminfo", meminfo::gather(proc_path))
                }))
            }

            if let Some(conf) = &self.collectors.netclass {
                let sys_path = self.sys_path.clone();
                let conf = conf.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("netclass", netclass::gather(conf, sys_path))
                }))
            }

            if let Some(conf) = &self.collectors.netdev {
                let proc_path = self.proc_path.clone();
                let conf = conf.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("netdev", netdev::gather(conf, proc_path))
                }))
            }

            if let Some(conf) = &self.collectors.netstat {
                let proc_path = self.proc_path.clone();
                let conf = conf.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("netstat", netstat::gather(conf, proc_path))
                }))
            }

            if self.collectors.nfs {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("nfs", nfs::gather(proc_path))
                }))
            }

            if self.collectors.nfsd {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("nfsd", nfsd::gather(proc_path))
                }))
            }

            if self.collectors.nvme {
                let sys_path = self.sys_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("nvme", nvme::gather(sys_path))
                }))
            }

            if self.collectors.os_release {
                tasks.push(tokio::spawn(async {
                    record_gather!("os", os_release::gather())
                }))
            }

            if let Some(conf) = &self.collectors.power_supply {
                let conf = conf.clone();
                let sys_path = self.sys_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("powersupplyclass", powersupplyclass::gather(conf, sys_path))
                }))
            }

            if self.collectors.pressure {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("pressure", pressure::gather(proc_path))
                }))
            }

            if self.collectors.processes {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("processes", processes::gather(proc_path))
                }))
            }

            if self.collectors.rapl {
                let sys_path = self.sys_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("rapl", rapl::gather(sys_path))
                }))
            }

            if self.collectors.schedstat {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("schedstat", schedstat::gather(proc_path))
                }))
            }

            if self.collectors.selinux {
                let proc_path = self.proc_path.clone();
                let sys_path = self.sys_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("selinux", selinux::gather(proc_path, sys_path))
                }))
            }

            if self.collectors.sockstat {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("sockstat", sockstat::gather(proc_path))
                }))
            }

            if self.collectors.softnet {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("softnet", softnet::gather(proc_path))
                }))
            }

            if self.collectors.softirqs {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("softirqs", softirqs::gather(proc_path))
                }))
            }

            if self.collectors.stat {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("stat", stat::gather(proc_path))
                }))
            }

            if self.collectors.tcpstat {
                tasks.push(tokio::spawn(async {
                    record_gather!("tcpstat", tcpstat::gather())
                }));
            }

            if self.collectors.thermal_zone {
                let sys_path = self.sys_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("thermal_zone", thermal_zone::gather(sys_path))
                }))
            }

            if self.collectors.time {
                tasks.push(tokio::spawn(async {
                    record_gather!("time", time::gather())
                }))
            }

            if self.collectors.timex {
                tasks.push(tokio::spawn(async {
                    record_gather!("timex", timex::gather())
                }))
            }

            if self.collectors.udp_queues {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("udp_queues", udp_queues::gather(proc_path))
                }))
            }

            if self.collectors.uname {
                tasks.push(tokio::spawn(async {
                    record_gather!("uname", uname::gather())
                }))
            }

            if let Some(conf) = &self.collectors.vmstat {
                let proc_path = self.proc_path.clone();
                let conf = conf.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("vmstat", vmstat::gather(conf, proc_path))
                }))
            }

            #[cfg(target_os = "linux")]
            if self.collectors.xfs {
                let sys_path = self.sys_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("xfs", xfs::gather(sys_path))
                }))
            }

            if self.collectors.zfs {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("zfs", zfs::gather(proc_path))
                }))
            }

            // MacOS
            #[cfg(target_os = "macos")]
            if self.boot_time {
                tasks.push(tokio::spawn(async {
                    record_gather!("boot_time", boot_time::gather())
                }));
            }

            let mut metrics = vec![];
            while let Some(Ok(mut parts)) = tasks.next().await {
                let now = chrono::Utc::now();
                parts
                    .iter_mut()
                    .for_each(|metric| metric.timestamp = Some(now));

                metrics.extend(parts);
            }

            if let Err(err) = out.send(metrics).await {
                error!(message = "Error sending node metrics", ?err);
                return Err(());
            }
        }

        Ok(())
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "node_metrics")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let nm = NodeMetrics {
            interval: self.interval,
            proc_path: self.proc_path.clone(),
            sys_path: self.sys_path.clone(),
            collectors: self.collectors.clone(),
        };

        Ok(Box::pin(nm.run(cx.shutdown, cx.output)))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }

    #[test]
    fn test_deserialize() {
        let cs: Collectors = serde_yaml::from_str(
            r#"
        arp: true
        "#,
        )
        .unwrap();

        assert!(cs.arp);
    }
}
