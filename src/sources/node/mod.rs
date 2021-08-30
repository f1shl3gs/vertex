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
use std::sync::Arc;

use cpu::CPUConfig;
use diskstats::DiskStatsConfig;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::sources::node::filesystem::FileSystemConfig;
use std::path::{PathBuf, Path};
use tokio::io::AsyncReadExt;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Collectors {
    #[serde(default = "default_true")]
    pub arp: bool,

    #[serde(default = "default_true")]
    pub btrfs: bool,

    #[serde(default = "default_true")]
    pub bonding: bool,

    pub cpu: Option<Arc<CPUConfig>>,

    #[serde(default = "default_true")]
    pub cpu_freq: bool,

    #[serde(default)]
    pub disk_stats: Option<Arc<DiskStatsConfig>>,

    #[serde(default)]
    pub filesystem: Option<Arc<FileSystemConfig>>,

    #[serde(default = "default_true")]
    pub loadavg: bool,

    #[serde(default = "default_true")]
    pub memory: bool,

    #[serde(default = "default_true")]
    pub nvme: bool,

    #[serde(default = "default_true")]
    pub tcpstat: bool,
}

impl Default for Collectors {
    fn default() -> Self {
        Self {
            arp: default_true(),
            btrfs: default_true(),
            bonding: default_true(),
            cpu: Some(Arc::new(CPUConfig::default())),
            cpu_freq: true,
            disk_stats: Some(Arc::new(DiskStatsConfig::default())),
            filesystem: Some(Arc::new(FileSystemConfig::default())),
            loadavg: default_true(),
            memory: default_true(),
            nvme: default_true(),
            tcpstat: default_true(),
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

impl NodeMetrics {
    async fn run(self, shutdown: ShutdownSignal, mut out: Pipeline) -> Result<(), ()> {
        let interval = tokio::time::interval(self.interval);
        let mut ticker = IntervalStream::new(interval)
            .take_until(shutdown);

        while let Some(_) = ticker.next().await {
            let mut tasks = Vec::with_capacity(16);

            if self.collectors.arp {
                let proc_path = self.proc_path.clone();
                tasks.push(tokio::spawn(async {
                    arp::gather(proc_path).await
                }));
            }

            if self.collectors.bonding {
                let sys_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async {
                    bonding::gather(sys_path).await
                }));
            }

            if let Some(ref conf) = self.collectors.cpu {
                let proc_path = self.proc_path.clone();
                let conf = conf.clone();

                tasks.push(tokio::spawn(async move {
                    conf.gather(proc_path).await
                }));
            }

            if let Some(ref conf) = self.collectors.filesystem {
                let proc_path = self.proc_path.clone();
                let conf = conf.clone();

                tasks.push(tokio::spawn(async move {
                    conf.gather(proc_path).await
                }))
            }

            if self.collectors.loadavg {
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    loadavg::gather(proc_path).await
                }))
            }

            if self.collectors.tcpstat {
                tasks.push(tokio::spawn(async {
                    tcpstat::gather().await
                }));
            }

            if let Some(ref conf) = self.collectors.disk_stats {
                let conf = conf.clone();

                tasks.push(tokio::spawn(async move {
                    conf.gather().await
                }))
            }

            if self.collectors.memory {
                let proc_path = self.proc_path.clone();

                tasks.push(tokio::spawn(async move {
                    meminfo::gather(proc_path).await
                }))
            }

            if self.collectors.nvme {
                let sys_path = self.sys_path.clone();

                tasks.push(tokio::spawn(async move {
                    nvme::gather(sys_path).await
                }))
            }

            let metrics = futures::future::join_all(tasks).await
                .iter()
                .flatten()
                .fold(Vec::new(), |mut metrics, result| {
                    match result {
                        Ok(ms) => {
                            metrics.extend_from_slice(ms)
                        }
                        _ => {}
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