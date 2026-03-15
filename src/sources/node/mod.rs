#![allow(dead_code)]

mod arp;
mod bcache;
mod bcachefs;
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
mod fibrechannel;
mod filefd;
mod filesystem;
pub mod hwmon;
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
mod nfs;
mod nfsd;
mod nvme;
mod os;
mod pcidevice;
mod powersupplyclass;
mod pressure;
mod processes;
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
use std::num::{ParseFloatError, ParseIntError};
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use configurable::{Configurable, configurable_component};
use event::{Metric, tags};
use framework::Source;
use framework::config::{OutputType, SourceConfig, SourceContext, default_interval, default_true};
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::task::JoinSet;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error(transparent)]
    Integer(ParseIntError),
    #[error(transparent)]
    Float(ParseFloatError),
    #[error("{0}")]
    Other(String),
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Glob(#[from] glob::PatternError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Parse(ParseError),
    #[error("{0}")]
    Other(String),

    #[error("no data")]
    NoData,
    #[error("malformed {0}")]
    Malformed(&'static str),
}

impl From<ParseIntError> for Error {
    fn from(value: ParseIntError) -> Self {
        Error::Parse(ParseError::Integer(value))
    }
}

impl From<ParseFloatError> for Error {
    fn from(value: ParseFloatError) -> Self {
        Error::Parse(ParseError::Float(value))
    }
}

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
    bcachefs: bool,

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
    os: bool,

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
            bcachefs: true,
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
            os: true,
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

fn default_root_path() -> PathBuf {
    PathBuf::from("/")
}

fn default_proc_path() -> PathBuf {
    PathBuf::from("/proc")
}

fn default_sys_path() -> PathBuf {
    PathBuf::from("/sys")
}

fn default_udev_path() -> PathBuf {
    PathBuf::from("/run/udev")
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

macro_rules! collect {
    ($collector: ident $(, $arg:expr)*) => {
        async move {
            let start = std::time::Instant::now();
            let result = $collector::collect( $( $arg, )* ).await;
            let elapsed = start.elapsed().as_secs_f64();

            let (mut metrics, success) = match result {
                Ok(ms) => (ms, 1.0),
                Err(err) => {
                    debug!(
                        message = "collect metrics failed",
                        collector = stringify!( $collector ),
                        %elapsed,
                        %err,
                    );
                    (Vec::with_capacity(2), 0.0)
                },
            };

            metrics.extend([
                Metric::gauge_with_tags(
                    "node_scrape_collector_duration_seconds",
                    "Duration of a collector scrape",
                    elapsed,
                    tags! ("collector" => stringify!($collector))
                ),
                Metric::gauge_with_tags(
                    "node_scrape_collector_success",
                    "Whether a collector succeeded",
                    success,
                    tags! ("collector" => stringify!($collector))
                )
            ]);

            metrics
        }
    };
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
    pub fn new(root: PathBuf, proc: PathBuf, sys: PathBuf, udev: PathBuf) -> Self {
        Paths(Arc::new(PathsInner {
            root,
            proc,
            sys,
            udev,
        }))
    }

    #[cfg(test)]
    fn test() -> Self {
        Paths::new(
            PathBuf::from("tests/node/fixtures/"),
            PathBuf::from("tests/node/fixtures/proc"),
            PathBuf::from("tests/node/fixtures/sys"),
            PathBuf::from("tests/node/fixtures/udev"),
        )
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
            tasks.spawn(collect!(arp, paths));
        }

        if let Some(conf) = &collectors.bcache {
            let conf = conf.clone();
            let paths = paths.clone();
            tasks.spawn(collect!(bcache, conf, paths));
        }

        if collectors.bcachefs {
            let paths = paths.clone();
            tasks.spawn(collect!(bcachefs, paths));
        }

        if collectors.bonding {
            let paths = paths.clone();
            tasks.spawn(collect!(bonding, paths));
        }

        #[cfg(target_os = "macos")]
        if collectors.boot_time {
            tasks.spawn(collect!(boot_time));
        }

        if collectors.btrfs {
            let paths = paths.clone();
            tasks.spawn(collect!(btrfs, paths));
        }

        if collectors.buddyinfo {
            let paths = paths.clone();
            tasks.spawn(collect!(buddyinfo, paths));
        }

        if collectors.cgroups {
            let paths = paths.clone();
            tasks.spawn(collect!(cgroups, paths));
        }

        if collectors.conntrack {
            let paths = paths.clone();
            tasks.spawn(collect!(conntrack, paths));
        }

        if let Some(conf) = &collectors.cpu {
            let conf = conf.clone();
            let paths = paths.clone();
            tasks.spawn(collect!(cpu, conf, paths));
        }

        if collectors.cpufreq {
            let paths = paths.clone();
            tasks.spawn(collect!(cpufreq, paths));
        }

        if let Some(conf) = &collectors.diskstats {
            let conf = conf.clone();
            let paths = paths.clone();
            tasks.spawn(collect!(diskstats, conf, paths));
        }

        if collectors.dmi {
            let paths = paths.clone();
            tasks.spawn(collect!(dmi, paths));
        }

        if collectors.drm {
            let paths = paths.clone();
            tasks.spawn(collect!(drm, paths));
        }

        if collectors.edac {
            let paths = paths.clone();
            tasks.spawn(collect!(edac, paths));
        }

        if collectors.entropy {
            let paths = paths.clone();
            tasks.spawn(collect!(entropy, paths));
        }

        if collectors.fibrechannel {
            let paths = paths.clone();
            tasks.spawn(collect!(fibrechannel, paths));
        }

        if collectors.filefd {
            let paths = paths.clone();
            tasks.spawn(collect!(filefd, paths));
        }

        if let Some(conf) = &collectors.filesystem {
            let conf = conf.clone();
            let paths = paths.clone();
            tasks.spawn(collect!(filesystem, conf, paths));
        }

        if collectors.hwmon {
            let paths = paths.clone();
            tasks.spawn(collect!(hwmon, paths));
        }

        if collectors.infiniband {
            let paths = paths.clone();
            tasks.spawn(collect!(infiniband, paths));
        }

        if collectors.interrupts {
            let paths = paths.clone();
            tasks.spawn(collect!(interrupts, paths));
        }

        if let Some(conf) = &collectors.ipvs {
            let conf = conf.clone();
            let paths = paths.clone();
            tasks.spawn(collect!(ipvs, conf, paths));
        }

        if collectors.kernel_hung {
            let paths = paths.clone();
            tasks.spawn(collect!(kernel_hung, paths));
        }

        if collectors.ksmd {
            let paths = paths.clone();
            tasks.spawn(collect!(ksmd, paths));
        }

        if collectors.lnstat {
            let paths = paths.clone();
            tasks.spawn(collect!(lnstat, paths));
        }

        if collectors.loadavg {
            let paths = paths.clone();
            tasks.spawn(collect!(loadavg, paths));
        }

        if collectors.mdadm {
            let paths = paths.clone();
            tasks.spawn(collect!(mdadm, paths));
        }

        if collectors.memory {
            let paths = paths.clone();
            tasks.spawn(collect!(meminfo, paths));
        }

        if collectors.mountstats {
            let paths = paths.clone();
            tasks.spawn(collect!(mountstats, paths));
        }

        if let Some(conf) = &collectors.netclass {
            let conf = conf.clone();
            let paths = paths.clone();
            tasks.spawn(collect!(netclass, conf, paths));
        }

        if let Some(conf) = &collectors.netdev {
            let conf = conf.clone();
            let paths = paths.clone();
            tasks.spawn(collect!(netdev, conf, paths));
        }

        if let Some(conf) = &collectors.netstat {
            let conf = conf.clone();
            let paths = paths.clone();
            tasks.spawn(collect!(netstat, conf, paths));
        }

        if collectors.nfs {
            let paths = paths.clone();
            tasks.spawn(collect!(nfs, paths));
        }

        if collectors.nfsd {
            let paths = paths.clone();
            tasks.spawn(collect!(nfsd, paths));
        }

        if collectors.nvme {
            let paths = paths.clone();
            tasks.spawn(collect!(nvme, paths));
        }

        if collectors.os {
            let paths = paths.clone();
            tasks.spawn(collect!(os, paths));
        }

        #[cfg(target_os = "linux")]
        if collectors.pcidevice {
            let paths = paths.clone();
            tasks.spawn(collect!(pcidevice, paths));
        }

        if let Some(conf) = &collectors.power_supply {
            let conf = conf.clone();
            let paths = paths.clone();
            tasks.spawn(collect!(powersupplyclass, conf, paths));
        }

        if collectors.pressure {
            let paths = paths.clone();
            tasks.spawn(collect!(pressure, paths));
        }

        if collectors.processes {
            let paths = paths.clone();
            tasks.spawn(collect!(processes, paths));
        }

        if collectors.rapl {
            let paths = paths.clone();
            tasks.spawn(collect!(rapl, paths));
        }

        if collectors.schedstat {
            let paths = paths.clone();
            tasks.spawn(collect!(schedstat, paths));
        }

        if collectors.selinux {
            let paths = paths.clone();
            tasks.spawn(collect!(selinux, paths));
        }

        if let Some(conf) = &collectors.slabinfo {
            let conf = conf.clone();
            let paths = paths.clone();
            tasks.spawn(collect!(slabinfo, conf, paths));
        }

        if collectors.sockstat {
            let paths = paths.clone();
            tasks.spawn(collect!(sockstat, paths));
        }

        if collectors.softnet {
            let paths = paths.clone();
            tasks.spawn(collect!(softnet, paths));
        }

        if collectors.softirqs {
            let paths = paths.clone();
            tasks.spawn(collect!(softirqs, paths));
        }

        if collectors.stat {
            let paths = paths.clone();
            tasks.spawn(collect!(stat, paths));
        }

        if collectors.swap {
            let paths = paths.clone();
            tasks.spawn(collect!(swap, paths));
        }

        if collectors.tapestats {
            let paths = paths.clone();
            tasks.spawn(collect!(tapestats, paths));
        }

        if collectors.tcpstat {
            tasks.spawn(collect!(tcpstat));
        }

        if collectors.thermal_zone {
            let paths = paths.clone();
            tasks.spawn(collect!(thermal_zone, paths));
        }

        if collectors.time {
            let paths = paths.clone();
            tasks.spawn(collect!(time, paths));
        }

        if collectors.timex {
            tasks.spawn(collect!(timex));
        }

        if collectors.udp_queues {
            let paths = paths.clone();
            tasks.spawn(collect!(udp_queues, paths));
        }

        if collectors.uname {
            tasks.spawn(collect!(uname));
        }

        if let Some(conf) = &collectors.vmstat {
            let conf = conf.clone();
            let paths = paths.clone();
            tasks.spawn(collect!(vmstat, conf, paths));
        }

        if collectors.watchdog {
            let paths = paths.clone();
            tasks.spawn(collect!(watchdog, paths));
        }

        if collectors.xfrm {
            let paths = paths.clone();
            tasks.spawn(collect!(xfrm, paths));
        }

        #[cfg(target_os = "linux")]
        if collectors.xfs {
            let paths = paths.clone();
            tasks.spawn(collect!(xfs, paths));
        }

        if collectors.zfs {
            let paths = paths.clone();
            tasks.spawn(collect!(zfs, paths));
        }

        if collectors.zoneinfo {
            let paths = paths.clone();
            tasks.spawn(collect!(zoneinfo, paths));
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
#[typetag::serde(name = "node")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        Ok(Box::pin(run(
            Paths::new(
                self.root_path.clone(),
                self.proc_path.clone(),
                self.sys_path.clone(),
                self.udev_path.clone(),
            ),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
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
