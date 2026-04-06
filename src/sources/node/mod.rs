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
mod meminfo_numa;
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
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Glob(#[from] glob::PatternError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Parse(ParseError),

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
    /// Exposes ARP statistics from /proc/net/arp.
    #[serde(default = "default_true")]
    arp: bool,

    /// Exposes bcache statistics from /sys/fs/bcache/.
    #[serde(default = "default_bcache_config")]
    bcache: Option<bcache::Config>,

    #[serde(default = "default_true")]
    bcachefs: bool,

    /// Exposes the number of configured and active slaves of Linux bonding interfaces.
    #[serde(default = "default_true")]
    bonding: bool,

    /// Exposes system boot time derived from the kern.boottime sysctl.
    #[cfg(target_os = "macos")]
    #[serde(default = "default_true")]
    boot_time: bool,

    /// Exposes btrfs statistics
    #[serde(default = "default_true")]
    btrfs: bool,

    /// Exposes statistics of memory fragments as reported by /proc/buddyinfo.
    #[serde(default)]
    buddyinfo: bool,

    /// A summary of the number of active and enabled cgroups
    #[serde(default)]
    cgroups: bool,

    /// Shows conntrack statistics (does nothing if no /proc/sys/net/netfilter/ present).
    #[serde(default = "default_true")]
    conntrack: bool,

    /// Exposes CPU statistics
    #[serde(default = "default_cpu_config")]
    cpu: Option<cpu::Config>,

    /// Exposes CPU frequency statistics
    #[serde(default = "default_true")]
    cpufreq: bool,

    /// Exposes disk I/O statistics.
    #[serde(default = "default_diskstats_config")]
    diskstats: Option<diskstats::Config>,

    /// Expose Desktop Management Interface (DMI) info from /sys/class/dmi/id/
    #[serde(default = "default_true")]
    dmi: bool,

    /// Expose GPU metrics using sysfs / DRM, amdgpu is the only driver which exposes this information through DRM
    #[serde(default)]
    drm: bool,

    /// Exposes error detection and correction statistics.
    #[serde(default = "default_true")]
    edac: bool,

    /// Exposes available entropy.
    #[serde(default = "default_true")]
    entropy: bool,

    /// Exposes fibre channel information and statistics from /sys/class/fc_host/.
    #[serde(default = "default_true")]
    fibrechannel: bool,

    /// Exposes file descriptor statistics from /proc/sys/fs/file-nr.
    #[serde(default = "default_true")]
    filefd: bool,

    /// Exposes filesystem statistics, such as disk space used.
    #[serde(default = "default_filesystem_config")]
    filesystem: Option<filesystem::Config>,

    /// Expose hardware monitoring and sensor data from /sys/class/hwmon/.
    #[serde(default = "default_true")]
    hwmon: bool,

    /// Exposes network statistics specific to InfiniBand and Intel OmniPath configurations.
    #[serde(default = "default_true")]
    infiniband: bool,

    /// Exposes detailed interrupts statistics.
    #[serde(default)]
    interrupts: bool,

    /// Exposes IPVS status from /proc/net/ip_vs and stats from /proc/net/ip_vs_stats.
    #[serde(default = "default_ipvs_config")]
    ipvs: Option<ipvs::Config>,

    /// Exposes number of tasks that have been detected as hung from `/proc/sys/kernel/hung_task_detect_count`.
    #[serde(default = "default_true")]
    kernel_hung: bool,

    /// Exposes kernel and system statistics from /sys/kernel/mm/ksm.
    #[serde(default)]
    ksmd: bool,

    /// Exposes stats from /proc/net/stat/
    #[serde(default)]
    lnstat: bool,

    /// Exposes load average.
    #[serde(default = "default_true")]
    loadavg: bool,

    /// Exposes statistics about devices in /proc/mdstat (does nothing if no /proc/mdstat present).
    #[serde(default = "default_true")]
    mdadm: bool,

    /// Exposes memory statistics.
    #[serde(default = "default_true")]
    meminfo: bool,

    /// Exposes memory statistics from /sys/devices/system/node/node[0-9]*/meminfo, /sys/devices/system/node/node[0-9]*/numastat.
    #[serde(default)]
    meminfo_numa: bool,

    /// Exposes filesystem statistics from /proc/self/mountstats. Exposes detailed NFS client statistics.
    #[serde(default)]
    mountstats: bool,

    /// Exposes network interface info from /sys/class/net/
    #[serde(default = "default_netclass_config")]
    netclass: Option<netclass::Config>,

    /// Exposes network interface statistics such as bytes transferred.
    #[serde(
        default = "default_netdev_config",
        with = "serde_yaml::with::singleton_map"
    )]
    netdev: Option<netdev::Config>,

    /// Exposes network statistics from /proc/net/netstat. This is the same information as netstat -s.
    #[serde(default = "default_netstat_config")]
    netstat: Option<netstat::Config>,

    /// Exposes NFS client statistics from /proc/net/rpc/nfs. This is the same information as nfsstat -c.
    #[serde(default = "default_true")]
    nfs: bool,

    /// Exposes NFS kernel server statistics from /proc/net/rpc/nfsd. This is the same information as nfsstat -s.
    #[serde(default = "default_true")]
    nfsd: bool,

    /// Exposes NVMe info from /sys/class/nvme/
    #[serde(default = "default_true")]
    nvme: bool,

    /// Expose OS release info from /etc/os-release or /usr/lib/os-release
    #[serde(default = "default_true")]
    os: bool,

    /// Exposes pci devices' information including their link status and parent devices.
    #[serde(default)]
    pcidevice: bool,

    /// Exposes Power Supply statistics from `/sys/class/power_supply`
    #[serde(default = "default_powersupply_config")]
    power_supply_class: Option<powersupplyclass::Config>,

    /// Exposes pressure stall statistics from `/proc/pressure/`
    #[serde(default = "default_true")]
    pressure: bool,

    /// Exposes aggregate process statistics from `/proc`.
    #[serde(default)]
    processes: bool,

    /// Exposes various statistics from `/sys/class/powercap`.
    #[serde(default = "default_true")]
    rapl: bool,

    /// Exposes task scheduler statistics from `/proc/schedstat`.
    #[serde(default = "default_true")]
    schedstat: bool,

    /// Exposes SELinux statistics.
    #[serde(default = "default_true")]
    selinux: bool,

    /// Exposes slab statistics from `/proc/slabinfo`. Note that permission of `/proc/slabinfo` is usually 0400, so set it appropriately.
    #[serde(default)]
    slabinfo: Option<slabinfo::Config>,

    /// Exposes various statistics from `/proc/net/sockstat`.
    #[serde(default = "default_true")]
    sockstat: bool,

    /// Exposes statistics from `/proc/net/softnet_stat`.
    #[serde(default = "default_true")]
    softnet: bool,

    /// Exposes detailed softirq statistics from `/proc/softirqs`.
    #[serde(default)]
    softirqs: bool,

    /// Exposes various statistics from `/proc/stat`. This includes boot time, forks and interrupts.
    #[serde(default = "default_true")]
    stat: bool,

    /// Expose swap information from `/proc/swaps`.
    #[serde(default)]
    swap: bool,

    /// Exposes statistics from `/sys/class/scsi_tape`.
    #[serde(default = "default_true")]
    tapestats: bool,

    /// Exposes TCP connection status information from `/proc/net/tcp` and `/proc/net/tcp6`. (Warning: the current version has potential performance issues in high load situations.)
    #[serde(default = "default_true")]
    tcpstat: bool,

    /// Exposes thermal zone & cooling device statistics from `/sys/class/thermal`.
    #[serde(default = "default_true")]
    thermal_zone: bool,

    /// Exposes the current system time.
    #[serde(default = "default_true")]
    time: bool,

    /// Exposes selected adjtimex(2) system call stats.
    #[serde(default = "default_true")]
    timex: bool,

    /// Exposes UDP total lengths of the rx_queue and tx_queue from `/proc/net/udp` and `/proc/net/udp6`.
    #[serde(default = "default_true")]
    udp_queues: bool,

    /// Exposes system information as provided by the uname system call.
    #[serde(default = "default_true")]
    uname: bool,

    /// Exposes statistics from `/proc/vmstat`.
    #[serde(default = "default_vmstat_config")]
    vmstat: Option<vmstat::Config>,

    /// Exposes statistics from `/sys/class/watchdog`
    #[serde(default = "default_true")]
    watchdog: bool,

    /// Exposes statistics from `/proc/net/xfrm_stat`
    #[serde(default)]
    xfrm: bool,

    /// Exposes XFS runtime statistics.
    #[cfg(target_os = "linux")]
    #[serde(default = "default_true")]
    xfs: bool,

    /// Exposes ZFS performance statistics.
    #[serde(default = "default_true")]
    zfs: bool,

    /// Exposes NUMA memory zone metrics.
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
            kernel_hung: true,
            ksmd: false,
            lnstat: false,
            loadavg: true,
            mdadm: true,
            meminfo: true,
            meminfo_numa: false,
            mountstats: false,
            netclass: default_netclass_config(),
            netdev: default_netdev_config(),
            netstat: default_netstat_config(),
            nfs: true,
            nfsd: true,
            nvme: true,
            os: true,
            pcidevice: false,
            power_supply_class: default_powersupply_config(),
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

/// Read contents of entire file. This is similar to [`std::fs::read_to_string`]
/// but without the call to [`stat()`], because many files in /proc or /sys
/// report incorrect file sizes (either 0 or 4096).
fn read_file_no_stat<P: AsRef<Path>>(path: P) -> std::io::Result<String> {
    let file = std::fs::File::open(path)?;
    std::io::read_to_string(file)
}

pub fn read_into<P, T, E>(path: P) -> Result<T, Error>
where
    P: AsRef<Path>,
    T: FromStr<Err = E>,
    Error: From<E>,
{
    let mut file = std::fs::File::open(path)?;

    // i64::MAX_STR_LEN is 20 and f64::MAX_STR_LEN is 24
    let mut buf = [0u8; 24];
    let size = file.read(&mut buf)?;

    String::from_utf8_lossy(&buf[..size])
        .trim()
        .parse::<T>()
        .map_err(Into::into)
}

/// A simplified std::fs::read_to_string
fn read_sys_file<P: AsRef<Path>>(path: P) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;

    let mut buf = String::with_capacity(128);
    Read::read_to_string(&mut file, &mut buf)?;

    let len = buf.trim_end().len();
    buf.truncate(len);

    Ok(buf)
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

        if collectors.meminfo {
            let paths = paths.clone();
            tasks.spawn(collect!(meminfo, paths));
        }

        if collectors.meminfo_numa {
            let paths = paths.clone();
            tasks.spawn(collect!(meminfo_numa, paths));
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

        if let Some(conf) = &collectors.power_supply_class {
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
            let paths = paths.clone();
            tasks.spawn(collect!(tcpstat, paths));
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
}
