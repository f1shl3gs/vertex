//! Exposes various statistics from /proc/net/sockstat and /proc/net/sockstat6

use event::Metric;

use super::{read_to_string, Error};

pub async fn gather(proc_path: &str) -> Result<Vec<Metric>, Error> {
    let stat4 = sockstat4(proc_path).await?;
    let stat6 = sockstat6(proc_path).await?;

    let mut metrics = stat4.metrics(false);
    metrics.extend(stat6.metrics(true));

    Ok(metrics)
}

/// NetSockstatProtocol contains statistics about a given socket protocol.
/// Option fields indicate that the value may or may not be present on
/// any given protocol
#[derive(Default)]
struct NetSockstatProtocol {
    protocol: String,
    inuse: i32,
    orphan: Option<i32>,
    tw: Option<i32>,
    alloc: Option<i32>,
    mem: Option<i32>,
    memory: Option<i32>,
}

/// A NetSockstat contains the output of /proc/net/sockstat{,6} for IPv4
/// or IPv6, respectively
#[derive(Default)]
struct NetSockstat {
    used: Option<i32>,
    protocols: Vec<NetSockstatProtocol>,
}

impl NetSockstat {
    fn metrics(&self, v6: bool) -> Vec<Metric> {
        const PAGESIZE: f64 = 4096.0;
        let mut metrics = Vec::with_capacity(7);

        // If sockstat contains the number of used sockets, export it
        if !v6 && self.used.is_some() {
            let v = self.used.unwrap() as f64;

            metrics.push(Metric::gauge(
                "node_sockstat_sockets_used",
                "Number of IPv4 sockets in use.",
                v,
            ));
        }

        for nsp in &self.protocols {
            let name = &format!("node_sockstat_{}_inuse", nsp.protocol);
            let desc = &format!("Number of {} sockets in stat inuse", nsp.protocol);
            metrics.push(Metric::gauge(name, desc, nsp.inuse));

            if let Some(v) = nsp.orphan {
                let name = &format!("node_sockstat_{}_orphan", nsp.protocol);
                let desc = &format!("Number of {} sockets in stat orphan", nsp.protocol);
                metrics.push(Metric::gauge(name, desc, v));
            }

            if let Some(v) = nsp.tw {
                let name = &format!("node_sockstat_{}_tw", nsp.protocol);
                let desc = &format!("Number of {} sockets in stat tw", nsp.protocol);
                metrics.push(Metric::gauge(name, desc, v));
            }

            if let Some(v) = nsp.alloc {
                let name = &format!("node_sockstat_{}_alloc", nsp.protocol);
                let desc = &format!("Number of {} sockets in stat alloc", nsp.protocol);
                metrics.push(Metric::gauge(name, desc, v));
            }

            if let Some(v) = nsp.mem {
                let v = v as f64 * PAGESIZE;
                let name = &format!("node_sockstat_{}_mem_bytes", nsp.protocol);
                let desc = &format!("Number of {} sockets in stat mem", nsp.protocol);
                metrics.push(Metric::gauge(name, desc, v));
            }

            if let Some(v) = nsp.memory {
                let name = &format!("node_sockstat_{}_memory", nsp.protocol);
                let desc = &format!("Number of {} sockets in stat memory", nsp.protocol);
                metrics.push(Metric::gauge(name, desc, v as f64))
            }
        }

        metrics
    }
}

async fn sockstat4(root: &str) -> Result<NetSockstat, Error> {
    // This file is small and can be read with one syscall
    let path = format!("{}/net/sockstat", root);
    let content = read_to_string(path).await?;

    parse_sockstat(&content)
}

async fn sockstat6(root: &str) -> Result<NetSockstat, Error> {
    // This file is small and can be read with one syscall
    let path = format!("{}/net/sockstat6", root);
    let content = read_to_string(path).await?;

    parse_sockstat(&content)
}

fn parse_sockstat(content: &str) -> Result<NetSockstat, Error> {
    let mut stat = NetSockstat::default();

    for line in content.lines() {
        let fields = line.split_ascii_whitespace().collect::<Vec<_>>();
        let size = fields.len();

        if size < 3 || size % 2 != 1 {
            return Err(Error::from("malformed sockstat line"));
        }

        let proto = fields[0].strip_suffix(':').unwrap();
        if proto == "sockets" {
            // Special case: IPv4 has a sockets "used" key/value pair that we
            // embed at the top level of the structure
            stat.used = fields[2].parse().ok();
            continue;
        }

        // Parse all other lines as individual protocols
        let mut nsp = NetSockstatProtocol {
            protocol: proto.to_string(),
            ..Default::default()
        };

        let mut i = 1;
        loop {
            if i == size {
                break;
            }

            match fields[i] {
                "inuse" => nsp.inuse = fields[i + 1].parse().unwrap_or(0),
                "orphan" => nsp.orphan = fields[i + 1].parse().ok(),
                "tw" => nsp.tw = fields[i + 1].parse().ok(),
                "alloc" => nsp.alloc = fields[i + 1].parse().ok(),
                "mem" => nsp.mem = fields[i + 1].parse().ok(),
                "memory" => nsp.memory = fields[i + 1].parse().ok(),
                _ => {}
            }

            i += 2;
        }

        stat.protocols.push(nsp);
    }

    Ok(stat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sockstat() {
        let input = r#"TCP6: inuse 24
UDP6: inuse 9
UDPLITE6: inuse 0
RAW6: inuse 1
FRAG6: inuse 0 memory 0"#;
        let ns = parse_sockstat(input).unwrap();
        assert_eq!(ns.used, None);

        // TCP6
        let nsp = ns.protocols.get(0).unwrap();
        assert_eq!(nsp.protocol, "TCP6");
        assert_eq!(nsp.inuse, 24);
        assert_eq!(nsp.orphan, None);
        assert_eq!(nsp.tw, None);

        // UDP6
        let nsp = ns.protocols.get(1).unwrap();
        assert_eq!(nsp.protocol, "UDP6");
        assert_eq!(nsp.inuse, 9);
        assert_eq!(nsp.mem, None);

        // UDPLITE6
        let nsp = ns.protocols.get(2).unwrap();
        assert_eq!(nsp.protocol, "UDPLITE6");
        assert_eq!(nsp.inuse, 0);

        // RAW6
        let nsp = ns.protocols.get(3).unwrap();
        assert_eq!(nsp.protocol, "RAW6");
        assert_eq!(nsp.inuse, 1);

        // FRAG6
        let nsp = ns.protocols.get(4).unwrap();
        assert_eq!(nsp.protocol, "FRAG6");
        assert_eq!(nsp.inuse, 0);
        assert_eq!(nsp.memory, Some(0));
    }
}
