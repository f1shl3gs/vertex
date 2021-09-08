use crate::event::Metric;

/// Exposes UDP total lengths of the rx_queue and tx_queue
/// from `/proc/net/udp` and `/proc/netudp6`

pub async fn gather(proc_path: &str) -> Result<Vec<Metric>, ()> {
    todo!()
}

/// NetIPSocketSummary provides already computed values like the
/// total queue lengths or the total number of used sockets. In contrast to
/// NetIPSocket it does not collect the parsed lines into a slice.
struct NetIPSocketSummary {
    // tx_queue_length shows the total queue length of all parsed tx_queue lengths
    tx_queue_length: u64,

    // rx_queue_length shows the total queue length of all parsed rx_queue lengths
    rx_queue_length: u64,

    // used_sockets shows the total number of parsed lines representing the number
    // of used sockets
    used_sockets: u64,
}

async fn net_udp_summary(root: &str) {}

async fn net_udp6_summary() {}

async fn udp_summary() {}