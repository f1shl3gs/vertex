#![allow(dead_code)]
#[allow(unused)]
#[allow(unused_variables)]
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
mod drm;
mod edac;
mod entropy;
mod errors;
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
mod sockstat;
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
mod xfs;
mod zfs;

use std::io::Read;
use std::str::FromStr;
use std::time::Duration;
use std::{path::Path, sync::Arc};

use configurable::configurable_component;
use event::{tags, Metric};
use framework::config::{
    default_interval, default_true, DataType, Output, SourceConfig, SourceContext,
};
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::Source;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::IntervalStream;
use typetag;

use self::errors::{Error, ErrorContext};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Collectors {
    #[serde(default = "default_true")]
    pub arp: bool,

    #[serde(default = "default_true")]
    pub btrfs: bool,

    #[serde(default = "default_true")]
    pub bonding: bool,

    #[serde(default = "default_true")]
    pub conntrack: bool,

    #[serde(default = "default_cpu_config")]
    pub cpu: Option<cpu::CPUConfig>,

    #[serde(default = "default_true")]
    pub cpufreq: bool,

    #[serde(default = "default_diskstats_config")]
    pub diskstats: Option<diskstats::DiskStatsConfig>,

    #[serde(default)]
    pub drm: bool,

    #[serde(default = "default_true")]
    pub edac: bool,

    #[serde(default = "default_true")]
    pub entropy: bool,

    #[serde(default = "default_true")]
    pub fibrechannel: bool,

    #[serde(default = "default_true")]
    pub filefd: bool,

    #[serde(default = "default_filesystem_config")]
    pub filesystem: Option<filesystem::FileSystemConfig>,

    #[serde(default = "default_true")]
    pub hwmon: bool,

    #[serde(default = "default_true")]
    pub infiniband: bool,

    #[serde(default = "default_ipvs_config")]
    pub ipvs: Option<ipvs::IPVSConfig>,

    #[serde(default = "default_true")]
    pub loadavg: bool,

    #[serde(default = "default_true")]
    pub mdadm: bool,

    #[serde(default = "default_true")]
    pub memory: bool,

    #[serde(default = "default_netclass_config")]
    pub netclass: Option<netclass::NetClassConfig>,

    #[serde(
        default = "default_netdev_config",
        with = "serde_yaml::with::singleton_map"
    )]
    pub netdev: Option<netdev::NetdevConfig>,

    #[serde(default = "default_netstat_config")]
    pub netstat: Option<netstat::NetstatConfig>,

    #[serde(default = "default_true")]
    pub nfs: bool,

    #[serde(default = "default_true")]
    pub nfsd: bool,

    #[serde(default = "default_true")]
    pub nvme: bool,

    #[serde(default = "default_true")]
    pub os_release: bool,

    #[serde(default = "default_powersupply_config")]
    pub power_supply: Option<powersupplyclass::PowerSupplyConfig>,

    #[serde(default = "default_true")]
    pub pressure: bool,

    #[serde(default)]
    pub processes: bool,

    #[serde(default = "default_true")]
    pub rapl: bool,

    #[serde(default = "default_true")]
    pub schedstat: bool,

    #[serde(default = "default_true")]
    pub sockstat: bool,

    #[serde(default = "default_true")]
    pub softnet: bool,

    #[serde(default = "default_true")]
    pub stat: bool,

    #[serde(default = "default_true")]
    pub tcpstat: bool,

    #[serde(default = "default_true")]
    pub thermal_zone: bool,

    #[serde(default = "default_true")]
    pub time: bool,

    #[serde(default = "default_true")]
    pub timex: bool,

    #[serde(default = "default_true")]
    pub udp_queues: bool,

    #[serde(default = "default_true")]
    pub uname: bool,

    #[serde(default = "default_vmstat_config")]
    pub vmstat: Option<vmstat::VMStatConfig>,

    #[serde(default = "default_true")]
    pub xfs: bool,

    #[serde(default = "default_true")]
    pub zfs: bool,

    // MacOS
    #[cfg(target_os = "macos")]
    #[serde(default = "default_true")]
    pub boot_time: bool,
}

fn default_cpu_config() -> Option<cpu::CPUConfig> {
    Some(cpu::CPUConfig::default())
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

impl Default for Collectors {
    fn default() -> Self {
        Self {
            arp: default_true(),
            btrfs: default_true(),
            bonding: default_true(),
            conntrack: default_true(),
            cpu: default_cpu_config(),
            cpufreq: true,
            diskstats: default_diskstats_config(),
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
            sockstat: default_true(),
            softnet: default_true(),
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

/// The Node source generates metrics about the host system scraped
/// from various sources. This is intended to be used when the collector is
/// deployed as an agent, and replace `node_exporter`.
#[configurable_component(source, name = "node")]
#[derive(Debug)]
#[serde(deny_unknown_fields)]
pub struct NodeMetricsConfig {
    /// procfs mountpoint.
    #[serde(default = "default_proc_path")]
    proc_path: String,

    /// sysfs mountpoint.
    #[serde(default = "default_sys_path")]
    sys_path: String,

    /// Duration between each scrape.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    #[serde(default = "default_collectors")]
    #[configurable(skip)]
    collectors: Collectors,
}

fn default_proc_path() -> String {
    "/proc".into()
}

fn default_sys_path() -> String {
    "/sys".into()
}

fn default_collectors() -> Collectors {
    Collectors::default()
}

impl Default for NodeMetricsConfig {
    fn default() -> Self {
        Self {
            proc_path: default_proc_path(),
            sys_path: default_sys_path(),
            interval: default_interval(),
            collectors: default_collectors(),
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct NodeMetrics {
    interval: Duration,
    proc_path: String,
    sys_path: String,

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
pub async fn read_to_string<P: AsRef<Path>>(path: P) -> Result<String, std::io::Error> {
    // let mut f = tokio::fs::File::open(path.as_ref()).await?;
    // let mut content = String::new();
    // f.read_to_string(&mut content).await?;
    // Ok(content)

    let mut file = std::fs::File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    while content.ends_with('\n') || content.ends_with('\t') || content.ends_with(' ') {
        content.pop();
    }

    Ok(content)
}

pub async fn read_into<P, T, E>(path: P) -> Result<T, Error>
where
    P: AsRef<Path>,
    T: FromStr<Err = E>,
    Error: From<E>,
{
    let content = read_to_string(path).await?;
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

        metrics.push(Metric::gauge_with_tags(
            "node_scrape_collector_duration_seconds",
            "Duration of a collector scrape.",
            duration as f64,
            tags! (
                "collector" => $name
            )
        ));
        metrics.push(Metric::gauge_with_tags(
            "node_scrape_collector_success",
            "Whether a collector succeeded.",
            success,
            tags! (
                "collector" => $name
            )
        ));

        metrics
    })
}

impl NodeMetrics {
    async fn run(self, shutdown: ShutdownSignal, mut out: Pipeline) -> Result<(), ()> {
        let interval = tokio::time::interval(self.interval);
        let mut ticker = IntervalStream::new(interval).take_until(shutdown);

        let proc_path = Arc::new(self.proc_path);
        let sys_path = Arc::new(self.sys_path);
        let cpu_conf = self.collectors.cpu.map(Arc::new);
        let diskstats = self.collectors.diskstats.map(Arc::new);
        let filesystem = self.collectors.filesystem.map(Arc::new);
        let ipvs = self.collectors.ipvs.map(Arc::new);
        let netclass = self.collectors.netclass.map(Arc::new);
        let netdev = self.collectors.netdev.map(Arc::new);
        let netstat = self.collectors.netstat.map(Arc::new);
        let power_supply = self.collectors.power_supply.map(Arc::new);
        let vmstat = self.collectors.vmstat.map(Arc::new);

        while ticker.next().await.is_some() {
            let mut tasks = Vec::new();

            if self.collectors.arp {
                let proc_path = Arc::clone(&proc_path);

                tasks.push(tokio::spawn(async move {
                    record_gather!("arp", arp::gather(proc_path.as_ref()))
                }));
            }

            if self.collectors.bonding {
                let sys_path = Arc::clone(&sys_path);

                tasks.push(tokio::spawn(async move {
                    record_gather!("bonding", bonding::gather(sys_path.as_ref()))
                }));
            }

            if self.collectors.btrfs {
                let sys_path = Arc::clone(&proc_path);
                tasks.push(tokio::spawn(async move {
                    record_gather!("btrfs", btrfs::gather(sys_path.as_ref()))
                }))
            }

            if self.collectors.conntrack {
                let proc_path = Arc::clone(&proc_path);

                tasks.push(tokio::spawn(async move {
                    record_gather!("conntrack", conntrack::gather(proc_path.as_ref()))
                }))
            }

            if let Some(ref conf) = cpu_conf {
                let proc_path = Arc::clone(&proc_path);
                let conf = Arc::clone(conf);

                tasks.push(tokio::spawn(async move {
                    record_gather!("cpu", conf.gather(proc_path.as_ref()))
                }));
            }

            if self.collectors.cpufreq {
                let sys_path = Arc::clone(&sys_path);

                tasks.push(tokio::spawn(async move {
                    record_gather!("cpufreq", cpufreq::gather(sys_path.as_ref()))
                }))
            }

            if let Some(ref conf) = diskstats {
                let conf = Arc::clone(conf);
                let proc_path = Arc::clone(&proc_path);

                tasks.push(tokio::spawn(async move {
                    record_gather!("diskstats", conf.gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.drm {
                let sys_path = Arc::clone(&sys_path);
                tasks.push(tokio::spawn(async move {
                    record_gather!("drm", drm::gather(sys_path.as_ref()))
                }))
            }

            if self.collectors.edac {
                let sys_path = Arc::clone(&sys_path);

                tasks.push(tokio::spawn(async move {
                    record_gather!("edac", edac::gather(sys_path.as_ref()))
                }))
            }

            if self.collectors.entropy {
                let proc_path = Arc::clone(&proc_path);

                tasks.push(tokio::spawn(async move {
                    record_gather!("entropy", entropy::gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.fibrechannel {
                let sys_path = Arc::clone(&sys_path);
                tasks.push(tokio::spawn(async move {
                    record_gather!("fibrechannel", fibrechannel::gather(sys_path.as_ref()))
                }))
            }

            if self.collectors.filefd {
                let proc_path = Arc::clone(&proc_path);

                tasks.push(tokio::spawn(async move {
                    record_gather!("filefd", filefd::gather(proc_path.as_ref()))
                }))
            }

            if let Some(ref conf) = filesystem {
                let proc_path = Arc::clone(&proc_path);
                let conf = Arc::clone(conf);

                tasks.push(tokio::spawn(async move {
                    record_gather!("filesystem", conf.gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.hwmon {
                let sys_path = Arc::clone(&sys_path);
                tasks.push(tokio::spawn(async move {
                    record_gather!("hwmon", hwmon::gather(sys_path.as_ref()))
                }))
            }

            if self.collectors.infiniband {
                let sys_path = Arc::clone(&sys_path);
                tasks.push(tokio::spawn(async move {
                    record_gather!("infiniband", infiniband::gather(sys_path.as_ref()))
                }))
            }

            if let Some(ref conf) = ipvs {
                let conf = Arc::clone(conf);
                let proc_path = Arc::clone(&proc_path);
                tasks.push(tokio::spawn(async move {
                    record_gather!("ipvs", ipvs::gather(conf.as_ref(), proc_path.as_ref()))
                }))
            }

            if self.collectors.loadavg {
                let proc_path = Arc::clone(&proc_path);

                tasks.push(tokio::spawn(async move {
                    record_gather!("loadavg", loadavg::gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.mdadm {
                let proc_path = Arc::clone(&proc_path);
                tasks.push(tokio::spawn(async move {
                    record_gather!("mdadm", mdadm::gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.memory {
                let proc_path = Arc::clone(&proc_path);

                tasks.push(tokio::spawn(async move {
                    record_gather!("meminfo", meminfo::gather(proc_path.as_ref()))
                }))
            }

            if let Some(ref conf) = netclass {
                let conf = Arc::clone(conf);
                let sys_path = Arc::clone(&sys_path);

                tasks.push(tokio::spawn(async move {
                    record_gather!(
                        "netclass",
                        netclass::gather(conf.as_ref(), sys_path.as_ref())
                    )
                }))
            }

            if let Some(ref conf) = netdev {
                let conf = Arc::clone(conf);
                let proc_path = Arc::clone(&proc_path);

                tasks.push(tokio::spawn(async move {
                    record_gather!("netdev", conf.gather(proc_path.as_ref()))
                }))
            }

            if let Some(ref conf) = netstat {
                let conf = Arc::clone(conf);
                let proc_path = Arc::clone(&proc_path);

                tasks.push(tokio::spawn(async move {
                    record_gather!(
                        "netstat",
                        netstat::gather(conf.as_ref(), proc_path.as_ref())
                    )
                }))
            }

            if self.collectors.nfs {
                let proc_path = Arc::clone(&proc_path);
                tasks.push(tokio::spawn(async move {
                    record_gather!("nfs", nfs::gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.nfsd {
                let proc_path = Arc::clone(&proc_path);
                tasks.push(tokio::spawn(async move {
                    record_gather!("nfsd", nfsd::gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.nvme {
                let sys_path = Arc::clone(&sys_path);

                tasks.push(tokio::spawn(async move {
                    record_gather!("nvme", nvme::gather(sys_path.as_ref()))
                }))
            }

            if self.collectors.os_release {
                tasks.push(tokio::spawn(async {
                    record_gather!("os", os_release::gather())
                }))
            }

            if let Some(ref conf) = power_supply {
                let sys_path = Arc::clone(&sys_path);
                let conf = Arc::clone(conf);

                tasks.push(tokio::spawn(async move {
                    record_gather!(
                        "powersupplyclass",
                        powersupplyclass::gather(sys_path.as_ref(), conf.as_ref())
                    )
                }))
            }

            if self.collectors.pressure {
                let proc_path = Arc::clone(&sys_path);
                tasks.push(tokio::spawn(async move {
                    record_gather!("pressure", pressure::gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.processes {
                let proc_path = Arc::clone(&proc_path);
                tasks.push(tokio::spawn(async move {
                    record_gather!("processes", processes::gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.rapl {
                let sys_path = Arc::clone(&sys_path);
                tasks.push(tokio::spawn(async move {
                    record_gather!("rapl", rapl::gather(sys_path.as_ref()))
                }))
            }

            if self.collectors.schedstat {
                let proc_path = Arc::clone(&proc_path);
                tasks.push(tokio::spawn(async move {
                    record_gather!("schedstat", schedstat::gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.sockstat {
                let proc_path = Arc::clone(&proc_path);
                tasks.push(tokio::spawn(async move {
                    record_gather!("sockstat", sockstat::gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.softnet {
                let proc_path = Arc::clone(&proc_path);
                tasks.push(tokio::spawn(async move {
                    record_gather!("softnet", softnet::gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.stat {
                let proc_path = Arc::clone(&proc_path);

                tasks.push(tokio::spawn(async move {
                    record_gather!("stat", stat::gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.tcpstat {
                tasks.push(tokio::spawn(async {
                    record_gather!("tcpstat", tcpstat::gather())
                }));
            }

            if self.collectors.thermal_zone {
                let sys_path = Arc::clone(&sys_path);

                tasks.push(tokio::spawn(async move {
                    record_gather!("thermal_zone", thermal_zone::gather(sys_path.as_ref()))
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
                let proc_path = Arc::clone(&proc_path);

                tasks.push(tokio::spawn(async move {
                    record_gather!("udp_queues", udp_queues::gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.uname {
                tasks.push(tokio::spawn(async {
                    record_gather!("uname", uname::gather())
                }))
            }

            if let Some(ref conf) = vmstat {
                let conf = Arc::clone(conf);
                let proc_path = Arc::clone(&proc_path);

                tasks.push(tokio::spawn(async move {
                    record_gather!("vmstat", conf.gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.xfs {
                let sys_path = Arc::clone(&sys_path);
                tasks.push(tokio::spawn(async move {
                    record_gather!("xfs", xfs::gather(sys_path.as_ref()))
                }))
            }

            if self.collectors.zfs {
                let proc_path = Arc::clone(&proc_path);
                tasks.push(tokio::spawn(async move {
                    record_gather!("zfs", zfs::gather(proc_path.as_ref()))
                }))
            }

            // MacOS
            #[cfg(target_os = "macos")]
            if self.boot_time {
                tasks.push(tokio::spawn(async {
                    record_gather!("boot_time", boot_time::gather())
                }));
            }

            let mut metrics = futures::future::join_all(tasks)
                .await
                .iter()
                .flatten()
                .fold(Vec::new(), |mut metrics, ms| {
                    metrics.extend_from_slice(ms);
                    metrics
                });

            let now = chrono::Utc::now();
            metrics.iter_mut().for_each(|metric| {
                metric.timestamp = Some(now);
            });

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
impl SourceConfig for NodeMetricsConfig {
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
        crate::testing::test_generate_config::<NodeMetricsConfig>()
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
