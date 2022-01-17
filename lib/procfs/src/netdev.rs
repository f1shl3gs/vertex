use crate::{read_to_string, Error, ProcFS};

#[derive(Debug, PartialEq)]
pub struct NetDeviceStatus {
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

impl ProcFS {
    pub async fn netdev(&self) -> Result<Vec<NetDeviceStatus>, Error> {
        let path = self.root.join("net/dev");

        let content = read_to_string(path).await?;
        let lines = content.lines();
        let mut stats = Vec::new();
        for line in lines.skip(2) {
            let stat = parse_netdev_status(line)?;
            stats.push(stat);
        }

        Ok(stats)
    }
}

/// parse lines like
/// ```text
///  face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
//     lo: 14748809    4780    0    0    0     0          0         0 14748809    4780    0    0    0     0       0          0
/// ```
pub fn parse_netdev_status(line: &str) -> Result<NetDeviceStatus, Error> {
    let parts = line.trim().split_ascii_whitespace().collect::<Vec<_>>();

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

    Ok(NetDeviceStatus {
        name: name.to_string(),
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
    fn test_parse_netdev_device_status() {
        let s = "  lo: 14748809    4780    0    0    0     0          0         0 14748809    4780    0    0    0     0       0          0";
        let ds = parse_netdev_status(s).unwrap();

        assert_eq!(ds.recv_bytes, 14748809);
        assert_eq!(ds.recv_packets, 4780);

        assert_eq!(ds.transmit_bytes, 14748809);
        assert_eq!(ds.transmit_packets, 4780);
    }

    #[tokio::test]
    async fn test_get_net_dev_stats() {
        let procfs = ProcFS::test_procfs();
        let stats = procfs.netdev().await.unwrap();

        assert_eq!(
            stats[0],
            NetDeviceStatus {
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
            NetDeviceStatus {
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
