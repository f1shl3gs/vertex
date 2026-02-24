use std::io::ErrorKind;
use std::path::PathBuf;

use event::{Metric, tags};

use super::{Error, read_string};

#[derive(Debug, Default)]
pub struct FibreChannelCounters {
    // /sys/class/fc_host/<Name>/statistics/dumped_frames
    dumped_frames: u64,

    // /sys/class/fc_host/<Name>/statistics/error_frames
    error_frames: u64,

    // /sys/class/fc_host/<Name>/statistics/invalid_crc_count
    invalid_crc_count: u64,

    // /sys/class/fc_host/<Name>/statistics/rx_frames
    rx_frames: u64,

    // /sys/class/fc_host/<Name>/statistics/rx_words
    rx_words: u64,

    // /sys/class/fc_host/<Name>/statistics/tx_frames
    tx_frames: u64,

    // /sys/class/fc_host/<Name>/statistics/tx_words
    tx_words: u64,

    // /sys/class/fc_host/<Name>/statistics/seconds_since_last_reset
    seconds_since_last_reset: u64,

    // /sys/class/fc_host/<Name>/statistics/invalid_tx_word_count
    invalid_tx_word_count: u64,

    // /sys/class/fc_host/<Name>/statistics/link_failure_count
    link_failure_count: u64,

    // /sys/class/fc_host/<Name>/statistics/loss_of_sync_count
    loss_of_sync_count: u64,

    // /sys/class/fc_host/<Name>/statistics/loss_of_signal_count
    loss_of_signal_count: u64,

    // /sys/class/fc_host/<Name>/statistics/nos_count
    nos_count: u64,

    // / sys/class/fc_host/<Name>/statistics/fcp_packet_aborts
    fcp_packet_aborts: u64,
}

#[derive(Debug, Default)]
pub struct FibreChannelHost {
    // /sys/class/fc_host/<Name>
    name: String,

    // /sys/class/fc_host/<Name>/speed
    speed: String,

    // /sys/class/fc_host/<Name>/port_state
    port_state: String,

    // /sys/class/fc_host/<Name>/port_type
    port_type: String,

    // /sys/class/fc_host/<Name>/symbolic_name
    symbolic_name: String,

    // /sys/class/fc_host/<Name>/node_name
    node_name: String,

    // /sys/class/fc_host/<Name>/port_id
    port_id: String,

    // /sys/class/fc_host/<Name>/port_name
    port_name: String,

    // /sys/class/fc_host/<Name>/fabric_name
    fabric_name: String,

    // /sys/class/fc_host/<Name>/dev_loss_tmo
    dev_loss_tmo: String,

    // /sys/class/fc_host/<Name>/supported_classes
    supported_classes: String,

    // /sys/class/fc_host/<Name>/supported_speeds
    supported_speeds: String,

    // /sys/class/fc_host/<Name>/statistics/*
    counters: FibreChannelCounters,
}

/// fibre_channel_class parse everything in /sys/class/fc_host
fn fibre_channel_class(sys_path: PathBuf) -> Result<Vec<FibreChannelHost>, Error> {
    let dirs = std::fs::read_dir(sys_path.join("class/fc_host"))?;

    let mut fcc = Vec::new();
    for entry in dirs.flatten() {
        let host = parse_fibre_channel_host(entry.path())?;
        fcc.push(host);
    }

    Ok(fcc)
}

fn parse_fibre_channel_host(root: PathBuf) -> Result<FibreChannelHost, Error> {
    let mut host = FibreChannelHost {
        name: root.file_name().unwrap().to_string_lossy().to_string(),
        ..Default::default()
    };

    for sub in [
        "speed",
        "port_state",
        "port_type",
        "node_name",
        "port_id",
        "port_name",
        "fabric_name",
        "dev_loss_tmo",
        "symbolic_name",
        "supported_classes",
        "supported_speeds",
    ] {
        let value = match read_string(root.join(sub)) {
            Ok(value) => value,
            Err(err) => {
                if err.kind() == ErrorKind::NotFound {
                    continue;
                }

                return Err(err.into());
            }
        };

        match sub {
            "speed" => host.speed = value,
            "port_state" => host.port_state = value,
            "port_type" => host.port_type = value,
            "node_name" => {
                host.node_name = match value.len() {
                    v if v > 2 => value[2..].to_string(),
                    _ => value,
                }
            }
            "port_id" => {
                host.port_id = match value.len() {
                    v if v > 2 => value[2..].to_string(),
                    _ => value,
                }
            }
            "port_name" => {
                host.port_name = match value.len() {
                    v if v > 2 => value[2..].to_string(),
                    _ => value,
                }
            }
            "fabric_name" => {
                host.fabric_name = match value.len() {
                    v if v > 2 => value[2..].to_string(),
                    _ => value,
                }
            }
            "dev_loss_tmo" => host.dev_loss_tmo = value,
            "supported_classes" => host.supported_classes = value,
            "supported_speeds" => host.supported_speeds = value,
            "symbolic_name" => host.symbolic_name = value,
            _ => {}
        }
    }

    host.counters = parse_fibre_channel_statistics(root)?;

    Ok(host)
}

fn read_hex(path: PathBuf) -> Result<u64, Error> {
    let content = read_string(path)?
        .trim_end()
        .strip_prefix("0x")
        .unwrap()
        .to_string();

    u64::from_str_radix(&content, 16).map_err(Into::into)
}

/// parse_fibre_channel_statistics parses metrics from a single FC host
fn parse_fibre_channel_statistics(root: PathBuf) -> Result<FibreChannelCounters, Error> {
    let dirs = std::fs::read_dir(root.join("statistics"))?;

    let mut counters = FibreChannelCounters::default();
    for entry in dirs.flatten() {
        let name = entry.file_name();
        let path = entry.path();

        match name.to_str().unwrap() {
            "dumped_frames" => counters.dumped_frames = read_hex(path)?,
            "error_frames" => counters.error_frames = read_hex(path)?,
            "fcp_packet_aborts" => counters.fcp_packet_aborts = read_hex(path)?,
            "invalid_tx_word_count" => counters.invalid_tx_word_count = read_hex(path)?,
            "invalid_crc_count" => counters.invalid_crc_count = read_hex(path)?,
            "link_failure_count" => counters.link_failure_count = read_hex(path)?,
            "loss_of_signal_count" => counters.loss_of_signal_count = read_hex(path)?,
            "loss_of_sync_count" => counters.loss_of_sync_count = read_hex(path)?,
            "nos_count" => counters.nos_count = read_hex(path)?,
            "rx_frames" => counters.rx_frames = read_hex(path)?,
            "rx_words" => counters.rx_words = read_hex(path)?,
            "seconds_since_last_reset" => counters.seconds_since_last_reset = read_hex(path)?,
            "tx_frames" => counters.tx_frames = read_hex(path)?,
            "tx_words" => counters.tx_words = read_hex(path)?,
            _ => {}
        }
    }

    Ok(counters)
}

pub async fn gather(sys_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let hosts = fibre_channel_class(sys_path)?;

    let mut metrics = vec![];
    for host in hosts {
        let name = host.name;

        metrics.push(Metric::gauge_with_tags(
            "node_fibrechannel_info",
            "Non-numeric data from /sys/class/fc_host/<host>, value is always 1.",
            1,
            tags!(
                "dev_loss_tmo" => host.dev_loss_tmo,
                "fabric_name" => host.fabric_name,
                "fc_host" => &name,
                "port_id" => host.port_id,
                "port_name" => host.port_name,
                "port_state" => host.port_state,
                "port_type" => host.port_type,
                "speed" => host.speed,
                "supported_classes" => host.supported_classes,
                "supported_speeds" => host.supported_speeds,
                "symbolic_name" => host.symbolic_name,
            ),
        ));

        if host.counters.dumped_frames != u64::MAX {
            metrics.push(Metric::sum_with_tags(
                "node_fibrechannel_dumped_frames_total",
                "Number of dumped frames",
                host.counters.dumped_frames,
                tags!(
                    "fc_host" => &name
                ),
            ));
        }
        if host.counters.error_frames != u64::MAX {
            metrics.push(Metric::sum_with_tags(
                "node_fibrechannel_error_frames_total",
                "Number of errors in frames",
                host.counters.error_frames,
                tags!(
                    "fc_host" => &name
                ),
            ));
        }
        if host.counters.invalid_crc_count != u64::MAX {
            metrics.push(Metric::sum_with_tags(
                "node_fibrechannel_invalid_crc_total",
                "Invalid Cyclic Redundancy Check count",
                host.counters.invalid_crc_count,
                tags!(
                    "fc_host" => &name
                ),
            ));
        }
        if host.counters.rx_frames != u64::MAX {
            metrics.push(Metric::sum_with_tags(
                "node_fibrechannel_rx_frames_total",
                "Number of frames received",
                host.counters.rx_frames,
                tags!(
                    "fc_host" => &name
                ),
            ));
        }
        if host.counters.rx_words != u64::MAX {
            metrics.push(Metric::sum_with_tags(
                "node_fibrechannel_rx_words_total",
                "Number of words received by host port",
                host.counters.rx_words,
                tags!(
                    "fc_host" => &name
                ),
            ));
        }
        if host.counters.tx_frames != u64::MAX {
            metrics.push(Metric::sum_with_tags(
                "node_fibrechannel_tx_frames_total",
                "Number of frames transmitted by host port",
                host.counters.tx_frames,
                tags!(
                    "fc_host" => &name
                ),
            ));
        }
        if host.counters.tx_words != u64::MAX {
            metrics.push(Metric::sum_with_tags(
                "node_fibrechannel_tx_words_total",
                "Number of words transmitted by host port",
                host.counters.tx_words,
                tags!(
                    "fc_host" => &name
                ),
            ));
        }
        if host.counters.seconds_since_last_reset != u64::MAX {
            metrics.push(Metric::sum_with_tags(
                "node_fibrechannel_seconds_since_last_reset_total",
                "Number of seconds since last host port reset",
                host.counters.seconds_since_last_reset,
                tags!(
                    "fc_host" => &name
                ),
            ));
        }
        if host.counters.invalid_tx_word_count != u64::MAX {
            metrics.push(Metric::sum_with_tags(
                "node_fibrechannel_invalid_tx_words_total",
                "Number of invalid words transmitted by host port",
                host.counters.invalid_tx_word_count,
                tags!(
                    "fc_host" => &name
                ),
            ));
        }
        if host.counters.link_failure_count != u64::MAX {
            metrics.push(Metric::sum_with_tags(
                "node_fibrechannel_link_failure_total",
                "Number of times the host port link has failed",
                host.counters.link_failure_count,
                tags!(
                    "fc_host" => &name
                ),
            ));
        }
        if host.counters.loss_of_sync_count != u64::MAX {
            metrics.push(Metric::sum_with_tags(
                "node_fibrechannel_loss_of_sync_total",
                "Number of failures on either bit or transmission word boundaries",
                host.counters.loss_of_sync_count,
                tags!(
                    "fc_host" => &name
                ),
            ));
        }
        if host.counters.loss_of_signal_count != u64::MAX {
            metrics.push(Metric::sum_with_tags(
                "node_fibrechannel_loss_of_signal_total",
                "Number of times signal has been lost",
                host.counters.loss_of_signal_count,
                tags!(
                    "fc_host" => &name
                ),
            ));
        }
        if host.counters.nos_count != u64::MAX {
            metrics.push(Metric::sum_with_tags(
                "node_fibrechannel_nos_total",
                "Number Not_Operational Primitive Sequence received by host port",
                host.counters.nos_count,
                tags!(
                    "fc_host" => &name
                ),
            ));
        }
        if host.counters.fcp_packet_aborts != u64::MAX {
            metrics.push(Metric::sum_with_tags(
                "node_fibrechannel_fcp_packet_aborts_total",
                "Number of aborted packets",
                host.counters.fcp_packet_aborts,
                tags!(
                    "fc_host" => name
                ),
            ));
        }
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let fcc = fibre_channel_class("tests/node/sys".into()).unwrap();
        assert_eq!(fcc.len(), 1);
        let host = &fcc[0];

        assert_eq!(host.name, "host0");
        assert_eq!(host.speed, "16 Gbit");
        assert_eq!(host.port_state, "Online");
        assert_eq!(host.port_type, "Point-To-Point (direct nport connection)");
        assert_eq!(host.port_name, "1000e0071bce95f2");
        assert_eq!(
            host.symbolic_name,
            "Emulex SN1100E2P FV12.4.270.3 DV12.4.0.0. HN:gotest. OS:Linux"
        );
        assert_eq!(host.node_name, "2000e0071bce95f2");
        assert_eq!(host.port_id, "000002");
        assert_eq!(host.fabric_name, "0");
        assert_eq!(host.dev_loss_tmo, "30");
        assert_eq!(host.supported_classes, "Class 3");
        assert_eq!(host.supported_speeds, "4 Gbit, 8 Gbit, 16 Gbit");
        assert_eq!(host.counters.dumped_frames, u64::MAX);
        assert_eq!(host.counters.error_frames, 0);
        assert_eq!(host.counters.invalid_crc_count, 0x2);
        assert_eq!(host.counters.rx_frames, 0x3);
        assert_eq!(host.counters.rx_words, 0x4);
        assert_eq!(host.counters.tx_frames, 0x5);
        assert_eq!(host.counters.tx_words, 0x6);
        assert_eq!(host.counters.seconds_since_last_reset, 0x7);
        assert_eq!(host.counters.invalid_tx_word_count, 0x8);
        assert_eq!(host.counters.link_failure_count, 0x9);
        assert_eq!(host.counters.loss_of_sync_count, 0x10);
        assert_eq!(host.counters.loss_of_signal_count, 0x11);
        assert_eq!(host.counters.nos_count, 0x12);
        assert_eq!(host.counters.fcp_packet_aborts, 0x13);
    }
}
