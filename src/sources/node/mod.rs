mod btrfs;
mod cpufreq;
mod diskstats;
mod arp;
mod bonding;
mod edac;
mod entropy;
mod fibre_channel;
mod filefd;
mod filesystem;
mod hwmon;
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

use typetag;
use serde::{Deserialize, Serialize};
use crate::sources::Source;
use crate::config::{SourceConfig, SourceContext, DataType, deserialize_duration, serialize_duration, default_true};
use tokio_stream::wrappers::IntervalStream;
use futures::{
    StreamExt,
    SinkExt,
};
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
use crate::sources::node::filesystem::FileSystemConfig;
use tokio::io::AsyncReadExt;
use crate::sources::node::errors::Error;
use std::str::FromStr;
use crate::sources::node::netdev::NetdevConfig;
use crate::sources::node::vmstat::VMStatConfig;
use crate::sources::node::netclass::NetClassConfig;
use crate::sources::node::netstat::NetstatConfig;

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
    pub cpu_freq: bool,

    #[serde(default)]
    pub disk_stats: Option<Arc<DiskStatsConfig>>,

    #[serde(default)]
    pub drm: bool,

    #[serde(default = "default_true")]
    pub edac: bool,

    #[serde(default = "default_true")]
    pub entropy: bool,

    #[serde(default = "default_true")]
    pub filefd: bool,

    #[serde(default)]
    pub filesystem: Option<Arc<FileSystemConfig>>,

    #[serde(default = "default_true")]
    pub hwmon: bool,

    #[serde(default = "default_true")]
    pub loadavg: bool,

    #[serde(default = "default_true")]
    pub memory: bool,

    #[serde(default)]
    pub netclass: Option<Arc<netclass::NetClassConfig>>,

    #[serde(default)]
    pub netdev: Option<Arc<netdev::NetdevConfig>>,

    #[serde(default)]
    pub netstat: Option<Arc<netstat::NetstatConfig>>,

    #[serde(default = "default_true")]
    pub nvme: bool,

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
    pub time: bool,

    #[serde(default = "default_true")]
    pub timex: bool,

    #[serde(default = "default_true")]
    pub uname: bool,

    pub vmstat: Option<Arc<vmstat::VMStatConfig>>,

    #[serde(default = "default_true")]
    pub xfs: bool,
}

impl Default for Collectors {
    fn default() -> Self {
        Self {
            arp: default_true(),
            btrfs: default_true(),
            bonding: default_true(),
            conntrack: default_true(),
            cpu: Some(Arc::new(CPUConfig::default())),
            cpu_freq: true,
            disk_stats: Some(Arc::new(DiskStatsConfig::default())),
            drm: default_true(),
            edac: default_true(),
            entropy: default_true(),
            filefd: default_true(),
            filesystem: Some(Arc::new(FileSystemConfig::default())),
            hwmon: default_true(),
            loadavg: default_true(),
            memory: default_true(),
            netclass: Some(Arc::new(NetClassConfig::default())),
            netdev: Some(Arc::new(NetdevConfig::default())),
            netstat: Some(Arc::new(NetstatConfig::default())),
            nvme: default_true(),
            pressure: default_true(),
            schedstat: default_true(),
            sockstat: default_true(),
            softnet: default_true(),
            stat: default_true(),
            time: default_true(),
            timex: default_true(),
            tcpstat: default_true(),
            uname: default_true(),
            vmstat: Some(Arc::new(VMStatConfig::default())),
            xfs: default_true(),
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

pub async fn read_to_string<P: AsRef<Path>>(path: P) -> Result<String, std::io::Error> {
    let mut f = tokio::fs::File::open(path.as_ref()).await?;
    let mut content = String::new();

    f.read_to_string(&mut content).await?;

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

impl NodeMetrics {
    async fn run(self, shutdown: ShutdownSignal, mut out: Pipeline) -> Result<(), ()> {
        let interval = tokio::time::interval(self.interval);
        let mut ticker = IntervalStream::new(interval)
            .take_until(shutdown);

        while ticker.next().await.is_some() {
            let mut tasks = Vec::with_capacity(16);

            if self.collectors.arp {
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    arp::gather(proc_path.as_ref()).await
                }));
            }

            if self.collectors.bonding {
                let sys_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    bonding::gather(sys_path.as_ref()).await
                }));
            }

            if self.collectors.conntrack {
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    conntrack::gather(proc_path.as_ref()).await
                }))
            }

            if let Some(ref conf) = self.collectors.cpu {
                let proc_path = self.proc_path.clone();
                let conf = conf.clone();

                tasks.push(tokio::spawn(async move {
                    conf.gather(proc_path.as_ref()).await
                }));
            }

            if self.collectors.cpu_freq {
                let sys_path = self.sys_path.clone();

                tasks.push(tokio::spawn(async move {
                    cpufreq::gather(sys_path.as_ref()).await
                }))
            }

            if let Some(ref conf) = self.collectors.disk_stats {
                let conf = conf.clone();
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    conf.gather(proc_path.as_ref()).await
                }))
            }

            if self.collectors.drm {
                let sys_path = self.sys_path.clone();
                tasks.push(tokio::spawn(async move {
                    drm::gather(sys_path.as_ref()).await
                }))
            }

            if self.collectors.edac {
                let sys_path = self.sys_path.clone();

                tasks.push(tokio::spawn(async move {
                    edac::gather(sys_path.as_ref()).await
                }))
            }

            if self.collectors.entropy {
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    entropy::gather(proc_path.as_ref()).await
                }))
            }

            if self.collectors.filefd {
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    filefd::gather(proc_path.as_ref()).await
                }))
            }

            if let Some(ref conf) = self.collectors.filesystem {
                let proc_path = self.proc_path.clone();
                let conf = conf.clone();

                tasks.push(tokio::spawn(async move {
                    conf.gather(proc_path.as_ref()).await
                }))
            }

            if self.collectors.hwmon {
                let sys_path = self.sys_path.clone();
                tasks.push(tokio::spawn(async move {
                    hwmon::gather(sys_path.as_ref()).await
                }))
            }

            if self.collectors.loadavg {
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    loadavg::gather(proc_path.as_ref()).await
                }))
            }

            if self.collectors.memory {
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    meminfo::gather(proc_path.as_ref()).await
                }))
            }

            if let Some(ref conf) = self.collectors.netclass {
                let conf = conf.clone();
                let sys_path = self.sys_path.clone();

                tasks.push(tokio::spawn(async move {
                    netclass::gather(conf.as_ref(), sys_path.as_ref()).await
                }))
            }

            if let Some(ref conf) = self.collectors.netdev {
                let conf = conf.clone();
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    conf.gather(proc_path.as_ref()).await
                }))
            }

            if let Some(ref conf) = self.collectors.netstat {
                let conf = conf.clone();
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    netstat::gather(conf.as_ref(), proc_path.as_ref()).await
                }))
            }

            if self.collectors.nvme {
                let sys_path = self.sys_path.clone();

                tasks.push(tokio::spawn(async move {
                    nvme::gather(sys_path.as_ref()).await
                }))
            }

            if self.collectors.pressure {
                let proc_path = self.sys_path.clone();
                tasks.push(tokio::spawn(async move {
                    pressure::gather(proc_path.as_ref()).await
                }))
            }

            if self.collectors.schedstat {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    schedstat::gather(proc_path.as_ref()).await
                }))
            }

            if self.collectors.sockstat {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    sockstat::gather(proc_path.as_ref()).await
                }))
            }

            if self.collectors.softnet {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async move {
                    softnet::gather(proc_path.as_ref()).await
                }))
            }

            if self.collectors.stat {
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    stat::gather(proc_path.as_ref()).await
                }))
            }

            if self.collectors.tcpstat {
                tasks.push(tokio::spawn(async {
                    tcpstat::gather().await
                }));
            }

            if self.collectors.time {
                tasks.push(tokio::spawn(async {
                    time::gather().await
                }))
            }

            if self.collectors.timex {
                tasks.push(tokio::spawn(async {
                    timex::gather().await
                }))
            }

            if self.collectors.uname {
                tasks.push(tokio::spawn(async {
                    uname::gather().await
                }))
            }

            if let Some(ref conf) = self.collectors.vmstat {
                let conf = conf.clone();
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    conf.gather(proc_path.as_ref()).await
                }))
            }

            if self.collectors.xfs {
                let sys_path = self.sys_path.clone();
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    xfs::gather(proc_path.as_ref(), sys_path.as_ref()).await
                }))
            }

            let metrics = futures::future::join_all(tasks).await
                .iter()
                .flatten()
                .fold(Vec::new(), |mut metrics, result| {
                    if let Ok(ms) = result {
                        metrics.extend_from_slice(ms)
                    }

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