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
mod watchdog;
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
use framework::config::{default_interval, default_true, Output, SourceConfig, SourceContext};
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::Source;
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;

fn default_cpu_config() -> Option<cpu::Config> {
    Some(cpu::Config::default())
}

fn default_bcache_config() -> Option<bcache::Config> {
    Some(bcache::Config::default())
}

fn default_diskstats_config() -> Option<diskstats::Config> {
    Some(diskstats::Config::default())
}

fn default_filesystem_config() -> Option<filesystem::Config> {
    Some(filesystem::Config::default())
}

fn default_ipvs_config() -> Option<ipvs::Config> {
    Some(ipvs::Config::default())
}

fn default_netclass_config() -> Option<netclass::Config> {
    Some(netclass::Config::default())
}

fn default_netdev_config() -> Option<netdev::Config> {
    Some(netdev::Config::default())
}

fn default_netstat_config() -> Option<netstat::Config> {
    Some(netstat::Config::default())
}

fn default_powersupply_config() -> Option<powersupplyclass::Config> {
    Some(powersupplyclass::Config::default())
}

fn default_vmstat_config() -> Option<vmstat::Config> {
    Some(vmstat::Config::default())
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
    cpu: Option<cpu::Config>,

    #[serde(default = "default_true")]
    cpufreq: bool,

    #[serde(default = "default_diskstats_config")]
    diskstats: Option<diskstats::Config>,

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
    filesystem: Option<filesystem::Config>,

    #[serde(default = "default_true")]
    hwmon: bool,

    #[serde(default = "default_true")]
    infiniband: bool,

    #[serde(default = "default_ipvs_config")]
    ipvs: Option<ipvs::Config>,

    #[serde(default = "default_true")]
    loadavg: bool,

    #[serde(default = "default_true")]
    mdadm: bool,

    #[serde(default = "default_true")]
    memory: bool,

    #[serde(default = "default_netclass_config")]
    netclass: Option<netclass::Config>,

    #[serde(
        default = "default_netdev_config",
        with = "serde_yaml::with::singleton_map"
    )]
    netdev: Option<netdev::Config>,

    #[serde(default = "default_netstat_config")]
    netstat: Option<netstat::Config>,

    #[serde(default = "default_true")]
    nfs: bool,

    #[serde(default = "default_true")]
    nfsd: bool,

    #[serde(default = "default_true")]
    nvme: bool,

    #[serde(default = "default_true")]
    os_release: bool,

    #[serde(default = "default_powersupply_config")]
    power_supply: Option<powersupplyclass::Config>,

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
    vmstat: Option<vmstat::Config>,

    #[serde(default = "default_true")]
    watchdog: bool,

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
            arp: true,
            bcache: default_bcache_config(),
            btrfs: true,
            bonding: true,
            conntrack: true,
            cpu: default_cpu_config(),
            cpufreq: true,
            diskstats: default_diskstats_config(),
            dmi: true,
            drm: true,
            edac: true,
            entropy: true,
            fibrechannel: true,
            filefd: true,
            filesystem: default_filesystem_config(),
            hwmon: true,
            infiniband: true,
            ipvs: default_ipvs_config(),
            loadavg: true,
            mdadm: true,
            memory: true,
            netclass: default_netclass_config(),
            netdev: default_netdev_config(),
            netstat: default_netstat_config(),
            nfs: true,
            nfsd: true,
            nvme: true,
            os_release: true,
            power_supply: default_powersupply_config(),
            pressure: true,
            processes: false,
            rapl: true,
            schedstat: true,
            selinux: true,
            sockstat: true,
            softnet: true,
            softirqs: false,
            stat: true,
            time: true,
            timex: true,
            tcpstat: true,
            thermal_zone: true,
            udp_queues: true,
            uname: true,
            vmstat: default_vmstat_config(),
            watchdog: true,
            xfs: true,
            zfs: true,

            // MacOS
            #[cfg(target_os = "macos")]
            boot_time: true,
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
    let mut data = std::fs::read(path)?;

    let mut len = data.len();
    while len > 0 {
        match data[len - 1] {
            b'\t' | b'\n' | b'\x0C' | b'\r' | b' ' => {}
            _ => {
                data.truncate(len);
                break;
            }
        }

        len -= 1;
    }

    Ok(unsafe { String::from_utf8_unchecked(data) })
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
        let result = $future.await;
        let duration = std::time::SystemTime::now()
            .duration_since(start)
            .unwrap()
            .as_secs_f64();
        let (mut metrics, success) = match result {
            Ok(ms) => (ms, 1.0),
            Err(err) => {
                debug!(
                    message = "gather metrics failed",
                    collector = $name,
                    %err,
                );

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

async fn run(
    interval: Duration,
    proc_path: PathBuf,
    sys_path: PathBuf,
    collectors: Collectors,
    mut shutdown: ShutdownSignal,
    mut out: Pipeline,
) -> Result<(), ()> {
    let mut ticker = tokio::time::interval(interval);

    loop {
        tokio::select! {
            biased;

            _ = &mut shutdown => break,
            _ = ticker.tick() => {}
        }

        let mut tasks = JoinSet::new();

        if collectors.arp {
            let proc_path = proc_path.clone();
            tasks.spawn(async move { record_gather!("arp", arp::gather(proc_path)) });
        }

        if let Some(conf) = &collectors.bcache {
            let sys_path = sys_path.clone();
            let conf = conf.clone();
            tasks.spawn(async move { record_gather!("bcache", bcache::gather(conf, sys_path)) });
        }

        if collectors.bonding {
            let sys_path = sys_path.clone();
            tasks.spawn(async move { record_gather!("bonding", bonding::gather(sys_path)) });
        }

        if collectors.btrfs {
            let sys_path = sys_path.clone();
            tasks.spawn(async move { record_gather!("btrfs", btrfs::gather(sys_path)) });
        }

        if collectors.conntrack {
            let proc_path = proc_path.clone();
            tasks.spawn(async move { record_gather!("conntrack", conntrack::gather(proc_path)) });
        }

        if let Some(conf) = &collectors.cpu {
            let proc_path = proc_path.clone();
            let conf = conf.clone();
            tasks.spawn(async move { record_gather!("cpu", cpu::gather(conf, proc_path)) });
        }

        if collectors.cpufreq {
            let sys_path = sys_path.clone();
            tasks.spawn(async move { record_gather!("cpufreq", cpufreq::gather(sys_path)) });
        }

        if let Some(conf) = &collectors.diskstats {
            let proc_path = proc_path.clone();
            let conf = conf.clone();
            tasks.spawn(
                async move { record_gather!("diskstats", diskstats::gather(conf, proc_path)) },
            );
        }

        if collectors.dmi {
            let sys_path = sys_path.clone();
            tasks.spawn(async move { record_gather!("dmi", dmi::gather(sys_path)) });
        }

        if collectors.drm {
            let sys_path = sys_path.clone();
            tasks.spawn(async move { record_gather!("drm", drm::gather(sys_path)) });
        }

        if collectors.edac {
            let sys_path = sys_path.clone();
            tasks.spawn(async move { record_gather!("edac", edac::gather(sys_path)) });
        }

        if collectors.entropy {
            let proc_path = proc_path.clone();
            tasks.spawn(async move { record_gather!("entropy", entropy::gather(proc_path)) });
        }

        if collectors.fibrechannel {
            let sys_path = sys_path.clone();
            tasks.spawn(
                async move { record_gather!("fibrechannel", fibrechannel::gather(sys_path)) },
            );
        }

        if collectors.filefd {
            let proc_path = proc_path.clone();
            tasks.spawn(async move { record_gather!("filefd", filefd::gather(proc_path)) });
        }

        if let Some(conf) = &collectors.filesystem {
            let proc_path = proc_path.clone();
            let conf = conf.clone();
            tasks.spawn(async move {
                record_gather!("filesystem", filesystem::gather(conf, proc_path))
            });
        }

        if collectors.hwmon {
            let sys_path = sys_path.clone();
            tasks.spawn(async move { record_gather!("hwmon", hwmon::gather(sys_path)) });
        }

        if collectors.infiniband {
            let sys_path = sys_path.clone();
            tasks.spawn(async move { record_gather!("infiniband", infiniband::gather(sys_path)) });
        }

        if let Some(conf) = &collectors.ipvs {
            let proc_path = proc_path.clone();
            let conf = conf.clone();
            tasks.spawn(async move { record_gather!("ipvs", ipvs::gather(conf, proc_path)) });
        }

        if collectors.loadavg {
            let proc_path = proc_path.clone();
            tasks.spawn(async move { record_gather!("loadavg", loadavg::gather(proc_path)) });
        }

        if collectors.mdadm {
            let proc_path = proc_path.clone();
            tasks.spawn(async move { record_gather!("mdadm", mdadm::gather(proc_path)) });
        }

        if collectors.memory {
            let proc_path = proc_path.clone();
            tasks.spawn(async move { record_gather!("meminfo", meminfo::gather(proc_path)) });
        }

        if let Some(conf) = &collectors.netclass {
            let sys_path = sys_path.clone();
            let conf = conf.clone();
            tasks
                .spawn(async move { record_gather!("netclass", netclass::gather(conf, sys_path)) });
        }

        if let Some(conf) = &collectors.netdev {
            let proc_path = proc_path.clone();
            let conf = conf.clone();
            tasks.spawn(async move { record_gather!("netdev", netdev::gather(conf, proc_path)) });
        }

        if let Some(conf) = &collectors.netstat {
            let proc_path = proc_path.clone();
            let conf = conf.clone();
            tasks.spawn(async move { record_gather!("netstat", netstat::gather(conf, proc_path)) });
        }

        if collectors.nfs {
            let proc_path = proc_path.clone();
            tasks.spawn(async move { record_gather!("nfs", nfs::gather(proc_path)) });
        }

        if collectors.nfsd {
            let proc_path = proc_path.clone();
            tasks.spawn(async move { record_gather!("nfsd", nfsd::gather(proc_path)) });
        }

        if collectors.nvme {
            let sys_path = sys_path.clone();
            tasks.spawn(async move { record_gather!("nvme", nvme::gather(sys_path)) });
        }

        if collectors.os_release {
            tasks.spawn(async { record_gather!("os", os_release::gather()) });
        }

        if let Some(conf) = &collectors.power_supply {
            let conf = conf.clone();
            let sys_path = sys_path.clone();
            tasks.spawn(async move {
                record_gather!("powersupplyclass", powersupplyclass::gather(conf, sys_path))
            });
        }

        if collectors.pressure {
            let proc_path = proc_path.clone();
            tasks.spawn(async move { record_gather!("pressure", pressure::gather(proc_path)) });
        }

        if collectors.processes {
            let proc_path = proc_path.clone();
            tasks.spawn(async move { record_gather!("processes", processes::gather(proc_path)) });
        }

        if collectors.rapl {
            let sys_path = sys_path.clone();
            tasks.spawn(async move { record_gather!("rapl", rapl::gather(sys_path)) });
        }

        if collectors.schedstat {
            let proc_path = proc_path.clone();
            tasks.spawn(async move { record_gather!("schedstat", schedstat::gather(proc_path)) });
        }

        if collectors.selinux {
            let proc_path = proc_path.clone();
            let sys_path = sys_path.clone();
            tasks.spawn(
                async move { record_gather!("selinux", selinux::gather(proc_path, sys_path)) },
            );
        }

        if collectors.sockstat {
            let proc_path = proc_path.clone();
            tasks.spawn(async move { record_gather!("sockstat", sockstat::gather(proc_path)) });
        }

        if collectors.softnet {
            let proc_path = proc_path.clone();
            tasks.spawn(async move { record_gather!("softnet", softnet::gather(proc_path)) });
        }

        if collectors.softirqs {
            let proc_path = proc_path.clone();
            tasks.spawn(async move { record_gather!("softirqs", softirqs::gather(proc_path)) });
        }

        if collectors.stat {
            let proc_path = proc_path.clone();
            tasks.spawn(async move { record_gather!("stat", stat::gather(proc_path)) });
        }

        if collectors.tcpstat {
            tasks.spawn(async { record_gather!("tcpstat", tcpstat::gather()) });
        }

        if collectors.thermal_zone {
            let sys_path = sys_path.clone();
            tasks.spawn(
                async move { record_gather!("thermal_zone", thermal_zone::gather(sys_path)) },
            );
        }

        if collectors.time {
            let sys_path = sys_path.clone();

            tasks.spawn(async { record_gather!("time", time::gather(sys_path)) });
        }

        if collectors.timex {
            tasks.spawn(async { record_gather!("timex", timex::gather()) });
        }

        if collectors.udp_queues {
            let proc_path = proc_path.clone();
            tasks.spawn(async move { record_gather!("udp_queues", udp_queues::gather(proc_path)) });
        }

        if collectors.uname {
            tasks.spawn(async { record_gather!("uname", uname::gather()) });
        }

        if let Some(conf) = &collectors.vmstat {
            let proc_path = proc_path.clone();
            let conf = conf.clone();
            tasks.spawn(async move { record_gather!("vmstat", vmstat::gather(conf, proc_path)) });
        }

        if collectors.watchdog {
            let sys_path = sys_path.clone();

            tasks.spawn(async move { record_gather!("watchdog", watchdog::gather(sys_path)) });
        }

        #[cfg(target_os = "linux")]
        if collectors.xfs {
            let sys_path = sys_path.clone();
            tasks.spawn(async move { record_gather!("xfs", xfs::gather(sys_path)) });
        }

        if collectors.zfs {
            let proc_path = proc_path.clone();
            tasks.spawn(async move { record_gather!("zfs", zfs::gather(proc_path)) });
        }

        // MacOS
        #[cfg(target_os = "macos")]
        if collectors.boot_time {
            tasks.spawn(async { record_gather!("boot_time", boot_time::gather()) });
        }

        while let Some(Ok(mut metrics)) = tasks.join_next().await {
            let now = chrono::Utc::now();
            metrics
                .iter_mut()
                .for_each(|metric| metric.timestamp = Some(now));

            if let Err(err) = out.send(metrics).await {
                error!(
                    message = "Error sending node metrics",
                    %err,
                );

                return Err(());
            }
        }
    }

    Ok(())
}

#[async_trait::async_trait]
#[typetag::serde(name = "node_metrics")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        Ok(Box::pin(run(
            self.interval,
            self.proc_path.clone(),
            self.sys_path.clone(),
            self.collectors.clone(),
            cx.shutdown,
            cx.output,
        )))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::metrics()]
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
