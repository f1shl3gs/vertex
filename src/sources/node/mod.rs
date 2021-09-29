mod arp;
mod bonding;
mod btrfs;
mod cpufreq;
mod diskstats;
mod edac;
mod entropy;
mod fibrechannel;
mod filefd;
mod filesystem;
pub mod hwmon;
mod infiniband;
mod ipvs;
mod loadavg;
mod mdadm;
mod meminfo;
mod netclass;
mod netdev;
mod netstat;
mod nfs;
mod nfsd;
mod nvme;
mod powersupplyclass;
mod pressure;
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
mod xfs;
mod zfs;
mod cpu;
mod conntrack;
mod errors;
mod os_release;
mod drm;
mod lnstat;
mod protocols;
mod network_route;
mod wifi;

use typetag;
use serde::{Deserialize, Serialize};
use crate::sources::Source;
use crate::config::{SourceConfig, SourceContext, DataType, deserialize_duration, serialize_duration, default_true};
use tokio_stream::wrappers::IntervalStream;
use futures::{StreamExt, SinkExt};
use crate::shutdown::ShutdownSignal;
use crate::pipeline::Pipeline;
use crate::event::{Event};
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
    path::Path,
};

use cpu::CPUConfig;
use diskstats::DiskStatsConfig;
use crate::sources::node::errors::Error;
use std::str::FromStr;
use crate::sources::node::netdev::NetdevConfig;
use crate::sources::node::vmstat::VMStatConfig;
use crate::sources::node::netclass::NetClassConfig;
use crate::sources::node::netstat::NetstatConfig;
use crate::sources::node::ipvs::IPVSConfig;
use std::io::Read;
use crate::{
    tags,
    event::Metric,
};

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

    pub cpu: Option<Arc<CPUConfig>>,

    #[serde(default = "default_true")]
    pub cpufreq: bool,

    #[serde(default)]
    pub diskstats: Option<Arc<DiskStatsConfig>>,

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

    #[serde(default)]
    pub filesystem: Option<Arc<filesystem::FileSystemConfig>>,

    #[serde(default = "default_true")]
    pub hwmon: bool,

    #[serde(default = "default_true")]
    pub infiniband: bool,

    #[serde(default)]
    pub ipvs: Option<Arc<IPVSConfig>>,

    #[serde(default = "default_true")]
    pub loadavg: bool,

    #[serde(default = "default_true")]
    pub mdadm: bool,

    #[serde(default = "default_true")]
    pub memory: bool,

    #[serde(default)]
    pub netclass: Option<Arc<netclass::NetClassConfig>>,

    #[serde(default)]
    pub netdev: Option<Arc<netdev::NetdevConfig>>,

    #[serde(default)]
    pub netstat: Option<Arc<netstat::NetstatConfig>>,

    #[serde(default = "default_true")]
    pub nfs: bool,

    #[serde(default = "default_true")]
    pub nfsd: bool,

    #[serde(default = "default_true")]
    pub nvme: bool,

    #[serde(default = "default_true")]
    pub os_release: bool,

    #[serde(default)]
    pub power_supply: Option<Arc<powersupplyclass::PowerSupplyConfig>>,

    #[serde(default = "default_true")]
    pub pressure: bool,

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

    pub vmstat: Option<Arc<vmstat::VMStatConfig>>,

    #[serde(default = "default_true")]
    pub xfs: bool,

    #[serde(default = "default_true")]
    pub zfs: bool,
}

impl Default for Collectors {
    fn default() -> Self {
        Self {
            arp: default_true(),
            btrfs: default_true(),
            bonding: default_true(),
            conntrack: default_true(),
            cpu: Some(Arc::new(CPUConfig::default())),
            cpufreq: true,
            diskstats: Some(Arc::new(DiskStatsConfig::default())),
            drm: default_true(),
            edac: default_true(),
            entropy: default_true(),
            fibrechannel: default_true(),
            filefd: default_true(),
            filesystem: Some(Arc::new(filesystem::FileSystemConfig::default())),
            hwmon: default_true(),
            infiniband: default_true(),
            ipvs: Some(Arc::new(ipvs::IPVSConfig::default())),
            loadavg: default_true(),
            mdadm: default_true(),
            memory: default_true(),
            netclass: Some(Arc::new(NetClassConfig::default())),
            netdev: Some(Arc::new(NetdevConfig::default())),
            netstat: Some(Arc::new(NetstatConfig::default())),
            nfs: default_true(),
            nfsd: default_true(),
            nvme: default_true(),
            os_release: default_true(),
            power_supply: Some(Arc::new(powersupplyclass::PowerSupplyConfig::default())),
            pressure: default_true(),
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
            vmstat: Some(Arc::new(VMStatConfig::default())),
            xfs: default_true(),
            zfs: default_true(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NodeMetricsConfig {
    #[serde(default = "default_interval", deserialize_with = "deserialize_duration", serialize_with = "serialize_duration")]
    interval: chrono::Duration,

    #[serde(default = "default_proc_path")]
    proc_path: String,

    #[serde(default = "default_sys_path")]
    sys_path: String,

    #[serde(default = "default_collectors")]
    collectors: Collectors,
}

fn default_interval() -> chrono::Duration {
    chrono::Duration::seconds(15)
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

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct NodeMetrics {
    interval: std::time::Duration,
    proc_path: Arc<String>,
    sys_path: Arc<String>,

    collectors: Collectors,
}

/// `read_to_string` should be a async function, but the implement do sync calls from
/// std, which will not call spawn_blocking and create extra threads for IO reading. It
/// actually reduce cpu usage an memory. The `tokio-uring` should be introduce once it's
/// ready.
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

    Ok(content)
}

pub async fn read_into<P, T, E>(path: P) -> Result<T, Error>
    where
        P: AsRef<Path>,
        T: FromStr<Err=E>,
        Error: From<E>
{
    let content = read_to_string(path).await?;
    Ok(<T as FromStr>::from_str(content.trim())?)
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
        let mut ticker = IntervalStream::new(interval)
            .take_until(shutdown);

        while ticker.next().await.is_some() {
            let mut tasks = Vec::new();

            let start = std::time::SystemTime::now();
            let end = std::time::SystemTime::now();
            let duration = end.duration_since(start).unwrap().as_secs();

            if self.collectors.arp {
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    record_gather!("arp", arp::gather(proc_path.as_ref()))
                }));
            }

            if self.collectors.bonding {
                let sys_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    record_gather!("bonding", bonding::gather(sys_path.as_ref()))
                }));
            }

            if self.collectors.btrfs {
                let sys_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("btrfs", btrfs::gather(sys_path.as_ref()))
                }))
            }

            if self.collectors.conntrack {
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    record_gather!("conntrack", conntrack::gather(proc_path.as_ref()))
                }))
            }

            if let Some(ref conf) = self.collectors.cpu {
                let proc_path = self.proc_path.clone();
                let conf = conf.clone();

                tasks.push(tokio::spawn(async move {
                    record_gather!("cpu", conf.gather(proc_path.as_ref()))
                }));
            }

            if self.collectors.cpufreq {
                let sys_path = self.sys_path.clone();

                tasks.push(tokio::spawn(async move {
                    record_gather!("cpufreq", cpufreq::gather(sys_path.as_ref()))
                }))
            }

            if let Some(ref conf) = self.collectors.diskstats {
                let conf = conf.clone();
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    record_gather!("diskstats", conf.gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.drm {
                let sys_path = self.sys_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("drm", drm::gather(sys_path.as_ref()))
                }))
            }

            if self.collectors.edac {
                let sys_path = self.sys_path.clone();

                tasks.push(tokio::spawn(async move {
                    record_gather!("edac", edac::gather(sys_path.as_ref()))
                }))
            }

            if self.collectors.entropy {
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    record_gather!("entropy", entropy::gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.fibrechannel {
                let sys_path = self.sys_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("fibrechannel", fibrechannel::gather(sys_path.as_ref()))
                }))
            }

            if self.collectors.filefd {
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    record_gather!("filefd", filefd::gather(proc_path.as_ref()))
                }))
            }

            if let Some(ref conf) = self.collectors.filesystem {
                let proc_path = self.proc_path.clone();
                let conf = conf.clone();

                tasks.push(tokio::spawn(async move {
                    record_gather!("filesystem", conf.gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.hwmon {
                let sys_path = self.sys_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("hwmon", hwmon::gather(sys_path.as_ref()))
                }))
            }

            if self.collectors.infiniband {
                let sys_path = self.sys_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("infiniband", infiniband::gather(sys_path.as_ref()))
                }))
            }

            if let Some(ref conf) = self.collectors.ipvs {
                let conf = conf.clone();
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("ipvs", ipvs::gather(conf.as_ref(), proc_path.as_ref()))
                }))
            }

            if self.collectors.loadavg {
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    record_gather!("loadavg", loadavg::gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.mdadm {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("mdadm", mdadm::gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.memory {
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    record_gather!("meminfo", meminfo::gather(proc_path.as_ref()))
                }))
            }

            if let Some(ref conf) = self.collectors.netclass {
                let conf = conf.clone();
                let sys_path = self.sys_path.clone();

                tasks.push(tokio::spawn(async move {
                    record_gather!("netclass", netclass::gather(conf.as_ref(), sys_path.as_ref()))
                }))
            }

            if let Some(ref conf) = self.collectors.netdev {
                let conf = conf.clone();
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    record_gather!("netdev", conf.gather(proc_path.as_ref()))
                }))
            }

            if let Some(ref conf) = self.collectors.netstat {
                let conf = conf.clone();
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    record_gather!("netstat", netstat::gather(conf.as_ref(), proc_path.as_ref()))
                }))
            }

            if self.collectors.nfs {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("nfs", nfs::gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.nfsd {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("nfsd", nfsd::gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.nvme {
                let sys_path = self.sys_path.clone();

                tasks.push(tokio::spawn(async move {
                    record_gather!("nvme", nvme::gather(sys_path.as_ref()))
                }))
            }

            if self.collectors.os_release {
                tasks.push(tokio::spawn(async {
                    record_gather!("os", os_release::gather())
                }))
            }

            if let Some(ref conf) = self.collectors.power_supply {
                let sys_path = self.sys_path.clone();
                let conf = conf.clone();

                tasks.push(tokio::spawn(async move {
                    record_gather!("powersupplyclass", powersupplyclass::gather(sys_path.as_ref(), conf.as_ref()))
                }))
            }

            if self.collectors.pressure {
                let proc_path = self.sys_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("pressure", pressure::gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.schedstat {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("schedstat", schedstat::gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.sockstat {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("sockstat", sockstat::gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.softnet {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("softnet", softnet::gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.stat {
                let proc_path = self.proc_path.clone();

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
                let sys_path = self.sys_path.clone();

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
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    record_gather!("udp_queues", udp_queues::gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.uname {
                tasks.push(tokio::spawn(async {
                    record_gather!("uname", uname::gather())
                }))
            }

            if let Some(ref conf) = self.collectors.vmstat {
                let conf = conf.clone();
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    record_gather!("vmstat", conf.gather(proc_path.as_ref()))
                }))
            }

            if self.collectors.xfs {
                let sys_path = self.sys_path.clone();
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    record_gather!("xfs", xfs::gather(proc_path.as_ref(), sys_path.as_ref()))
                }))
            }

            if self.collectors.zfs {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    record_gather!("zfs", zfs::gather(proc_path.as_ref()))
                }))
            }

            let metrics = futures::future::join_all(tasks).await
                .iter()
                .flatten()
                .fold(Vec::new(), |mut metrics, ms| {
                    metrics.extend_from_slice(ms);
                    metrics
                });

            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
            let now = now.as_millis() as i64;

            let mut stream = futures::stream::iter(metrics)
                .map(|mut m| {
                    m.timestamp = now;
                    Event::Metric(m)
                })
                .map(Ok);
            out.send_all(&mut stream).await;
        }

        Ok(())
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "node_metrics")]
impl SourceConfig for NodeMetricsConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        let nm = NodeMetrics {
            interval: self.interval.to_std().unwrap(),
            proc_path: default_proc_path().into(),
            sys_path: default_sys_path().into(),
            collectors: self.collectors.clone(),
        };

        Ok(Box::pin(nm.run(ctx.shutdown, ctx.out)))
    }

    fn output_type(&self) -> DataType {
        DataType::Metric
    }

    fn source_type(&self) -> &'static str {
        "node_metrics"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: add more test for default values

    #[test]
    fn test_deserialize() {
        let cs: Collectors = serde_yaml::from_str(r#"
        arp: true
        "#).unwrap();

        println!("{:?}", cs);
    }

    #[test]
    fn test_pwd() {
        let pwd = std::env::current_dir().unwrap();
        println!("{:?}", pwd);
    }
}