use super::{read_to_string, Error, ErrorContext};
use event::{tags, Metric};
use framework::config::serde_regex;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::num::ParseIntError;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NetdevConfig {
    #[serde(with = "serde_regex")]
    Include(Regex),

    #[serde(with = "serde_regex")]
    Exclude(Regex),

    All,
}

impl Default for NetdevConfig {
    fn default() -> Self {
        Self::All
    }
}

impl NetdevConfig {
    pub async fn gather(&self, proc_path: &str) -> Result<Vec<Metric>, Error> {
        let stats = self
            .get_net_dev_stats(proc_path)
            .await
            .context("get netdev stats failed")?;

        let mut metrics = Vec::new();
        for stat in stats {
            let device = &stat.name;

            match self {
                NetdevConfig::Include(re) => {
                    if !re.is_match(device) {
                        continue;
                    }
                }
                NetdevConfig::Exclude(re) => {
                    if re.is_match(device) {
                        continue;
                    }
                }
                NetdevConfig::All => {}
            }

            metrics.extend_from_slice(&[
                Metric::sum_with_tags(
                    "node_network_receive_bytes_total",
                    "Network device statistic receive_bytes",
                    stat.recv_bytes as f64,
                    tags!(
                        "device" => device,
                    ),
                ),
                Metric::sum_with_tags(
                    "node_network_receive_packets_total",
                    "Network device statistic receive_packets",
                    stat.recv_packets as f64,
                    tags!(
                        "device" => device,
                    ),
                ),
                Metric::sum_with_tags(
                    "node_network_receive_errs_total",
                    "Network device statistic receive_errs",
                    stat.recv_errs as f64,
                    tags!(
                        "device" => device,
                    ),
                ),
                Metric::sum_with_tags(
                    "node_network_receive_drop_total",
                    "Network device statistic receive_drop",
                    stat.recv_drop as f64,
                    tags!(
                        "device" => device,
                    ),
                ),
                Metric::sum_with_tags(
                    "node_network_receive_fifo_total",
                    "Network device statistic receive_fifo",
                    stat.recv_fifo as f64,
                    tags!(
                        "device" => device,
                    ),
                ),
                Metric::sum_with_tags(
                    "node_network_receive_frame_total",
                    "Network device statistic receive_frame",
                    stat.recv_frame as f64,
                    tags!(
                        "device" => device,
                    ),
                ),
                Metric::sum_with_tags(
                    "node_network_receive_compressed_total",
                    "Network device statistic receive_compressed",
                    stat.recv_compressed as f64,
                    tags!(
                        "device" => device,
                    ),
                ),
                Metric::sum_with_tags(
                    "node_network_receive_multicast_total",
                    "Network device statistic receive_multicast",
                    stat.recv_multicast as f64,
                    tags!(
                        "device" => device,
                    ),
                ),
                Metric::sum_with_tags(
                    "node_network_transmit_bytes_total",
                    "Network device statistic transmit_bytes",
                    stat.transmit_bytes as f64,
                    tags!(
                        "device" => device,
                    ),
                ),
                Metric::sum_with_tags(
                    "node_network_transmit_packets_total",
                    "Network device statistic transmit_packets",
                    stat.transmit_packets as f64,
                    tags!(
                        "device" => device,
                    ),
                ),
                Metric::sum_with_tags(
                    "node_network_transmit_errs_total",
                    "Network device statistic transmit_errs",
                    stat.transmit_errs as f64,
                    tags!(
                        "device" => device,
                    ),
                ),
                Metric::sum_with_tags(
                    "node_network_transmit_drop_total",
                    "Network device statistic transmit_drop",
                    stat.transmit_drop as f64,
                    tags!(
                        "device" => device,
                    ),
                ),
                Metric::sum_with_tags(
                    "node_network_transmit_fifo_total",
                    "Network device statistic transmit_fifo",
                    stat.transmit_fifo as f64,
                    tags!(
                        "device" => device,
                    ),
                ),
                Metric::sum_with_tags(
                    "node_network_transmit_colls_total",
                    "Network device statistic transmit_colls",
                    stat.transmit_colls as f64,
                    tags!(
                        "device" => device,
                    ),
                ),
                Metric::sum_with_tags(
                    "node_network_transmit_carrier_total",
                    "Network device statistic transmit_carrier",
                    stat.transmit_carrier as f64,
                    tags!(
                        "device" => device,
                    ),
                ),
                Metric::sum_with_tags(
                    "node_network_transmit_compressed_total",
                    "Network device statistic transmit_compressed",
                    stat.transmit_compressed as f64,
                    tags!(
                        "device" => device,
                    ),
                ),
            ])
        }

        Ok(metrics)
    }

    async fn get_net_dev_stats(&self, proc_path: &str) -> Result<Vec<DeviceStatus>, Error> {
        let path = format!("{}/net/dev", proc_path);

        let content = read_to_string(path).await?;
        let lines = content.lines();
        let mut stats = Vec::new();
        for line in lines.skip(2) {
            let stat = DeviceStatus::from_str(line)?;
            stats.push(stat);
        }

        Ok(stats)
    }
}

#[derive(Debug, PartialEq)]
struct DeviceStatus {
    name: String,

    recv_bytes: u64,
    recv_packets: u64,
    recv_errs: u64,
    recv_drop: u64,
    recv_fifo: u64,
    recv_frame: u64,
    recv_compressed: u64,
    recv_multicast: u64,

    transmit_bytes: u64,
    transmit_packets: u64,
    transmit_errs: u64,
    transmit_drop: u64,
    transmit_fifo: u64,
    transmit_colls: u64,
    transmit_carrier: u64,
    transmit_compressed: u64,
}

impl DeviceStatus {
    /// parse lines like
    /// ```text
    ///  face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
    //     lo: 14748809    4780    0    0    0     0          0         0 14748809    4780    0    0    0     0       0          0
    /// ```
    fn from_str(s: &str) -> Result<Self, ParseIntError> {
        let parts = s.trim().split_ascii_whitespace().collect::<Vec<_>>();

        let name = parts[0].strip_suffix(':').unwrap().to_string();
        let recv_bytes = parts[1].parse()?;
        let recv_packets = parts[2].parse()?;
        let recv_errs = parts[3].parse()?;
        let recv_drop = parts[4].parse()?;
        let recv_fifo = parts[5].parse()?;
        let recv_frame = parts[6].parse()?;
        let recv_compressed = parts[7].parse()?;
        let recv_multicast = parts[8].parse()?;
        let transmit_bytes = parts[9].parse()?;
        let transmit_packets = parts[10].parse()?;
        let transmit_errs = parts[11].parse()?;
        let transmit_drop = parts[12].parse()?;
        let transmit_fifo = parts[13].parse()?;
        let transmit_colls = parts[14].parse()?;
        let transmit_carrier = parts[15].parse()?;
        let transmit_compressed = parts[16].parse()?;

        Ok(Self {
            name,
            recv_bytes,
            recv_packets,
            recv_errs,
            recv_drop,
            recv_fifo,
            recv_frame,
            recv_compressed,
            recv_multicast,
            transmit_bytes,
            transmit_packets,
            transmit_errs,
            transmit_drop,
            transmit_fifo,
            transmit_colls,
            transmit_carrier,
            transmit_compressed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize() {
        #[derive(Deserialize)]
        struct Dummy {
            #[serde(with = "serde_yaml::with::singleton_map")]
            config: NetdevConfig,
        }

        serde_yaml::from_str::<Dummy>(
            r#"
config:
    include: .*
        "#,
        )
        .unwrap();
    }

    #[test]
    fn test_device_status_from_str() {
        let s = "  lo: 14748809    4780    0    0    0     0          0         0 14748809    4780    0    0    0     0       0          0";
        let ds = DeviceStatus::from_str(s).unwrap();

        assert_eq!(ds.recv_bytes, 14748809);
        assert_eq!(ds.recv_packets, 4780);

        assert_eq!(ds.transmit_bytes, 14748809);
        assert_eq!(ds.transmit_packets, 4780);
    }

    #[tokio::test]
    async fn test_get_net_dev_stats() {
        let conf = NetdevConfig::Include(regex::Regex::new(".*").unwrap());
        let path = "tests/fixtures/proc";
        let stats = conf.get_net_dev_stats(path).await.unwrap();

        assert_eq!(
            stats[0],
            DeviceStatus {
                name: "vethf345468".to_string(),
                recv_bytes: 648,
                recv_packets: 8,
                recv_errs: 0,
                recv_drop: 0,
                recv_fifo: 0,
                recv_frame: 0,
                recv_compressed: 0,
                recv_multicast: 0,
                transmit_bytes: 438,
                transmit_packets: 5,
                transmit_errs: 0,
                transmit_drop: 0,
                transmit_fifo: 0,
                transmit_colls: 0,
                transmit_carrier: 0,
                transmit_compressed: 0,
            }
        );

        assert_eq!(
            stats[1],
            DeviceStatus {
                name: "lo".to_string(),
                recv_bytes: 1664039048,
                recv_packets: 1566805,
                recv_errs: 0,
                recv_drop: 0,
                recv_fifo: 0,
                recv_frame: 0,
                recv_compressed: 0,
                recv_multicast: 0,
                transmit_bytes: 1664039048,
                transmit_packets: 1566805,
                transmit_errs: 0,
                transmit_drop: 0,
                transmit_fifo: 0,
                transmit_colls: 0,
                transmit_carrier: 0,
                transmit_compressed: 0,
            }
        )
    }
}
