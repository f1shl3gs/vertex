use configurable::Configurable;
use event::{Metric, tags};
use framework::config::serde_regex;
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::{Error, Paths, read_file_no_stat};

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

pub async fn collect(conf: Config, paths: Paths) -> Result<Vec<Metric>, Error> {
    let content = read_file_no_stat(paths.proc().join("net/dev"))?;

    let mut metrics = Vec::new();
    for line in content.lines().skip(2) {
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

        let tags = tags!("device" => stat.name);
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
    if parts.len() < 17 {
        return Err(Error::Malformed("netdev stat line"));
    }

    let name = parts[0].strip_suffix(':').unwrap_or(parts[0]);
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

/*

    #[derive(Debug)]
    struct LinkMessage {
        // Always set to AF_UNSPEC (0)
        family: u16,
        // device typ
        typ: u16,
        // Unique interface index, using a nonzero value with NetLink
        // will instruct the kernel to create a device with the given
        // index (kernel 3.7+ required)
        index: u32,
        // contains device flags, see netdevice(7)
        flags: u32,
        // Change flags, specifies which flags will be affected by the Flags field
        change: u32,
    }

    struct LinkAttributes {
        name: String,
        mtu: u32,

        stats: LinkStats,
    }

    struct Attribute {
        length: u16,
        typ: u16,
    }

    // https://github.com/torvalds/linux/blob/master/include/uapi/linux/if_link.h#L42-L246
    #[derive(Debug, Default)]
    struct LinkStats {
        rx_packets: u64, // total packets received
        tx_packets: u64, // total packets transmitted
        rx_bytes: u64,   // total bytes received
        tx_bytes: u64,   // total bytes transmitted
        rx_errors: u64,  // bad packets received
        tx_errors: u64,  // packet transmit problems
        rx_dropped: u64, // no space in linux buffers
        tx_dropped: u64, // no space available in linux
        multicast: u64,  // multicast packets received
        collisions: u64,

        // detailed rx_errors:
        rx_length_errors: u64,
        rx_over_errors: u64,   // receiver ring buff overflow
        rx_crc_errors: u64,    // recved pkt with crc error
        rx_frame_errors: u64,  // recv'd frame alignment error
        rx_fifo_errors: u64,   // recv'r fifo overrun
        rx_missed_errors: u64, // receiver missed packet

        // detailed tx_errors
        tx_aborted_errors: u64,
        tx_carrier_errors: u64,
        tx_fifo_errors: u64,
        tx_heartbeat_errors: u64,
        tx_window_errors: u64,

        // for cslip etc
        rx_compressed: u64,
        tx_compressed: u64,
        rx_no_handler: u64, // dropped, no handler found
    }

    impl LinkStats {
        fn parse(buf: &[u8]) -> Result<Self, Error> {
            let len = buf.len();
            if len != 184 && len != 192 && len != 200 {
                return Err(Error::Malformed("incorrect size, want: 184 or 192 or 200"));
            }

            let mut stats = LinkStats::default();
            unsafe {
                std::ptr::copy_nonoverlapping(
                    buf.as_ptr(),
                    (&mut stats) as *const Self as *mut u8,
                    len,
                )
            }

            Ok(stats)
        }
    }

    #[tokio::test]
    async fn nl() {
        let mut conn = NetlinkConnection::_conn(0).unwrap();

        #[rustfmt::skip]
        let req: [u8; 32] = [
            // length
            32, 0, 0, 0,
            18, 0,
            1, 3,
            158, 82, 110, 230, 54, 197, 8, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ];

        conn.write_all(&req).await.unwrap();

        let mut buf = [0u8; 4096];
        'RECV: loop {
            let count = conn.read(&mut buf).await.unwrap();
            println!("recv {}", count);

            println!("buf {:?}", &buf[..count]);

            let mut offset = 0;
            while offset + 16 < count {
                let header = unsafe { &*(buf.as_ptr().add(offset) as *const Header) };
                if header.length as usize + offset > count {
                    panic!("buf too short");
                }

                if header.length <= 16 {
                    offset += 16;
                    continue;
                }

                println!("header {:#?}", header);

                let data = &buf[offset + 16..offset + header.length as usize];
                println!("data {:?}", data);

                if header.typ == 3 {
                    break 'RECV;
                }

                offset += header.length as usize;
            }
        }
    }

    fn align(len: usize) -> usize {
        ((len) + 4 - 1) & !3
    }

    const IFLA_IFNAME: u16 = 3;
    const IFLA_MTU: u16 = 4;
    const IFLA_TXQLEN: u16 = 13;
    const IFLA_STATS64: u16 = 23;

    #[test]
    fn decode() {
        let input = &[
            0u8, 0, 4, 3, 1, 0, 0, 0, 73, 0, 1, 0, 0, 0, 0, 0, 7, 0, 3, 0, 108, 111, 0, 0, 8, 0,
            13, 0, 232, 3, 0, 0, 5, 0, 16, 0, 0, 0, 0, 0, 5, 0, 17, 0, 0, 0, 0, 0, 5, 0, 67, 0, 1,
            0, 0, 0, 8, 0, 4, 0, 0, 0, 1, 0, 8, 0, 50, 0, 0, 0, 0, 0, 8, 0, 51, 0, 0, 0, 0, 0, 8,
            0, 27, 0, 0, 0, 0, 0, 8, 0, 30, 0, 0, 0, 0, 0, 8, 0, 61, 0, 0, 0, 0, 0, 8, 0, 31, 0, 1,
            0, 0, 0, 8, 0, 40, 0, 255, 255, 0, 0, 8, 0, 41, 0, 0, 0, 1, 0, 8, 0, 58, 0, 0, 0, 1, 0,
            8, 0, 63, 0, 0, 0, 1, 0, 8, 0, 64, 0, 0, 0, 1, 0, 8, 0, 59, 0, 248, 255, 7, 0, 8, 0,
            60, 0, 255, 255, 0, 0, 8, 0, 66, 0, 0, 0, 0, 0, 8, 0, 32, 0, 1, 0, 0, 0, 5, 0, 33, 0,
            1, 0, 0, 0, 8, 0, 35, 0, 0, 0, 0, 0, 8, 0, 47, 0, 0, 0, 0, 0, 8, 0, 48, 0, 0, 0, 0, 0,
            6, 0, 68, 0, 0, 0, 0, 0, 6, 0, 69, 0, 0, 0, 0, 0, 5, 0, 39, 0, 0, 0, 0, 0, 10, 0, 1, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 10, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 204, 0, 23, 0, 231, 253,
            22, 17, 0, 0, 0, 0, 231, 253, 22, 17, 0, 0, 0, 0, 74, 107, 143, 131, 138, 0, 0, 0, 74,
            107, 143, 131, 138, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 100, 0, 7, 0, 231, 253, 22, 17, 231, 253, 22, 17, 74, 107, 143, 131, 74,
            107, 143, 131, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 12, 0,
            43, 0, 5, 0, 2, 0, 0, 0, 0, 0, 12, 0, 6, 0, 110, 111, 113, 117, 101, 117, 101, 0, 68,
            3, 26, 0, 20, 0, 45, 0, 8, 0, 1, 0, 1, 0, 0, 0, 5, 0, 2, 0, 0, 0, 0, 0, 140, 0, 2, 0,
            136, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0,
            1, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 39, 0, 0, 232, 3,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 160, 2, 10, 0, 8, 0,
            1, 0, 0, 0, 0, 128, 20, 0, 5, 0, 255, 255, 0, 0, 85, 0, 0, 0, 251, 120, 0, 0, 232, 3,
            0, 0, 244, 0, 2, 0, 1, 0, 0, 0, 64, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0,
            0, 0, 1, 0, 0, 0, 255, 255, 255, 255, 160, 15, 0, 0, 232, 3, 0, 0, 255, 255, 255, 255,
            128, 58, 9, 0, 128, 81, 1, 0, 3, 0, 0, 0, 88, 2, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 1, 0,
            0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 96, 234, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 16, 39, 0, 0,
            232, 3, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 238, 54, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 4, 0, 0, 0, 0, 0, 0, 255, 255, 0, 0, 255, 255, 255, 255, 1, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 52, 1, 3, 0, 38, 0, 0, 0, 0, 0, 0, 0, 227, 138, 243, 0, 0, 0, 0,
            0, 144, 253, 141, 159, 5, 0, 0, 0, 227, 138, 243, 0, 0, 0, 0, 0, 35, 139, 243, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 114, 110,
            243, 0, 0, 0, 0, 0, 114, 110, 243, 0, 0, 0, 0, 0, 80, 225, 134, 159, 5, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            152, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 60, 0, 6, 0, 7, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 20, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 5, 0, 8, 0, 0, 0, 0, 0, 36, 0, 14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 62, 128, 4, 0, 65,
            128, 252, 5, 0, 0, 16, 0, 2, 0, 158, 82, 110, 230, 60, 252, 15, 0,
        ];

        let input = &input[16..];
        let mut pos = 0;
        while pos < input.len() {
            let len = ((input[pos + 1] as u16) << 8) | (input[pos] as u16);
            let typ = ((input[pos + 3] as u16) << 8) | (input[pos + 2] as u16);

            println!("{}: {}", typ, len);

            match typ {
                IFLA_IFNAME => {
                    // `0` terminator is contained, but rust doesn't need it
                    println!(
                        "ifname: {}",
                        String::from_utf8_lossy(&input[pos + 4..pos + len as usize - 1])
                    )
                }
                IFLA_MTU => {
                    let data = &input[pos + 4..pos + len as usize];
                    println!("mtu: {:?}", u32::from_ne_bytes(data.try_into().unwrap()));
                }
                IFLA_TXQLEN => {
                    let len = u32::from_ne_bytes(input[pos + 4..pos + 8].try_into().unwrap());
                    println!("txqlen: {}", len);
                }
                IFLA_STATS64 => {
                    let data = &input[pos + 4..pos + len as usize];
                    let stats = LinkStats::parse(data).unwrap();
                    println!("stats: {:#?}", stats);
                }
                _ => {}
            }

            pos += align(len as usize);
        }
    }
*/
