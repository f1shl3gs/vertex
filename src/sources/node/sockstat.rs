//! Exposes various statistics from /proc/net/sockstat and /proc/net/sockstat6

use std::io::ErrorKind;

use event::Metric;

use super::{Error, Paths, read_file_no_stat};

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let mut metrics = Vec::with_capacity(16);

    // if IPv4 and/or IPv6 are disabled on this kernel, handle it gracefully
    let root = paths.proc().join("net");
    for filename in ["sockstat", "sockstat6"] {
        match read_file_no_stat(root.join(filename)) {
            Ok(content) => {
                let stats = parse_sockstat(&content)?;

                // If sockstat contains the number of used sockets, export it
                if filename == "sockstat"
                    && let Some(value) = stats.used
                {
                    metrics.push(Metric::gauge(
                        "node_sockstat_sockets_used",
                        "Number of IPv4 sockets in use.",
                        value,
                    ));
                }

                for (protocol, stats) in stats.protocols {
                    metrics.push(Metric::gauge(
                        format!("node_sockstat_{}_inuse", protocol),
                        format!("Number of {} sockets in state inuse", protocol),
                        stats.inuse,
                    ));

                    if let Some(value) = stats.orphan {
                        metrics.push(Metric::gauge(
                            format!("node_sockstat_{}_orphan", protocol),
                            format!("Number of {} sockets in state orphan", protocol),
                            value,
                        ));
                    }

                    if let Some(value) = stats.tw {
                        metrics.push(Metric::gauge(
                            format!("node_sockstat_{}_tw", protocol),
                            format!("Number of {} sockets in state tw", protocol),
                            value,
                        ));
                    }

                    if let Some(value) = stats.alloc {
                        metrics.push(Metric::gauge(
                            format!("node_sockstat_{}_alloc", protocol),
                            format!("Number of {} sockets in state alloc", protocol),
                            value,
                        ));
                    }

                    if let Some(value) = stats.mem {
                        metrics.extend([
                            Metric::gauge(
                                format!("node_sockstat_{}_mem", protocol),
                                format!("Number of {} sockets in state mem", protocol),
                                value,
                            ),
                            Metric::gauge(
                                format!("node_sockstat_{}_mem_bytes", protocol),
                                format!("Number of {} sockets in state mem_bytes", protocol),
                                value * 4096, // for x86 platform page_size is always 4096
                            ),
                        ]);
                    }

                    if let Some(value) = stats.memory {
                        metrics.push(Metric::gauge(
                            format!("node_sockstat_{}_memory", protocol),
                            format!("Number of {} sockets in state memory", protocol),
                            value,
                        ))
                    }
                }
            }
            Err(err) => {
                if err.kind() != ErrorKind::NotFound {
                    return Err(err.into());
                }

                debug!(message = "sockstat statistics not found, skipping", %filename);
            }
        }
    }

    Ok(metrics)
}

/// NetSockstatProtocol contains statistics about a given socket protocol.
/// Option fields indicate that the value may or may not be present on
/// any given protocol
#[derive(Default)]
struct ProtocolStats {
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
struct NetSockstat<'a> {
    used: Option<i32>,
    protocols: Vec<(&'a str, ProtocolStats)>,
}

fn parse_sockstat(content: &str) -> Result<NetSockstat<'_>, Error> {
    let mut stat = NetSockstat::default();

    for line in content.lines() {
        let Some((proto, remaining)) = line.split_once(':') else {
            continue;
        };

        if proto == "sockets" {
            let Some((key, value)) = remaining.trim().split_once(' ') else {
                return Err(Error::Malformed("sockets line of sockstat"));
            };
            if key != "used" {
                return Err(Error::Malformed("sockets line of sockstat"));
            }

            stat.used = Some(value.parse::<i32>()?);
            continue;
        }

        stat.protocols
            .push((proto, parse_protocol_stats(remaining)?))
    }

    Ok(stat)
}

fn parse_protocol_stats(line: &str) -> Result<ProtocolStats, Error> {
    let mut fields = line.split_ascii_whitespace();

    let mut stats = ProtocolStats::default();
    while let Some(key) = fields.next() {
        let value = match fields.next() {
            Some(field) => field.parse::<i32>()?,
            None => return Err(Error::Malformed("protocol stats")),
        };

        match key {
            "inuse" => stats.inuse = value,
            "orphan" => stats.orphan = Some(value),
            "tw" => stats.tw = Some(value),
            "alloc" => stats.alloc = Some(value),
            "mem" => stats.mem = Some(value),
            "memory" => stats.memory = Some(value),
            _ => {}
        }
    }

    Ok(stats)
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
    fn parse() {
        let content = std::fs::read_to_string("tests/node/fixtures/proc/net/sockstat6").unwrap();
        let ns = parse_sockstat(&content).unwrap();
        assert_eq!(ns.used, None);

        // TCP6
        let (protocol, nsp) = ns.protocols.first().unwrap();
        assert_eq!(*protocol, "TCP6");
        assert_eq!(nsp.inuse, 17);
        assert_eq!(nsp.orphan, None);
        assert_eq!(nsp.tw, None);

        // UDP6
        let (protocol, nsp) = ns.protocols.get(1).unwrap();
        assert_eq!(*protocol, "UDP6");
        assert_eq!(nsp.inuse, 9);
        assert_eq!(nsp.mem, None);

        // UDPLITE6
        let (protocol, nsp) = ns.protocols.get(2).unwrap();
        assert_eq!(*protocol, "UDPLITE6");
        assert_eq!(nsp.inuse, 0);

        // RAW6
        let (protocol, nsp) = ns.protocols.get(3).unwrap();
        assert_eq!(*protocol, "RAW6");
        assert_eq!(nsp.inuse, 1);

        // FRAG6
        let (protocol, nsp) = ns.protocols.get(4).unwrap();
        assert_eq!(*protocol, "FRAG6");
        assert_eq!(nsp.inuse, 0);
        assert_eq!(nsp.memory, Some(0));
    }
}
