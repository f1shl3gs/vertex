use crate::sources::node::errors::{Error, ErrorContext};
use std::collections::BTreeMap;
use crate::sources::node::{read_to_string, read_into};
use std::path::PathBuf;

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
pub async fn fibre_channel_class(sys_path: &str) -> Result<Vec<FibreChannelHost>, Error> {
    let mut fcc = Vec::new();
    let path = format!("{}/class/fc_host", sys_path);
    for dir in std::fs::read_dir(path)? {
        if let Ok(entry) = dir {
            let host = parse_fibre_channel_host(entry.path()).await?;
            fcc.push(host);
        }
    }

    Ok(fcc)
}

async fn parse_fibre_channel_host(root: PathBuf) -> Result<FibreChannelHost, Error> {
    let mut host = FibreChannelHost::default();
    host.name = root.file_name().unwrap().to_str().unwrap().to_string();

    for sub in vec!["speed", "port_state", "port_type", "node_name", "port_id", "port_name", "fabric_name", "dev_loss_tmo", "symbolic_name", "supported_classes", "supported_speeds"] {
        let name = root.join(sub);
        let value = read_to_string(name).await
            .context("failed to read sub file")?
            .trim_end()
            .to_string();

        match sub {
            "speed" => host.speed = value,
            "port_state" => host.port_state = value,
            "port_type" => host.port_type = value,
            "node_name" => host.node_name = match value.len() {
                v if v > 2 => value[2..].to_string(),
                _ => value,
            },
            "port_id" => host.port_id = match value.len() {
                v if v > 2 => value[2..].to_string(),
                _ => value
            },
            "port_name" => host.port_name = match value.len() {
                v if v > 2 => value[2..].to_string(),
                _ => value
            },
            "fabric_name" => host.fabric_name = match value.len() {
                v if v > 2 => value[2..].to_string(),
                _ => value
            },
            "dev_loss_tmo" => host.dev_loss_tmo = value,
            "supported_classes" => host.supported_classes = value,
            "supported_speeds" => host.supported_speeds = value,
            "symbolic_name" => host.symbolic_name = value,
            _ => {}
        }
    }

    host.counters = parse_fibre_channel_statistics(root).await?;

    Ok(host)
}

async fn read_hex(path: PathBuf) -> Result<u64, Error> {
    let content = read_to_string(path).await?
        .trim_end()
        .strip_prefix("0x")
        .unwrap()
        .to_string();

    let v = u64::from_str_radix(&content, 16)?;

    Ok(v)
}

/// parse_fibre_channel_statistics parses metrics from a single FC host
async fn parse_fibre_channel_statistics(root: PathBuf) -> Result<FibreChannelCounters, Error> {
    let mut counters = FibreChannelCounters::default();

    let path = root.join("statistics");
    for dir in std::fs::read_dir(path)? {
        let ent = match dir {
            Ok(ent) => ent,
            _ => continue
        };

        let name = ent.file_name();
        let path = ent.path();
        match name.to_str().unwrap() {
            "dumped_frames" => counters.dumped_frames = read_hex(path).await?,
            "error_frames" => counters.error_frames = read_hex(path).await?,
            "fcp_packet_aborts" => counters.fcp_packet_aborts = read_hex(path).await?,
            "invalid_tx_word_count" => counters.invalid_tx_word_count = read_hex(path).await?,
            "invalid_crc_count" => counters.invalid_crc_count = read_hex(path).await?,
            "link_failure_count" => counters.link_failure_count = read_hex(path).await?,
            "loss_of_signal_count" => counters.loss_of_signal_count = read_hex(path).await?,
            "loss_of_sync_count" => counters.loss_of_sync_count = read_hex(path).await?,
            "nos_count" => counters.nos_count = read_hex(path).await?,
            "rx_frames" => counters.rx_frames = read_hex(path).await?,
            "rx_words" => counters.rx_words = read_hex(path).await?,
            "seconds_since_last_reset" => counters.seconds_since_last_reset = read_hex(path).await?,
            "tx_frames" => counters.tx_frames = read_hex(path).await?,
            "tx_words" => counters.tx_words = read_hex(path).await?,
            _ => {}
        }
    }

    Ok(counters)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fibre_channel_class() {
        let fcc = fibre_channel_class("testdata/sys").await.unwrap();
        assert_eq!(fcc.len(), 1);
        let host = &fcc[0];

        assert_eq!(host.name, "host0");
        assert_eq!(host.speed, "16 Gbit");
        assert_eq!(host.port_state, "Online");
        assert_eq!(host.port_type, "Point-To-Point (direct nport connection)");
        assert_eq!(host.port_name, "1000e0071bce95f2");
        assert_eq!(host.symbolic_name, "Emulex SN1100E2P FV12.4.270.3 DV12.4.0.0. HN:gotest. OS:Linux");
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