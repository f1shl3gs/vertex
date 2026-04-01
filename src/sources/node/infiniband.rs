#![deny(dead_code)]

use std::io::ErrorKind;
use std::path::PathBuf;

use event::{Metric, tags};

use super::{Error, Paths, read_sys_file};

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

/// [`InfiniBandHwCounters`] contains counter value from files in
/// /sys/class/infiniband/<Name>/ports/<Port>/hw_counters
///for a single port of one InfiniBand device
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Default)]
struct InfiniBandHwCounters {
    duplicate_request: Option<u64>,     // hw_counters/duplicate_request
    implied_nak_seq_err: Option<u64>,   // hw_counters/implied_nak_seq_err
    lifespan: Option<u64>,              // hw_counters/lifespan
    local_ack_timeout_err: Option<u64>, // hw_counters/local_ack_timeout_err
    np_cnp_sent: Option<u64>,           // hw_counters/np_cnp_sent
    np_ecn_marked_roce_packets: Option<u64>, // hw_counters/np_ecn_marked_roce_packets
    out_of_buffer: Option<u64>,         // hw_counters/out_of_buffer
    out_of_sequence: Option<u64>,       // hw_counters/out_of_sequence
    packet_seq_err: Option<u64>,        // hw_counters/packet_seq_err
    req_cqe_error: Option<u64>,         // hw_counters/req_cqe_error
    req_cqe_flush_error: Option<u64>,   // hw_counters/req_cqe_flush_error
    req_remote_access_errors: Option<u64>, // hw_counters/req_remote_access_errors
    req_remote_invalid_request: Option<u64>, // hw_counters/req_remote_invalid_request
    resp_cqe_error: Option<u64>,        // hw_counters/resp_cqe_error
    resp_cqe_flush_error: Option<u64>,  // hw_counters/resp_cqe_flush_error
    resp_local_length_error: Option<u64>, // hw_counters/resp_local_length_error
    resp_remote_access_errors: Option<u64>, // hw_counters/resp_remote_access_errors
    rnr_nak_retry_err: Option<u64>,     // hw_counters/rnr_nak_retry_err
    roce_adp_retrans: Option<u64>,      // hw_counters/roce_adp_retrans
    roce_adp_retrans_to: Option<u64>,   // hw_counters/roce_adp_retrans_to
    roce_slow_restart: Option<u64>,     // hw_counters/roce_slow_restart
    roce_slow_restart_cnps: Option<u64>, // hw_counters/roce_slow_restart_cnps
    roce_slow_restart_trans: Option<u64>, // hw_counters/roce_slow_restart_trans
    rp_cnp_handled: Option<u64>,        // hw_counters/rp_cnp_handled
    rp_cnp_ignored: Option<u64>,        // hw_counters/rp_cnp_ignored
    rx_atomic_requests: Option<u64>,    // hw_counters/rx_atomic_requests
    rx_dct_connect: Option<u64>,        // hw_counters/rx_dct_connect
    rx_icrc_encapsulated: Option<u64>,  // hw_counters/rx_icrc_encapsulated
    rx_read_requests: Option<u64>,      // hw_counters/rx_read_requests
    rx_write_requests: Option<u64>,     // hw_counters/rx_write_requests
}

fn load_infiniband_hw_counters(root: PathBuf) -> Result<InfiniBandHwCounters, Error> {
    let mut counters = InfiniBandHwCounters::default();

    let mut path = root.join("d");
    for (filename, dst) in [
        ("duplicate_request", &mut counters.duplicate_request),
        ("implied_nak_seq_err", &mut counters.implied_nak_seq_err),
        ("lifespan", &mut counters.lifespan),
        ("local_ack_timeout_err", &mut counters.local_ack_timeout_err),
        ("np_cnp_sent", &mut counters.np_cnp_sent),
        (
            "np_ecn_marked_roce_packets",
            &mut counters.np_ecn_marked_roce_packets,
        ),
        ("out_of_buffer", &mut counters.out_of_buffer),
        ("out_of_sequence", &mut counters.out_of_sequence),
        ("packet_seq_err", &mut counters.packet_seq_err),
        ("req_cqe_error", &mut counters.req_cqe_error),
        ("req_cqe_flush_error", &mut counters.req_cqe_flush_error),
        (
            "req_remote_access_errors",
            &mut counters.req_remote_access_errors,
        ),
        (
            "req_remote_invalid_request",
            &mut counters.req_remote_invalid_request,
        ),
        ("resp_cqe_error", &mut counters.resp_cqe_error),
        ("resp_cqe_flush_error", &mut counters.resp_cqe_flush_error),
        (
            "resp_local_length_error",
            &mut counters.resp_local_length_error,
        ),
        (
            "resp_remote_access_errors",
            &mut counters.resp_remote_access_errors,
        ),
        ("rnr_nak_retry_err", &mut counters.rnr_nak_retry_err),
        ("roce_adp_retrans", &mut counters.roce_adp_retrans),
        ("roce_adp_retrans_to", &mut counters.roce_adp_retrans_to),
        ("roce_slow_restart", &mut counters.roce_slow_restart),
        (
            "roce_slow_restart_cnps",
            &mut counters.roce_slow_restart_cnps,
        ),
        (
            "roce_slow_restart_trans",
            &mut counters.roce_slow_restart_trans,
        ),
        ("rp_cnp_handled", &mut counters.rp_cnp_handled),
        ("rp_cnp_ignored", &mut counters.rp_cnp_ignored),
        ("rx_atomic_requests", &mut counters.rx_atomic_requests),
        ("rx_dct_connect", &mut counters.rx_dct_connect),
        ("rx_icrc_encapsulated", &mut counters.rx_icrc_encapsulated),
        ("rx_read_requests", &mut counters.rx_read_requests),
        ("rx_write_requests", &mut counters.rx_write_requests),
    ] {
        path.set_file_name(filename);
        let Ok(content) = read_sys_file(&path) else {
            continue;
        };

        if let Ok(value) = content.parse::<u64>() {
            *dst = Some(value);
        }
    }

    Ok(counters)
}

/// InfiniBandPort contains info from files in
/// /sys/class/infiniband/<name>/ports/<port>
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Default)]
struct InfiniBandPort {
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
    hw_counters: Option<InfiniBandHwCounters>,
}

/// InfiniBandDevice contains info from files in /sys/class/infiniband for
/// a single InfiniBand device
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Default)]
struct InfiniBandDevice {
    // name: String,

    // /sys/class/infiniband/<name>/board_id
    board_id: String,

    // /sys/class/infiniband/<name>/fw_ver
    fw_ver: String,

    // /sys/class/infiniband/<name>/hca_type
    hca_type: String,

    ports: Vec<InfiniBandPort>,
}

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let root = paths.sys().join("class/infiniband");
    let mut metrics = Vec::new();
    for entry in root.read_dir()?.flatten() {
        let Ok(typ) = entry.file_type() else { continue };
        if !typ.is_dir() {
            continue;
        }

        let filename = entry.file_name();
        let device = filename.to_string_lossy();

        let stats = parse_infiniband_device(entry.path())?;
        metrics.push(Metric::gauge_with_tags(
            "node_infiniband_info",
            "Non-numeric data from /sys/class/infiniband/<device>, value is always 1.",
            1,
            tags!(
                "device" => device.as_ref(),
                "board_id" => stats.board_id,
                "firmware_version" => stats.fw_ver,
                "hca_type" => stats.hca_type
            ),
        ));

        for port in stats.ports {
            let tags = tags!(
                "device" => device.as_ref(),
                "port" => port.port
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
            if let Some(value) = port.counters.port_unicast_xmit_packets {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_legacy_unicast_packets_transmitted_total",
                    "Number of unicast packets transmitted",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = port.counters.port_xmit_data_64 {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_legacy_data_transmitted_bytes_total",
                    "Number of data octets transmitted on all links",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = port.counters.port_xmit_packets_64 {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_legacy_packets_transmitted_total",
                    "Number of data packets received on all links",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = port.counters.excessive_buffer_overrun_errors {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_excessive_buffer_overrun_errors_total",
                    "Number of times that OverrunErrors consecutive flow control update periods occurred, each having at least one overrun error.",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = port.counters.link_downed {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_link_downed_total",
                    "Number of times the link failed to recover from an error state and went down",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = port.counters.link_error_recovery {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_link_error_recovery_total",
                    "Number of times the link successfully recovered from an error state",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = port.counters.local_link_integrity_errors {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_local_link_integrity_errors_total",
                    "Number of times that the count of local physical errors exceeded the threshold specified by LocalPhyErrors.",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = port.counters.multicast_rcv_packets {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_multicast_packets_received_total",
                    "Number of multicast packets received (including errors)",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = port.counters.multicast_xmit_packets {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_multicast_packets_transmitted_total",
                    "Number of multicast packets transmitted (including errors)",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = port.counters.port_rcv_errors {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_port_constraint_errors_received_total",
                    "Number of packets received on the switch physical port that are discarded",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = port.counters.port_xmit_constraint_errors {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_port_constraint_errors_transmitted_total",
                    "Number of packets not transmitted from the switch physical port",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = port.counters.port_rcv_data {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_port_data_received_bytes_total",
                    "Number of data octets received on all links",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = port.counters.port_xmit_data {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_port_data_transmitted_bytes_total",
                    "Number of data octets transmitted on all links",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = port.counters.port_rcv_discards {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_port_discards_received_total",
                    "Number of inbound packets discarded by the port because the port is down or congested",
                    value,
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
                    tags.clone(),
                ))
            }

            let Some(counters) = port.hw_counters else {
                continue;
            };
            if let Some(value) = counters.lifespan {
                metrics.push(Metric::gauge_with_tags(
                    "node_infiniband_lifespan_seconds",
                    "The maximum period in ms which defines the aging of the counter reads. Two consecutive reads within this period might return the same values.",
                    value / 1000,
                    tags.clone(),
                ));
            }
            if let Some(value) = counters.duplicate_request {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_duplicate_requests_packets_total",
                    "The number of received packets. A duplicate request is a request that had been previously executed.",
                    value,
                    tags.clone(),
                ));
            }
            if let Some(value) = counters.implied_nak_seq_err {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_implied_nak_seq_errors_total",
                    "The number of time the requested decided an ACK. with a PSN larger than the expected PSN for an RDMA read or response.",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = counters.local_ack_timeout_err {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_local_ack_timeout_errors_total",
                    "The number of times QP's ack timer expired for RC, XRC, DCT QPs at the sender side. The QP retry limit was not exceed, therefore it is still recoverable error.",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = counters.np_cnp_sent {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_np_cnp_packets_sent_total",
                    "The number of CNP packets sent by the Notification Point when it noticed congestion experienced in the RoCEv2 IP header (ECN bits). The counters was added in MLNX_OFED 4.1",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = counters.np_ecn_marked_roce_packets {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_np_ecn_marked_roce_packets_received_total",
                    "The number of RoCEv2 packets received by the notification point which were marked for experiencing the congestion (ECN bits where '11' on the ingress RoCE traffic) . The counters was added in MLNX_OFED 4.1",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = counters.out_of_buffer {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_out_of_buffer_drops_total",
                    "The number of drops occurred due to lack of WQE for the associated QPs.",
                    value,
                    tags.clone(),
                ));
            }
            if let Some(value) = counters.out_of_sequence {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_out_of_sequence_packets_received_total",
                    "The number of out of sequence packets received.",
                    value,
                    tags.clone(),
                ));
            }
            if let Some(value) = counters.packet_seq_err {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_packet_sequence_errors_total",
                    "The number of received NAK sequence error packets. The QP retry limit was not exceeded.",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = counters.req_cqe_error {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_req_cqes_errors_total",
                    "The number of times requester detected CQEs completed with errors. The counters was added in MLNX_OFED 4.1",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = counters.req_cqe_flush_error {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_req_cqes_flush_errors_total",
                    "The number of times requester detected CQEs completed with flushed errors. The counters was added in MLNX_OFED 4.1",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = counters.req_remote_access_errors {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_req_remote_access_errors_total",
                    "The number of times requester detected remote access errors. The counters was added in MLNX_OFED 4.1",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = counters.req_remote_invalid_request {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_req_remote_invalid_request_errors_total",
                    "The number of times requester detected remote invalid request errors. The counters was added in MLNX_OFED 4.1",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = counters.resp_cqe_error {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_resp_cqes_errors_total",
                    "The number of times responder detected CQEs completed with errors. The counters was added in MLNX_OFED 4.1",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = counters.resp_cqe_flush_error {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_resp_cqes_flush_errors_total",
                    "The number of times responder detected CQEs completed with flushed errors. The counters was added in MLNX_OFED 4.1",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = counters.resp_local_length_error {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_resp_local_length_errors_total",
                    "The number of times responder detected local length errors. The counters was added in MLNX_OFED 4.1",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = counters.resp_remote_access_errors {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_resp_remote_access_errors_total",
                    "The number of times responder detected remote access errors. The counters was added in MLNX_OFED 4.1",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = counters.rnr_nak_retry_err {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_rnr_nak_retry_packets_received_total",
                    "The number of received RNR NAK packets. The QP retry limit was not exceeded.",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = counters.roce_adp_retrans {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_roce_adp_retransmits_total",
                    "The number of adaptive retransmissions for RoCE traffic. The counter was added in MLNX_OFED rev 5.0-1.0.0.0 and kernel v5.6.0",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = counters.roce_adp_retrans_to {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_roce_adp_retransmits_timeout_total",
                    "The number of times RoCE traffic reached timeout due to adaptive retransmission. The counter was added in MLNX_OFED rev 5.0-1.0.0.0 and kernel v5.6.0",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = counters.roce_slow_restart {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_roce_slow_restart_used_total",
                    "The number of times RoCE slow restart was used. The counter was added in MLNX_OFED rev 5.0-1.0.0.0 and kernel v5.6.0",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = counters.roce_slow_restart_cnps {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_roce_slow_restart_cnps_total",
                    "The number of times RoCE slow restart generated CNP packets. The counter was added in MLNX_OFED rev 5.0-1.0.0.0 and kernel v5.6.0",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = counters.roce_slow_restart_trans {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_roce_slow_restart_total",
                    "The number of times RoCE slow restart changed state to slow restart. The counter was added in MLNX_OFED rev 5.0-1.0.0.0 and kernel v5.6.0",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = counters.rp_cnp_handled {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_rp_cnp_packets_handled_total",
                    "The number of CNP packets handled by the Reaction Point HCA to throttle the transmission rate. The counters was added in MLNX_OFED 4.1",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = counters.rp_cnp_ignored {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_rp_cnp_ignored_packets_received_total",
                    "The number of CNP packets received and ignored by the Reaction Point HCA. This counter should not raise if RoCE Congestion Control was enabled in the network. If this counter raise, verify that ECN was enabled on the adapter.",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = counters.rx_atomic_requests {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_rx_atomic_requests_total",
                    "The number of received ATOMIC request for the associated QPs.",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = counters.rx_dct_connect {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_rx_dct_connect_requests_total",
                    "The number of received connection requests for the associated DCTs.",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = counters.rx_read_requests {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_rx_read_requests_total",
                    "The number of received READ requests for the associated QPs.",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = counters.rx_write_requests {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_rx_write_requests_total",
                    "The number of received WRITE requests for the associated QPs.",
                    value,
                    tags.clone(),
                ))
            }
            if let Some(value) = counters.rx_icrc_encapsulated {
                metrics.push(Metric::sum_with_tags(
                    "node_infiniband_rx_icrc_encapsulated_errors_total",
                    "The number of RoCE packets with ICRC errors. This counter was added in MLNX_OFED 4.4 and kernel 4.19",
                    value,
                    tags,
                ))
            }
        }
    }

    Ok(metrics)
}

fn parse_infiniband_device(root: PathBuf) -> Result<InfiniBandDevice, Error> {
    let mut device = InfiniBandDevice::default();

    for sub in ["board_id", "fw_ver", "hca_type"] {
        match read_sys_file(root.join(sub)) {
            Ok(content) => match sub {
                "board_id" => device.board_id = content,
                "fw_ver" => device.fw_ver = content,
                "hca_type" => device.hca_type = content,
                _ => {}
            },
            Err(err) => {
                if err.kind() == ErrorKind::NotFound {
                    continue;
                }

                return Err(err.into());
            }
        }
    }

    // /sys/class/infiniband/<name>/ports
    for entry in root.join("ports").read_dir()?.flatten() {
        let Ok(typ) = entry.file_type() else { continue };
        if !typ.is_dir() {
            continue;
        }

        let filename = entry.file_name();
        let port = filename.to_string_lossy();
        let Ok(port) = port.parse::<u32>() else {
            continue;
        };

        let mut port_stats = parse_infiniband_port(entry.path())?;
        port_stats.port = port;

        device.ports.push(port_stats);
    }

    Ok(device)
}

/// parse_infiniband_port scans predefined files in /sys/class/infiniband/<device>/ports/<port>
/// directory and gets their contents
fn parse_infiniband_port(root: PathBuf) -> Result<InfiniBandPort, Error> {
    let mut ibp = InfiniBandPort::default();

    let content = read_sys_file(root.join("state"))?;
    let (id, name) = parse_state(&content)?;
    ibp.state = name.to_string();
    ibp.state_id = id;

    let content = read_sys_file(root.join("phys_state"))?;
    let (id, name) = parse_state(&content)?;
    ibp.phys_state = name.to_string();
    ibp.phys_state_id = id;

    let content = read_sys_file(root.join("rate"))?;
    ibp.rate = parse_rate(&content)?;

    ibp.counters = parse_infiniband_counters(root.join("counters"))?;

    let path = root.join("hw_counters");
    if path.is_dir() {
        ibp.hw_counters = Some(load_infiniband_hw_counters(path)?);
    }

    Ok(ibp)
}

// Parse InfiniBand state. Expected format: "<id>: <string-representation>"
fn parse_state(content: &str) -> Result<(u32, &str), Error> {
    let Some((first, second)) = content.split_once(':') else {
        return Err(Error::Malformed("infiniband state"));
    };

    let id = first.parse::<u32>()?;

    Ok((id, second.trim()))
}

// Parse rate (example: "100 Gb/sec (4X EDR)") and return it as bytes/second
fn parse_rate(content: &str) -> Result<u64, Error> {
    let Some((value, _)) = content.split_once(' ') else {
        return Err(Error::Malformed("infiniband rate"));
    };

    let rate = value.parse::<f64>()?;

    // convert Gb/s into bytes/s
    Ok((rate * 125000000.0) as u64)
}

fn parse_infiniband_counters(root: PathBuf) -> Result<InfiniBandCounters, Error> {
    let mut counters = InfiniBandCounters::default();
    for entry in root.read_dir()?.flatten() {
        let Ok(typ) = entry.file_type() else { continue };
        if !typ.is_file() {
            continue;
        }

        let value = match read_sys_file(entry.path()) {
            Ok(content) => {
                // Ugly workaround for handling https://github.com/prometheus/node_exporter/issues/966
                // when counters are `N/A (not available)`.
                // This was already patched and submitted, see
                // https://www.spinics.net/lists/linux-rdma/msg68596.html
                // Remove this as soon as the fix lands in the enterprise distros.
                if content.contains("N/A (no PMA)") {
                    continue;
                }

                content.parse::<u64>().ok()
            }
            Err(err) => {
                if err.kind() == ErrorKind::NotFound
                    || err.kind() == ErrorKind::PermissionDenied
                    || err.kind() == ErrorKind::InvalidData
                {
                    continue;
                }

                return Err(err.into());
            }
        };

        // According to Mellanox, the metrics port_rcv_ata, port_xmit_data,
        // port_rcv_data_64, and port_xmit_data_64 "are devided by 4 unconditionally"
        // as they represent the amount of data being transmitted and received per lane.
        // Mellanox cards have 4 lanes per port, so all values must be multiplied by 4
        // to get the expected value.
        match entry.file_name().to_string_lossy().as_ref() {
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
    match root.join("counters_ext").read_dir() {
        Ok(dirs) => {
            for entry in dirs.flatten() {
                let name = entry.file_name();
                let content = match read_sys_file(entry.path()) {
                    Ok(content) => {
                        // Ugly workaround for handling https://github.com/prometheus/node_exporter/issues/966
                        // when counters are `N/A (not available)`.
                        // This was already patched and submitted, see
                        // https://www.spinics.net/lists/linux-rdma/msg68596.html
                        // Remove this as soon as the fix lands in the enterprise distros.
                        if content.contains("N/A (no PMA)") {
                            continue;
                        }

                        content
                    }
                    Err(err) => {
                        if err.kind() == ErrorKind::NotFound
                            || err.kind() == ErrorKind::PermissionDenied
                            || err.kind() == ErrorKind::InvalidData
                        {
                            continue;
                        }

                        return Err(err.into());
                    }
                };

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
            if err.kind() != ErrorKind::NotFound {
                return Err(err.into());
            }
        }
    };

    Ok(counters)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn smoke() {
        let paths = Paths::test();
        let metrics = collect(paths).await.unwrap();
        assert!(!metrics.is_empty());
    }

    #[test]
    fn rate() {
        for (input, want) in [
            ("0 GB/sec", 0),
            ("2.5 Gb/sec (1X SDR)", 312500000),
            ("500 Gb/sec (4X HDR)", 62500000000),
        ] {
            let got = parse_rate(input).unwrap();
            assert_eq!(got, want)
        }
    }

    #[test]
    fn device() {
        let root = PathBuf::from("tests/node/fixtures/sys/class/infiniband");
        let got = parse_infiniband_device(root.join("i40iw0")).unwrap();

        assert_eq!(
            got,
            InfiniBandDevice {
                board_id: "I40IW Board ID".to_string(),
                fw_ver: "0.2".to_string(),
                hca_type: "I40IW".to_string(),
                ports: vec![InfiniBandPort {
                    port: 1,
                    state: "ACTIVE".to_string(),
                    state_id: 4,
                    phys_state: "LinkUp".to_string(),
                    phys_state_id: 5,
                    rate: 1250000000,
                    counters: InfiniBandCounters {
                        port_multicast_rcv_packets: None,
                        port_multicast_xmit_packets: None,
                        port_rcv_data_64: None,
                        port_rcv_packets_64: None,
                        port_unicast_rcv_packets: None,
                        port_unicast_xmit_packets: None,
                        port_xmit_data_64: None,
                        port_xmit_packets_64: None,
                        excessive_buffer_overrun_errors: None,
                        link_downed: None,
                        link_error_recovery: None,
                        local_link_integrity_errors: None,
                        multicast_rcv_packets: None,
                        multicast_xmit_packets: None,
                        port_rcv_constraint_errors: None,
                        port_rcv_data: None,
                        port_rcv_discards: None,
                        port_rcv_errors: None,
                        port_rcv_packets: None,
                        port_rcv_remote_physical_errors: None,
                        port_rcv_switch_relay_errors: None,
                        port_xmit_constraint_errors: None,
                        port_xmit_data: None,
                        port_xmit_discards: None,
                        port_xmit_packets: None,
                        port_xmit_wait: None,
                        symbol_error: None,
                        unicast_rcv_packets: None,
                        unicast_xmit_packets: None,
                        vl15_dropped: None,
                    },
                    hw_counters: None,
                },],
            }
        );
    }

    #[test]
    fn infiniband_devices() {
        let root = PathBuf::from("tests/node/fixtures/sys/class/infiniband");

        let got = parse_infiniband_device(root.join("hfi1_0")).unwrap();
        assert_eq!(
            got,
            InfiniBandDevice {
                // name: "hfi1_0".to_string(),
                board_id:
                    "HPE 100Gb 1-port OP101 QSFP28 x16 PCIe Gen3 with Intel Omni-Path Adapter"
                        .to_string(),
                fw_ver: "1.27.0".to_string(),
                hca_type: "".to_string(),
                ports: vec![InfiniBandPort {
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
                    hw_counters: None,
                }],
            }
        );

        let got = parse_infiniband_device(root.join("mlx4_0")).unwrap();
        assert_eq!(
            got,
            InfiniBandDevice {
                // name: "mlx4_0".to_string(),
                board_id: "SM_1141000001000".to_string(),
                fw_ver: "2.31.5050".to_string(),
                hca_type: "MT4099".to_string(),
                ports: vec![
                    InfiniBandPort {
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
                        hw_counters: None,
                    },
                    InfiniBandPort {
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
                        hw_counters: None,
                    },
                ],
            }
        );

        let got = parse_infiniband_device(root.join("mlx5_0")).unwrap();
        assert_eq!(
            got,
            InfiniBandDevice {
                board_id: "SM_2001000001034".to_string(),
                fw_ver: "14.28.2006".to_string(),
                hca_type: "MT4118".to_string(),
                ports: vec![InfiniBandPort {
                    port: 1,
                    state: "ACTIVE".to_string(),
                    state_id: 4,
                    phys_state: "ACTIVE".to_string(),
                    phys_state_id: 4,
                    rate: 3125000000,
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
                        multicast_rcv_packets: Some(0),
                        multicast_xmit_packets: Some(0),
                        port_rcv_constraint_errors: Some(0),
                        port_rcv_data: Some(72505381512),
                        port_rcv_discards: None,
                        port_rcv_errors: Some(0),
                        port_rcv_packets: Some(541889824),
                        port_rcv_remote_physical_errors: Some(0),
                        port_rcv_switch_relay_errors: Some(0),
                        port_xmit_constraint_errors: Some(0),
                        port_xmit_data: Some(11523046035392),
                        port_xmit_discards: Some(0),
                        port_xmit_packets: Some(10907922116),
                        port_xmit_wait: Some(0),
                        symbol_error: Some(0),
                        unicast_rcv_packets: Some(541889824),
                        unicast_xmit_packets: Some(10907922116),
                        vl15_dropped: Some(0),
                    },
                    hw_counters: Some(InfiniBandHwCounters {
                        duplicate_request: Some(41),
                        implied_nak_seq_err: Some(0),
                        lifespan: Some(10),
                        local_ack_timeout_err: Some(131),
                        np_cnp_sent: None,
                        np_ecn_marked_roce_packets: None,
                        out_of_buffer: Some(0),
                        out_of_sequence: Some(1),
                        packet_seq_err: Some(1),
                        req_cqe_error: Some(3481),
                        req_cqe_flush_error: Some(80),
                        req_remote_access_errors: Some(0),
                        req_remote_invalid_request: Some(0),
                        resp_cqe_error: Some(8109),
                        resp_cqe_flush_error: Some(4708),
                        resp_local_length_error: Some(0),
                        resp_remote_access_errors: Some(0),
                        rnr_nak_retry_err: Some(0),
                        roce_adp_retrans: Some(99),
                        roce_adp_retrans_to: Some(4),
                        roce_slow_restart: Some(0),
                        roce_slow_restart_cnps: Some(131),
                        roce_slow_restart_trans: Some(0),
                        rp_cnp_handled: None,
                        rp_cnp_ignored: None,
                        rx_atomic_requests: Some(0),
                        rx_dct_connect: Some(0),
                        rx_icrc_encapsulated: None,
                        rx_read_requests: Some(175528982),
                        rx_write_requests: Some(742114),
                    }),
                }],
            },
        )
    }
}
