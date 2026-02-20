use std::path::{Path, PathBuf};

use event::{Metric, tags, tags::Key};

use super::{Error, read_string};

/// InfiniBandCounters contains counter values from files in
/// /sys/class/infiniband/<Name>/ports/<Port>/counters or
/// /sys/class/infiniband/<Name>/ports/<Port>/counters_ext
/// for a single port of one InfiniBand device.
#[derive(Debug, Default, PartialEq)]
struct InfiniBandCounters {
    // counters_ext/port_multicast_rcv_packets
    port_multicast_rcv_packets: Option<u64>,

    // counters_ext/port_multicast_xmit_packets
    port_multicast_xmit_packets: Option<u64>,

    // counters_ext/port_rcv_data_64
    port_rcv_data_64: Option<u64>,

    // counters_ext/port_rcv_packets_64
    port_rcv_packets_64: Option<u64>,

    // counters_ext/port_unicast_rcv_packets
    port_unicast_rcv_packets: Option<u64>,

    // counters_ext/port_unicast_xmit_packets
    port_unicast_xmit_packets: Option<u64>,

    // counters_ext/port_xmit_data_64
    port_xmit_data_64: Option<u64>,

    // counters_ext/port_xmit_packets_64
    port_xmit_packets_64: Option<u64>,

    // counters/excessive_buffer_overrun_errors
    excessive_buffer_overrun_errors: Option<u64>,

    // counters/link_downed
    link_downed: Option<u64>,

    // counters/link_error_recovery
    link_error_recovery: Option<u64>,

    // counters/local_link_integrity_errors
    local_link_integrity_errors: Option<u64>,

    // counters/multicast_rcv_packets
    multicast_rcv_packets: Option<u64>,

    // counters/multicast_xmit_packets
    multicast_xmit_packets: Option<u64>,

    // counters/port_rcv_constraint_errors
    port_rcv_constraint_errors: Option<u64>,

    // counters/port_rcv_data
    port_rcv_data: Option<u64>,

    // counters/port_rcv_discards
    port_rcv_discards: Option<u64>,

    // counters/port_rcv_errors
    port_rcv_errors: Option<u64>,

    // counters/port_rcv_packets
    port_rcv_packets: Option<u64>,

    // counters/port_rcv_remote_physical_errors
    port_rcv_remote_physical_errors: Option<u64>,

    // counters/port_rcv_switch_relay_errors
    port_rcv_switch_relay_errors: Option<u64>,

    // counters/port_xmit_constraint_errors
    port_xmit_constraint_errors: Option<u64>,

    // counters/port_xmit_data
    port_xmit_data: Option<u64>,

    // counters/port_xmit_discards
    port_xmit_discards: Option<u64>,

    // counters/port_xmit_packets
    port_xmit_packets: Option<u64>,

    // counters/port_xmit_wait
    port_xmit_wait: Option<u64>,

    // counters/symbol_error
    symbol_error: Option<u64>,

    // counters/unicast_rcv_packets
    unicast_rcv_packets: Option<u64>,

    // counters/unicast_xmit_packets
    unicast_xmit_packets: Option<u64>,

    // counters/VL15_dropped
    vl15_dropped: Option<u64>,
}

/// InfiniBandPort contains info from files in
/// /sys/class/infiniband/<name>/ports/<port>
#[derive(Debug, Default, PartialEq)]
struct InfiniBandPort {
    name: String,
    port: u32,

    // String representation from /sys/class/infiniband/<name>/ports/<port>/state
    state: String,

    // ID from /sys/class/infiniband/<name>/ports/<port>/state
    state_id: u32,

    // String representation from /sys/class/infiniband/<name>/ports/<port>/phys_state
    phys_state: String,

    // ID from /sys/class/infiniband/<name>/ports/<port>/phys_state
    phys_state_id: u32,

    // in bytes/second from /sys/class/infiniband/<name>/ports/<port>/rate
    rate: u64,

    counters: InfiniBandCounters,
}

/// InfiniBandDevice contains info from files in /sys/class/infiniband for
/// a single InfiniBand device
#[derive(Debug, Default, PartialEq)]
struct InfiniBandDevice {
    name: String,

    // /sys/class/infiniband/<name>/board_id
    board_id: String,

    // /sys/class/infiniband/<name>/fw_ver
    fw_ver: String,

    // /sys/class/infiniband/<name>/hca_type
    hca_type: String,

    ports: Vec<InfiniBandPort>,
}

pub async fn gather(sys_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let devices = infiniband_class(sys_path.join("class/infiniband")).await?;

    let mut metrics = vec![];
    for device in devices {
        metrics.push(Metric::gauge_with_tags(
            "node_infiniband_info",
            "Non-numeric data from /sys/class/infiniband/<device>, value is always 1.",
            1,
            tags!(
                Key::from_static("device") => device.name.clone(),
                Key::from_static("board_id") => device.board_id,
                Key::from_static("firmware_version") => device.fw_ver,
                Key::from_static("hca_type") => device.hca_type
            ),
        ));

        for port in device.ports {
            let tags = tags!(
                Key::from_static("device") => device.name.clone(),
                Key::from_static("port") => port.port
            );

            metrics.extend([
                Metric::gauge_with_tags(
                    "node_infiniband_state_id",
                    "State of the InfiniBand port (0: no change, 1: down, 2: init, 3: armed, 4: active, 5: act defer)",
                    port.state_id,
                    tags.clone(),
                ),
                Metric::gauge_with_tags(
                    "node_infiniband_physical_state_id",
                    "Physical state of the InfiniBand port (0: no change, 1: sleep, 2: polling, 3: disable, 4: shift, 5: link up, 6: link error recover, 7: phytest)",
                    port.phys_state_id,
                    tags.clone(),
                ),
                Metric::gauge_with_tags(
                    "node_infiniband_rate_bytes_per_second",
                    "Maximum signal transfer rate",
                    port.rate,
                    tags.clone(),
                ),
            ]);

            if let Some(value) = port.counters.port_multicast_rcv_packets {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_legacy_multicast_packets_received_total",
                    "Number of multicast packets received",
                    value,
                    tags.clone(),
                ))
            }

            if let Some(value) = port.counters.port_multicast_xmit_packets {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_legacy_multicast_packets_transmitted_total",
                    "Number of multicast packets transmitted",
                    value,
                    tags.clone(),
                ))
            }

            if let Some(value) = port.counters.port_rcv_data_64 {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_legacy_data_received_bytes_total",
                    "Number of data octets received on all links",
                    value,
                    tags.clone(),
                ))
            }

            if let Some(value) = port.counters.port_rcv_packets_64 {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_legacy_packets_received_total",
                    "Number of data packets received on all links",
                    value,
                    tags.clone(),
                ))
            }

            if let Some(value) = port.counters.port_unicast_rcv_packets {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_legacy_unicast_packets_received_total",
                    "Number of unicast packets received",
                    value,
                    tags.clone(),
                ))
            }

            if let Some(v) = port.counters.port_unicast_xmit_packets {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_legacy_unicast_packets_transmitted_total",
                    "Number of unicast packets transmitted",
                    v,
                    tags.clone(),
                ))
            }

            if let Some(v) = port.counters.port_xmit_data_64 {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_legacy_data_transmitted_bytes_total",
                    "Number of data octets transmitted on all links",
                    v,
                    tags.clone(),
                ))
            }

            if let Some(v) = port.counters.port_xmit_packets_64 {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_legacy_packets_transmitted_total",
                    "Number of data packets received on all links",
                    v,
                    tags.clone(),
                ))
            }

            if let Some(v) = port.counters.excessive_buffer_overrun_errors {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_excessive_buffer_overrun_errors_total",
                    "Number of times that OverrunErrors consecutive flow control update periods occurred, each having at least one overrun error.",
                    v,
                    tags.clone(),
                ))
            }

            if let Some(v) = port.counters.link_downed {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_link_downed_total",
                    "Number of times the link failed to recover from an error state and went down",
                    v,
                    tags.clone(),
                ))
            }

            if let Some(v) = port.counters.link_error_recovery {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_link_error_recovery_total",
                    "Number of times the link successfully recovered from an error state",
                    v,
                    tags.clone(),
                ))
            }

            if let Some(v) = port.counters.local_link_integrity_errors {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_local_link_integrity_errors_total",
                    "Number of times that the count of local physical errors exceeded the threshold specified by LocalPhyErrors.",
                    v,
                    tags.clone(),
                ))
            }

            if let Some(v) = port.counters.multicast_rcv_packets {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_multicast_packets_received_total",
                    "Number of multicast packets received (including errors)",
                    v,
                    tags.clone(),
                ))
            }

            if let Some(v) = port.counters.multicast_xmit_packets {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_multicast_packets_transmitted_total",
                    "Number of multicast packets transmitted (including errors)",
                    v,
                    tags.clone(),
                ))
            }

            if let Some(v) = port.counters.port_rcv_errors {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_port_constraint_errors_received_total",
                    "Number of packets received on the switch physical port that are discarded",
                    v,
                    tags.clone(),
                ))
            }

            if let Some(v) = port.counters.port_xmit_constraint_errors {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_port_constraint_errors_transmitted_total",
                    "Number of packets not transmitted from the switch physical port",
                    v,
                    tags.clone(),
                ))
            }

            if let Some(v) = port.counters.port_rcv_data {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_port_data_received_bytes_total",
                    "Number of data octets received on all links",
                    v,
                    tags.clone(),
                ))
            }

            if let Some(v) = port.counters.port_xmit_data {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_port_data_transmitted_bytes_total",
                    "Number of data octets transmitted on all links",
                    v,
                    tags.clone(),
                ))
            }

            if let Some(v) = port.counters.port_rcv_discards {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_port_discards_received_total",
                    "Number of inbound packets discarded by the port because the port is down or congested",
                    v,
                    tags.clone(),
                ))
            }

            if let Some(value) = port.counters.port_xmit_discards {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_port_discards_transmitted_total",
                    "Number of outbound packets discarded by the port because the port is down or congested",
                    value,
                    tags.clone(),
                ))
            }

            if let Some(value) = port.counters.port_rcv_errors {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_port_errors_received_total",
                    "Number of packets containing an error that were received on this port",
                    value,
                    tags.clone(),
                ))
            }

            if let Some(value) = port.counters.port_rcv_packets {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_port_packets_received_total",
                    "Number of packets received on all VLs by this port (including errors)",
                    value,
                    tags.clone(),
                ))
            }

            if let Some(value) = port.counters.port_xmit_packets {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_port_packets_transmitted_total",
                    "Number of packets transmitted on all VLs from this port (including errors)",
                    value,
                    tags.clone(),
                ))
            }

            if let Some(value) = port.counters.port_xmit_wait {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_port_transmit_wait_total",
                    "Number of ticks during which the port had data to transmit but no data was sent during the entire tick",
                    value,
                    tags.clone(),
                ))
            }

            if let Some(value) = port.counters.unicast_rcv_packets {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_unicast_packets_received_total",
                    "Number of unicast packets received (including errors)",
                    value,
                    tags.clone(),
                ))
            }

            if let Some(value) = port.counters.unicast_xmit_packets {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_unicast_packets_transmitted_total",
                    "Number of unicast packets transmitted (including errors)",
                    value,
                    tags.clone(),
                ))
            }

            if let Some(value) = port.counters.port_rcv_remote_physical_errors {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_port_receive_remote_physical_errors_total",
                    "Number of packets marked with the EBP (End of Bad Packet) delimiter received on the port.",
                    value,
                    tags.clone(),
                ))
            }

            if let Some(value) = port.counters.port_rcv_switch_relay_errors {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_port_receive_switch_relay_errors_total",
                    "Number of packets that could not be forwarded by the switch.",
                    value,
                    tags.clone(),
                ))
            }

            if let Some(value) = port.counters.symbol_error {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_symbol_error_total",
                    "Number of minor link errors detected on one or more physical lanes.",
                    value,
                    tags.clone(),
                ))
            }

            if let Some(value) = port.counters.vl15_dropped {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_vl15_dropped_total",
                    "Number of incoming VL15 packets dropped due to resource limitations.",
                    value,
                    tags,
                ))
            }
        }
    }

    Ok(metrics)
}

async fn infiniband_class<P: AsRef<Path>>(root: P) -> Result<Vec<InfiniBandDevice>, Error> {
    let dirs = std::fs::read_dir(root)?;

    let mut devices = vec![];
    for entry in dirs.flatten() {
        let dev = parse_infiniband_device(entry.path()).await?;
        devices.push(dev);
    }

    Ok(devices)
}

async fn parse_infiniband_device(root: PathBuf) -> Result<InfiniBandDevice, Error> {
    let name = root.file_name().unwrap().to_str().unwrap();
    let mut device = InfiniBandDevice {
        name: name.to_string(),
        ..Default::default()
    };

    for sub in ["board_id", "fw_ver", "hca_type"] {
        let content = match read_string(root.join(sub)) {
            Ok(c) => c,
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    continue;
                }

                return Err(format!("failed to read file {name}, {err}").into());
            }
        };

        match sub {
            "board_id" => device.board_id = content,
            "fw_ver" => device.fw_ver = content,
            "hca_type" => device.hca_type = content,
            _ => {}
        }
    }

    let dirs = std::fs::read_dir(root.join("ports"))?;
    for entry in dirs.flatten() {
        let port = parse_infiniband_port(name, entry.path()).await?;
        device.ports.push(port);
    }

    Ok(device)
}

/// parse_infiniband_port scans predefined files in /sys/class/infiniband/<device>/ports/<port>
/// directory and gets their contents
async fn parse_infiniband_port(name: &str, root: PathBuf) -> Result<InfiniBandPort, Error> {
    let port = root
        .file_name()
        .expect("filename should present in path")
        .to_string_lossy()
        .parse::<u32>()?;
    let mut ibp = InfiniBandPort {
        port,
        name: name.to_string(),
        ..Default::default()
    };

    let content = read_string(root.join("state"))?;
    let (id, name) = parse_state(&content)?;
    ibp.state = name;
    ibp.state_id = id;

    let content = read_string(root.join("phys_state"))?;
    let (id, name) = parse_state(&content)?;
    ibp.phys_state = name;
    ibp.phys_state_id = id;

    let content = read_string(root.join("rate"))?;
    let rate = parse_rate(&content)?;
    ibp.rate = rate;

    let counters = parse_infiniband_counters(root).await?;
    ibp.counters = counters;

    Ok(ibp)
}

// Parse InfiniBand state. Expected format: "<id>: <string-representation>"
fn parse_state(s: &str) -> Result<(u32, String), Error> {
    let parts = s.split(':').map(|p| p.trim()).collect::<Vec<_>>();
    if parts.len() != 2 {
        return Err(format!("failed to split {s} into 'ID: Name'").into());
    }

    let name = parts[1].to_string();
    let id = parts[0].parse()?;

    Ok((id, name))
}

// Parse rate (example: "100 Gb/sec (4X EDR)") and return it as bytes/second
fn parse_rate(s: &str) -> Result<u64, Error> {
    let parts = s.splitn(2, ' ').collect::<Vec<_>>();
    if parts.len() != 2 {
        return Err(format!("failed to split {s}").into());
    }

    let v = parts[0].parse::<f64>()?;
    // convert Gb/s into bytes/s
    let rate = v * 125000000.0;

    Ok(rate as u64)
}

async fn parse_infiniband_counters(root: PathBuf) -> Result<InfiniBandCounters, Error> {
    let dirs = std::fs::read_dir(root.join("counters"))?;

    let mut counters = InfiniBandCounters::default();
    for entry in dirs.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_str().unwrap();

        let content = match read_string(path) {
            Ok(c) => c,
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    continue;
                }

                return Err(format!("failed to read file {name}, {err}").into());
            }
        };

        // Ugly workaround for handling https://github.com/prometheus/node_exporter/issues/966
        // when counters are `N/A (not available)`.
        // This was already patched and submitted, see
        // https://www.spinics.net/lists/linux-rdma/msg68596.html
        // Remove this as soon as the fix lands in the enterprise distros.
        if content.contains("N/A (no PMA)") {
            continue;
        }

        // According to Mellanox, the metrics port_rcv_ata, port_xmit_data,
        // port_rcv_data_64, and port_xmit_data_64 "are devided by 4 unconditionally"
        // as they represent the amount of data being transmitted and received per lane.
        // Mellanox cards have 4 lanes per port, so all values must be multiplied by 4
        // to get the expected value.
        let value = content.parse::<u64>().ok();
        match name {
            "excessive_buffer_overrun_errors" => counters.excessive_buffer_overrun_errors = value,
            "link_downed" => counters.link_downed = value,
            "link_error_recovery" => counters.link_error_recovery = value,
            "local_link_integrity_errors" => counters.local_link_integrity_errors = value,
            "multicast_rcv_packets" => counters.multicast_rcv_packets = value,
            "multicast_xmit_packets" => counters.multicast_xmit_packets = value,
            "port_rcv_constraint_errors" => counters.port_rcv_constraint_errors = value,
            "port_rcv_data" => counters.port_rcv_data = value.map(|v| v * 4),
            "port_rcv_discards" => counters.port_rcv_discards = value,
            "port_rcv_errors" => counters.port_rcv_errors = value,
            "port_rcv_packets" => counters.port_rcv_packets = value,
            "port_rcv_remote_physical_errors" => counters.port_rcv_remote_physical_errors = value,
            "port_rcv_switch_relay_errors" => counters.port_rcv_switch_relay_errors = value,
            "port_xmit_constraint_errors" => counters.port_xmit_constraint_errors = value,
            "port_xmit_data" => counters.port_xmit_data = value.map(|v| v * 4),
            "port_xmit_discards" => counters.port_xmit_discards = value,
            "port_xmit_packets" => counters.port_xmit_packets = value,
            "port_xmit_wait" => counters.port_xmit_wait = value,
            "symbol_error" => counters.symbol_error = value,
            "unicast_rcv_packets" => counters.unicast_rcv_packets = value,
            "unicast_xmit_packets" => counters.unicast_xmit_packets = value,
            "VL15_dropped" => counters.vl15_dropped = value,
            _ => {}
        }
    }

    // Parse legacy counters
    match std::fs::read_dir(root.join("counters_ext")) {
        Ok(dirs) => {
            for entry in dirs.flatten() {
                let name = entry.file_name();

                let content = match read_string(entry.path()) {
                    Ok(c) => c,
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(format!(
                            "failed to read file {}, err: {}",
                            name.to_string_lossy(),
                            err
                        )
                        .into());
                    }
                };

                // Ugly workaround for handling https://github.com/prometheus/node_exporter/issues/966
                // when counters are `N/A (not available)`.
                // This was already patched and submitted, see
                // https://www.spinics.net/lists/linux-rdma/msg68596.html
                // Remove this as soon as the fix lands in the enterprise distros.
                if content.contains("N/A (no PMA)") {
                    continue;
                }

                match name.to_string_lossy().as_ref() {
                    "port_multicast_rcv_packets" => {
                        counters.port_multicast_rcv_packets = content.parse::<u64>().ok()
                    }
                    "port_multicast_xmit_packets" => {
                        counters.port_multicast_xmit_packets = content.parse::<u64>().ok()
                    }
                    "port_rcv_data_64" => {
                        counters.port_rcv_data_64 = content.parse::<u64>().ok().map(|v| v * 4)
                    }
                    "port_rcv_packets_64" => {
                        counters.port_rcv_packets_64 = content.parse::<u64>().ok()
                    }
                    "port_unicast_rcv_packets" => {
                        counters.port_unicast_rcv_packets = content.parse::<u64>().ok()
                    }
                    "port_unicast_xmit_packets" => {
                        counters.port_unicast_xmit_packets = content.parse::<u64>().ok()
                    }
                    "port_xmit_data_64" => {
                        counters.port_xmit_data_64 = content.parse::<u64>().ok().map(|v| v * 4)
                    }
                    "port_xmit_packets_64" => {
                        counters.port_xmit_packets_64 = content.parse::<u64>().ok()
                    }
                    _ => {}
                }
            }
        }
        Err(err) => {
            if err.kind() != std::io::ErrorKind::NotFound {
                return Err(err.into());
            }
        }
    };

    Ok(counters)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_slow_rate() {
        struct Case {
            rate: String,
            want: u64,
        }

        let cases = vec![
            Case {
                rate: "0 GB/sec".to_string(),
                want: 0,
            },
            Case {
                rate: "2.5 Gb/sec (1X SDR)".to_string(),
                want: 312500000,
            },
            Case {
                rate: "500 Gb/sec (4X HDR)".to_string(),
                want: 62500000000,
            },
        ];

        for case in cases {
            let v = parse_rate(&case.rate).unwrap();
            assert_eq!(v, case.want)
        }
    }

    #[tokio::test]
    async fn test_infiniband_class() {
        let root = PathBuf::from("tests/node/sys/class/infiniband");
        let mut devs = infiniband_class(root).await.unwrap();

        // The readdir_r is not guaranteed to return in any specific order.
        // And the order of Github CI and Centos Stream is different, so it must be sorted
        // See: https://utcc.utoronto.ca/~cks/space/blog/unix/ReaddirOrder
        devs.sort_by(|a, b| a.name.cmp(&b.name));
        devs.iter_mut()
            .for_each(|dev| dev.ports.sort_by(|a, b| a.port.cmp(&b.port)));

        assert_eq!(
            devs[0],
            InfiniBandDevice {
                name: "hfi1_0".to_string(),
                board_id:
                    "HPE 100Gb 1-port OP101 QSFP28 x16 PCIe Gen3 with Intel Omni-Path Adapter"
                        .to_string(),
                fw_ver: "1.27.0".to_string(),
                hca_type: "".to_string(),
                ports: vec![InfiniBandPort {
                    name: "hfi1_0".to_string(),
                    port: 1,
                    state: "ACTIVE".to_string(),
                    state_id: 4,
                    phys_state: "LinkUp".to_string(),
                    phys_state_id: 5,
                    rate: 12500000000,
                    counters: InfiniBandCounters {
                        port_multicast_rcv_packets: None,
                        port_multicast_xmit_packets: None,
                        port_rcv_data_64: None,
                        port_rcv_packets_64: None,
                        port_unicast_rcv_packets: None,
                        port_unicast_xmit_packets: None,
                        port_xmit_data_64: None,
                        port_xmit_packets_64: None,
                        excessive_buffer_overrun_errors: Some(0),
                        link_downed: Some(0),
                        link_error_recovery: Some(0),
                        local_link_integrity_errors: Some(0),
                        multicast_rcv_packets: None,
                        multicast_xmit_packets: None,
                        port_rcv_constraint_errors: Some(0),
                        port_rcv_data: Some(1380366808104),
                        port_rcv_discards: None,
                        port_rcv_errors: Some(0),
                        port_rcv_packets: Some(638036947),
                        port_rcv_remote_physical_errors: Some(0),
                        port_rcv_switch_relay_errors: Some(0),
                        port_xmit_constraint_errors: Some(0),
                        port_xmit_data: Some(1094233306172),
                        port_xmit_discards: Some(0),
                        port_xmit_packets: Some(568318856),
                        port_xmit_wait: Some(0),
                        symbol_error: Some(0),
                        unicast_rcv_packets: None,
                        unicast_xmit_packets: None,
                        vl15_dropped: Some(0),
                    },
                }],
            }
        );

        assert_eq!(
            devs[1],
            InfiniBandDevice {
                name: "mlx4_0".to_string(),
                board_id: "SM_1141000001000".to_string(),
                fw_ver: "2.31.5050".to_string(),
                hca_type: "MT4099".to_string(),
                ports: vec![
                    InfiniBandPort {
                        name: "mlx4_0".to_string(),
                        port: 1,
                        state: "ACTIVE".to_string(),
                        state_id: 4,
                        phys_state: "LinkUp".to_string(),
                        phys_state_id: 5,
                        rate: 5000000000,
                        counters: InfiniBandCounters {
                            port_multicast_rcv_packets: None,
                            port_multicast_xmit_packets: None,
                            port_rcv_data_64: None,
                            port_rcv_packets_64: None,
                            port_unicast_rcv_packets: None,
                            port_unicast_xmit_packets: None,
                            port_xmit_data_64: None,
                            port_xmit_packets_64: None,
                            excessive_buffer_overrun_errors: Some(0),
                            link_downed: Some(0),
                            link_error_recovery: Some(0),
                            local_link_integrity_errors: Some(0),
                            multicast_rcv_packets: None,
                            multicast_xmit_packets: None,
                            port_rcv_constraint_errors: Some(0),
                            port_rcv_data: Some(8884894436),
                            port_rcv_discards: None,
                            port_rcv_errors: Some(0),
                            port_rcv_packets: Some(87169372),
                            port_rcv_remote_physical_errors: Some(0),
                            port_rcv_switch_relay_errors: Some(0),
                            port_xmit_constraint_errors: Some(0),
                            port_xmit_data: Some(106036453180),
                            port_xmit_discards: Some(0),
                            port_xmit_packets: Some(85734114),
                            port_xmit_wait: Some(3599),
                            symbol_error: Some(0),
                            unicast_rcv_packets: None,
                            unicast_xmit_packets: None,
                            vl15_dropped: Some(0),
                        },
                    },
                    InfiniBandPort {
                        name: "mlx4_0".to_string(),
                        port: 2,
                        state: "ACTIVE".to_string(),
                        state_id: 4,
                        phys_state: "LinkUp".to_string(),
                        phys_state_id: 5,
                        rate: 5000000000,
                        counters: InfiniBandCounters {
                            port_multicast_rcv_packets: None,
                            port_multicast_xmit_packets: None,
                            port_rcv_data_64: None,
                            port_rcv_packets_64: None,
                            port_unicast_rcv_packets: None,
                            port_unicast_xmit_packets: None,
                            port_xmit_data_64: None,
                            port_xmit_packets_64: None,
                            excessive_buffer_overrun_errors: Some(0),
                            link_downed: Some(0),
                            link_error_recovery: Some(0),
                            local_link_integrity_errors: Some(0),
                            multicast_rcv_packets: None,
                            multicast_xmit_packets: None,
                            port_rcv_constraint_errors: Some(0),
                            port_rcv_data: Some(9841747136),
                            port_rcv_discards: None,
                            port_rcv_errors: Some(0),
                            port_rcv_packets: Some(89332064),
                            port_rcv_remote_physical_errors: Some(0),
                            port_rcv_switch_relay_errors: Some(0),
                            port_xmit_constraint_errors: Some(0),
                            port_xmit_data: Some(106161427560),
                            port_xmit_discards: Some(0),
                            port_xmit_packets: Some(88622850),
                            port_xmit_wait: Some(3846),
                            symbol_error: Some(0),
                            unicast_rcv_packets: None,
                            unicast_xmit_packets: None,
                            vl15_dropped: Some(0),
                        },
                    },
                ],
            }
        )
    }
}
