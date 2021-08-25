mod boot_time;
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

use typetag;
use serde::{Deserialize, Serialize};

use crate::sources::Source;
use crate::config::{SourceConfig, SourceContext, DataType};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
enum Collector {
    Cpu,
    Disk,
    Filesystem,
    Load,
    Memory,
    Network,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    interval_sec: u64,

    collectors: Option<Vec<Collector>>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct NodeMetrics {}

#[async_trait::async_trait]
#[typetag::serde(name = "node_metrics")]
impl SourceConfig for NodeMetrics {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        todo!()
    }

    fn output_type(&self) -> DataType {
        todo!()
    }

    fn source_type(&self) -> &'static str {
        todo!()
    }
}

impl NodeMetrics {

}

#[cfg(test)]
mod tests {
    use tokio_stream::{StreamExt};
    use tokio::sync::oneshot;
    use tokio_stream::wrappers::IntervalStream;

    #[tokio::test]
    async fn test_tokio_select() {
        println!("start");

        let (tx, rx) = oneshot::channel();

        let _h = tokio::spawn(async {
            let sleep = tokio::time::sleep(tokio::time::Duration::from_secs(5));
            sleep.await;

            tx.send(true).unwrap();
        });

        let sleep = tokio::time::sleep(tokio::time::Duration::from_secs(6));

        tokio::select! {
            val = rx => {
                assert_eq!(val.unwrap(), true);
                println!("rx");
            },
            _ = sleep => {
                assert_eq!(true, false);
                println!("sleep done");
            }
        }

        println!("done")
    }

    #[tokio::test]
    async fn tick_till_done() {
        let (tx, rx) = oneshot::channel();

        tokio::spawn(async {
            let sleep = tokio::time::sleep(tokio::time::Duration::from_secs(5));
            sleep.await;

            tx.send(true).unwrap();
        });

        tokio::select!{
            _ = async {
                let d = tokio::time::Duration::from_secs(1);
                let interval = tokio::time::interval(d);
                let mut stream = IntervalStream::new(interval);

                while let Some(ts) = stream.next().await {
                    println!("{:?}", ts);
                }

                println!("tick done")
            } => {},

            _ = rx => {
                println!("done")
            }
        }
    }
}