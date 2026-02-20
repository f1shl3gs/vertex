use std::path::PathBuf;

use configurable::Configurable;
use event::{Metric, tags, tags::Key};
use framework::config::serde_regex;
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::Error;

#[derive(Clone, Configurable, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Config {
    #[serde(with = "serde_regex")]
    Include(Regex),

    #[serde(with = "serde_regex")]
    Exclude(Regex),

    #[default]
    All,
}

pub async fn gather(conf: Config, proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let data = std::fs::read_to_string(proc_path.join("net/dev"))?;

    let mut metrics = Vec::new();
    for line in data.lines().skip(2) {
        let stat = parse_device_status(line)?;

        match &conf {
            Config::Include(re) => {
                if !re.is_match(stat.name) {
                    continue;
                }
            }
            Config::Exclude(re) => {
                if re.is_match(stat.name) {
                    continue;
                }
            }
            Config::All => {}
        }

        let tags = tags!(Key::from_static("device") => stat.name);
        metrics.extend([
            Metric::sum_with_tags(
                "node_network_receive_bytes_total",
                "Network device statistic receive_bytes",
                stat.recv_bytes,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_network_receive_packets_total",
                "Network device statistic receive_packets",
                stat.recv_packets,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_network_receive_errs_total",
                "Network device statistic receive_errs",
                stat.recv_errs,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_network_receive_drop_total",
                "Network device statistic receive_drop",
                stat.recv_drop,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_network_receive_fifo_total",
                "Network device statistic receive_fifo",
                stat.recv_fifo,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_network_receive_frame_total",
                "Network device statistic receive_frame",
                stat.recv_frame,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_network_receive_compressed_total",
                "Network device statistic receive_compressed",
                stat.recv_compressed,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_network_receive_multicast_total",
                "Network device statistic receive_multicast",
                stat.recv_multicast,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_network_transmit_bytes_total",
                "Network device statistic transmit_bytes",
                stat.transmit_bytes,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_network_transmit_packets_total",
                "Network device statistic transmit_packets",
                stat.transmit_packets,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_network_transmit_errs_total",
                "Network device statistic transmit_errs",
                stat.transmit_errs,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_network_transmit_drop_total",
                "Network device statistic transmit_drop",
                stat.transmit_drop,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_network_transmit_fifo_total",
                "Network device statistic transmit_fifo",
                stat.transmit_fifo,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_network_transmit_colls_total",
                "Network device statistic transmit_colls",
                stat.transmit_colls,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_network_transmit_carrier_total",
                "Network device statistic transmit_carrier",
                stat.transmit_carrier,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_network_transmit_compressed_total",
                "Network device statistic transmit_compressed",
                stat.transmit_compressed,
                tags,
            ),
        ])
    }

    Ok(metrics)
}

#[derive(Debug, PartialEq)]
struct DeviceStatus<'a> {
    name: &'a str,

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

/// parse lines like
/// ```text
/// Inter-|   Receive                                                |  Transmit
///  face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
//     lo: 14748809    4780    0    0    0     0          0         0 14748809    4780    0    0    0     0       0          0
/// ```
fn parse_device_status(line: &str) -> Result<DeviceStatus<'_>, Error> {
    let parts = line.split_ascii_whitespace().collect::<Vec<_>>();

    let name = parts[0].strip_suffix(':').unwrap();
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

    Ok(DeviceStatus {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_status_line() {
        let input = "vethf345468:     648       8    0    0    0     0          0         0      438       5    0    0    0     0       0          0";
        let stats = parse_device_status(input).unwrap();
        assert_eq!(
            stats,
            DeviceStatus {
                name: "vethf345468",
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

        let input = "    lo: 1664039048 1566805    0    0    0     0          0         0 1664039048 1566805    0    0    0     0       0          0";
        let stats = parse_device_status(input).unwrap();
        assert_eq!(
            stats,
            DeviceStatus {
                name: "lo",
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
