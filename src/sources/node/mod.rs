#![allow(dead_code)]

mod arp;
mod bcache;
mod bonding;
#[cfg(target_os = "macos")]
mod boot_time;
mod btrfs;
mod buddyinfo;
mod cgroups;
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
mod hwmon;
mod infiniband;
mod interrupts;
mod ipvs;
#[cfg(target_os = "linux")]
mod kernel_hung;
mod ksmd;
mod lnstat;
mod loadavg;
mod mdadm;
mod meminfo;
mod mountstats;
mod netclass;
mod netdev;
mod netstat;
mod network_route;
mod nfs;
mod nfsd;
mod nvme;
mod os_release;
mod pcidevice;
mod powersupplyclass;
mod pressure;
mod processes;
mod protocols;
mod rapl;
mod schedstat;
mod selinux;
mod slabinfo;
mod sockstat;
mod softirqs;
mod softnet;
mod stat;
#[cfg(target_os = "linux")]
mod swap;
mod tapestats;
mod tcpstat;
mod thermal_zone;
mod time;
mod timex;
mod udp_queues;
mod uname;
mod vmstat;
mod watchdog;
mod xfrm;
#[cfg(target_os = "linux")]
mod xfs;
mod zfs;
mod zoneinfo;

use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use configurable::{Configurable, configurable_component};
use error::Error;
use event::{Metric, tags};
use framework::Source;
use framework::config::{OutputType, SourceConfig, SourceContext, default_interval, default_true};
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
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

#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Collectors {
    #[serde(default = "default_true")]
    arp: bool,

    #[serde(default = "default_bcache_config")]
    bcache: Option<bcache::Config>,

    #[serde(default = "default_true")]
    bonding: bool,

    // MacOS
    #[cfg(target_os = "macos")]
    #[serde(default = "default_true")]
    boot_time: bool,

    #[serde(default = "default_true")]
    btrfs: bool,

    #[serde(default)]
    buddyinfo: bool,

    #[serde(default)]
    cgroups: bool,

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

    #[serde(default)]
    interrupts: bool,

    #[serde(default = "default_ipvs_config")]
    ipvs: Option<ipvs::Config>,

    #[serde(default)]
    kernel_hung: bool,

    #[serde(default)]
    ksmd: bool,

    /// Exposes stats from /proc/net/stat/
    #[serde(default)]
    lnstat: bool,

    #[serde(default = "default_true")]
    loadavg: bool,

    #[serde(default = "default_true")]
    mdadm: bool,

    #[serde(default = "default_true")]
    memory: bool,

    #[serde(default)]
    mountstats: bool,

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

    #[serde(default)]
    pcidevice: bool,

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

    #[serde(default)]
    slabinfo: Option<slabinfo::Config>,

    #[serde(default = "default_true")]
    sockstat: bool,

    #[serde(default = "default_true")]
    softnet: bool,

    #[serde(default)]
    softirqs: bool,

    #[serde(default = "default_true")]
    stat: bool,

    #[serde(default)]
    swap: bool,

    #[serde(default = "default_true")]
    tapestats: bool,

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

    #[serde(default)]
    xfrm: bool,

    #[cfg(target_os = "linux")]
    #[serde(default = "default_true")]
    xfs: bool,

    #[serde(default = "default_true")]
    zfs: bool,

    #[cfg(target_os = "linux")]
    #[serde(default)]
    zoneinfo: bool,
}

impl Default for Collectors {
    fn default() -> Self {
        Self {
            arp: true,
            bcache: default_bcache_config(),
            bonding: true,
            #[cfg(target_os = "macos")]
            boot_time: true,
            btrfs: true,
            buddyinfo: false,
            cgroups: false,
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
            interrupts: false,
            ipvs: default_ipvs_config(),
            kernel_hung: false,
            ksmd: false,
            lnstat: false,
            loadavg: true,
            mdadm: true,
            memory: true,
            mountstats: false,
            netclass: default_netclass_config(),
            netdev: default_netdev_config(),
            netstat: default_netstat_config(),
            nfs: true,
            nfsd: true,
            nvme: true,
            os_release: true,
            pcidevice: false,
            power_supply: default_powersupply_config(),
            pressure: true,
            processes: false,
            rapl: true,
            schedstat: true,
            selinux: true,
            slabinfo: None,
            sockstat: true,
            softnet: true,
            softirqs: false,
            stat: true,
            swap: false,
            tapestats: true,
            time: true,
            timex: true,
            tcpstat: true,
            thermal_zone: true,
            udp_queues: true,
            uname: true,
            vmstat: default_vmstat_config(),
            watchdog: true,
            xfrm: false,
            xfs: true,
            zfs: true,
            zoneinfo: false,
        }
    }
}

fn default_proc_path() -> PathBuf {
    "/proc".into()
}

fn default_sys_path() -> PathBuf {
    "/sys".into()
}

fn default_udev_path() -> PathBuf {
    PathBuf::from("/run/udev")
}

fn default_root_path() -> PathBuf {
    PathBuf::from("/")
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

    #[serde(default = "default_root_path")]
    root_path: PathBuf,

    /// sysfs mountpoint.
    #[serde(default = "default_sys_path")]
    sys_path: PathBuf,

    #[serde(default = "default_udev_path")]
    udev_path: PathBuf,

    /// Duration between each scrape.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    #[serde(default)]
    collectors: Collectors,
}

/// The files this function will(should) be reading is under `/sys` and `/proc` which is
/// very small, so the performance should never be a problem.
pub fn read_string<P: AsRef<Path>>(path: P) -> std::io::Result<String> {
    let mut data = read_file(path)?;

    let trimmed = data.trim_ascii_end();
    unsafe { data.set_len(trimmed.len()) }

    String::from_utf8(data).map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))
}

/// `read_file` read contents of entire file. This is similar to `std::fs::read`
/// but without the call to libc::stat, because many files in /proc and /sys report
/// incorrect file sizes (either 0 or 4096). Reads a max file size of 1024kB. For
/// files larger than this, a reader should be used.
#[inline]
fn read_file<P: AsRef<Path>>(path: P) -> std::io::Result<Vec<u8>> {
    const MAX_BUF_SIZE: usize = 1024 * 1024;
    const STEP: usize = 32;

    let mut file = std::fs::File::open(&path)?;
    let mut buf = Vec::with_capacity(STEP);
    let mut total_read = 0;

    loop {
        if total_read + STEP > MAX_BUF_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::FileTooLarge,
                "file exceeds maximum allowed size",
            ));
        }

        // Ensure there's at least one byte available for reading
        buf.resize(total_read + STEP, 0);

        let cnt = file.read(&mut buf[total_read..total_read + STEP])?;
        total_read += cnt;

        if cnt == 0 {
            break;
        }
    }

    buf.truncate(total_read);

    Ok(buf)
}

pub fn read_into<P, T, E>(path: P) -> Result<T, Error>
where
    P: AsRef<Path>,
    T: FromStr<Err = E>,
    Error: From<E>,
{
    let content = read_string(path)?;
    Ok(<T as FromStr>::from_str(content.as_str())?)
}

#[async_trait::async_trait]
#[typetag::serde(name = "node")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let paths = Paths::new(
            self.root_path.clone(),
            self.proc_path.clone(),
            self.sys_path.clone(),
            self.udev_path.clone(),
        );

        Ok(Box::pin(run(
            paths,
            self.collectors.clone(),
            self.interval,
            cx.shutdown,
            cx.output,
        )))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::metric()]
    }
}

struct PathsInner {
    root: PathBuf,
    proc: PathBuf,
    sys: PathBuf,
    udev: PathBuf,
}

#[derive(Clone)]
pub struct Paths(Arc<PathsInner>);

impl Paths {
    fn new(root: PathBuf, proc: PathBuf, sys: PathBuf, udev: PathBuf) -> Self {
        Paths(Arc::new(PathsInner {
            root,
            proc,
            sys,
            udev,
        }))
    }

    #[inline]
    fn root(&self) -> &Path {
        self.0.root.as_path()
    }

    #[inline]
    fn proc(&self) -> &Path {
        self.0.proc.as_path()
    }

    #[inline]
    fn sys(&self) -> &Path {
        self.0.sys.as_path()
    }

    #[inline]
    fn udev(&self) -> &Path {
        self.0.udev.as_path()
    }
}

macro_rules! record_gather {
    ($name: expr, $future: expr) => ({
        let start = std::time::Instant::now();
        let result = $future.await;
        let elapsed = start.elapsed().as_secs_f64();

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
                "Duration of a collector scrape",
                elapsed,
                tags! ("collector" => $name)
            ),
            Metric::gauge_with_tags(
                "node_scrape_collector_success",
                "Whether a collector succeeded",
                success,
                tags! ("collector" => $name)
            )
        ]);

        metrics
    })
}

async fn run(
    paths: Paths,
    collectors: Collectors,
    interval: Duration,
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
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("arp", arp::collect(paths)) });
        }

        if let Some(conf) = &collectors.bcache {
            let conf = conf.clone();
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("bcache", bcache::collect(conf, paths)) });
        }

        if collectors.bonding {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("bonding", bonding::collect(paths)) });
        }

        #[cfg(target_os = "macos")]
        if collectors.boot_time {
            tasks.spawn(async { record_gather!("boot_time", boot_time::collect()) });
        }

        if collectors.btrfs {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("btrfs", btrfs::collect(paths)) });
        }

        if collectors.buddyinfo {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("buddyinfo", buddyinfo::collect(paths)) });
        }

        if collectors.cgroups {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("cgroups", cgroups::collect(paths)) });
        }

        if collectors.conntrack {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("conntrack", conntrack::collect(paths)) });
        }

        if let Some(conf) = &collectors.cpu {
            let conf = conf.clone();
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("cpu", cpu::collect(conf, paths)) });
        }

        if collectors.cpufreq {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("cpufreq", cpufreq::collect(paths)) });
        }

        if let Some(conf) = &collectors.diskstats {
            let conf = conf.clone();
            let paths = paths.clone();
            tasks
                .spawn(async move { record_gather!("diskstats", diskstats::collect(conf, paths)) });
        }

        if collectors.dmi {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("dmi", dmi::collect(paths)) });
        }

        if collectors.drm {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("drm", drm::collect(paths)) });
        }

        if collectors.edac {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("edac", edac::collect(paths)) });
        }

        if collectors.entropy {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("entropy", entropy::collect(paths)) });
        }

        if collectors.fibrechannel {
            let paths = paths.clone();
            tasks
                .spawn(async move { record_gather!("fibrechannel", fibrechannel::collect(paths)) });
        }

        if collectors.filefd {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("filefd", filefd::collect(paths)) });
        }

        if let Some(conf) = &collectors.filesystem {
            let conf = conf.clone();
            let paths = paths.clone();
            tasks.spawn(
                async move { record_gather!("filesystem", filesystem::collect(conf, paths)) },
            );
        }

        if collectors.hwmon {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("hwmon", hwmon::collect(paths)) });
        }

        if collectors.infiniband {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("infiniband", infiniband::collect(paths)) });
        }

        if collectors.interrupts {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("interrupts", interrupts::collect(paths)) });
        }

        if let Some(conf) = &collectors.ipvs {
            let conf = conf.clone();
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("ipvs", ipvs::collect(conf, paths)) });
        }

        if collectors.kernel_hung {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("kernel_hung", kernel_hung::collect(paths)) });
        }

        if collectors.ksmd {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("ksmd", ksmd::collect(paths)) });
        }

        if collectors.lnstat {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("lnstat", lnstat::collect(paths)) });
        }

        if collectors.loadavg {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("loadavg", loadavg::collect(paths)) });
        }

        if collectors.mdadm {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("mdadm", mdadm::collect(paths)) });
        }

        if collectors.memory {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("meminfo", meminfo::collect(paths)) });
        }

        if collectors.mountstats {
            let proc_path = proc_path.clone();
            tasks
                .spawn(async move { record_gather!("mountstats", mountstats::collect(proc_path)) });
        }

        if let Some(conf) = &collectors.netclass {
            let conf = conf.clone();
            let paths = paths.clone();

            tasks.spawn(async move { record_gather!("netclass", netclass::collect(conf, paths)) });
        }

        if let Some(conf) = &collectors.netdev {
            let conf = conf.clone();
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("netdev", netdev::collect(conf, paths)) });
        }

        if let Some(conf) = &collectors.netstat {
            let conf = conf.clone();
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("netstat", netstat::collect(conf, paths)) });
        }

        if collectors.nfs {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("nfs", nfs::collect(paths)) });
        }

        if collectors.nfsd {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("nfsd", nfsd::collect(paths)) });
        }

        if collectors.nvme {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("nvme", nvme::collect(paths)) });
        }

        if collectors.os_release {
            let paths = paths.clone();
            tasks.spawn(async { record_gather!("os", os_release::collect(paths)) });
        }

        #[cfg(target_os = "linux")]
        if collectors.pcidevice {
            let paths = paths.clone();
            tasks.spawn(async { record_gather!("pcidevice", pcidevice::collect(paths)) });
        }

        if let Some(conf) = &collectors.power_supply {
            let conf = conf.clone();
            let paths = paths.clone();
            tasks.spawn(async move {
                record_gather!("powersupplyclass", powersupplyclass::collect(conf, paths))
            });
        }

        if collectors.pressure {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("pressure", pressure::collect(paths)) });
        }

        if collectors.processes {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("processes", processes::collect(paths)) });
        }

        if collectors.rapl {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("rapl", rapl::collect(paths)) });
        }

        if collectors.schedstat {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("schedstat", schedstat::collect(paths)) });
        }

        if collectors.selinux {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("selinux", selinux::collect(paths)) });
        }

        if let Some(config) = &collectors.slabinfo {
            let config = config.clone();
            let paths = paths.clone();
            tasks
                .spawn(async move { record_gather!("slabinfo", slabinfo::collect(config, paths)) });
        }

        if collectors.sockstat {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("sockstat", sockstat::collect(paths)) });
        }

        if collectors.softnet {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("softnet", softnet::collect(paths)) });
        }

        if collectors.softirqs {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("softirqs", softirqs::collect(paths)) });
        }

        if collectors.stat {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("stat", stat::collect(paths)) });
        }

        if collectors.swap {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("swap", swap::collect(paths)) });
        }

        if collectors.tapestats {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("tapestats", tapestats::collect(paths)) });
        }

        if collectors.tcpstat {
            tasks.spawn(async { record_gather!("tcpstat", tcpstat::collect()) });
        }

        if collectors.thermal_zone {
            let paths = paths.clone();
            tasks
                .spawn(async move { record_gather!("thermal_zone", thermal_zone::collect(paths)) });
        }

        if collectors.time {
            let paths = paths.clone();
            tasks.spawn(async { record_gather!("time", time::collect(paths)) });
        }

        if collectors.timex {
            tasks.spawn(async { record_gather!("timex", timex::collect()) });
        }

        if collectors.udp_queues {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("udp_queues", udp_queues::collect(paths)) });
        }

        if collectors.uname {
            tasks.spawn(async { record_gather!("uname", uname::collect()) });
        }

        if let Some(conf) = &collectors.vmstat {
            let paths = paths.clone();
            let conf = conf.clone();
            tasks.spawn(async move { record_gather!("vmstat", vmstat::collect(conf, paths)) });
        }

        if collectors.watchdog {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("watchdog", watchdog::collect(paths)) });
        }

        if collectors.xfrm {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("xfrm", xfrm::collect(paths)) });
        }

        #[cfg(target_os = "linux")]
        if collectors.xfs {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("xfs", xfs::collect(paths)) });
        }

        if collectors.zfs {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("zfs", zfs::collect(paths)) });
        }

        if collectors.zoneinfo {
            let paths = paths.clone();
            tasks.spawn(async move { record_gather!("zoneinfo", zoneinfo::collect(paths)) });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }
}
