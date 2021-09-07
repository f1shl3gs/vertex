use serde::{Deserialize, Serialize};
use crate::event::Metric;
use crate::sources::node::errors::Error;
use crate::sources::node::read_to_string;
use tokio::io::AsyncBufReadExt;

#[derive(Debug, Deserialize, Serialize)]
pub struct IPVSConfig {
    #[serde(default = "default_labels")]
    labels: Vec<String>,
}

fn default_labels() -> Vec<String> {
    vec![
        "local_address".to_string(),
        "local_port".to_string(),
        "remote_address".to_string(),
        "remote_port".to_string(),
        "proto".to_string(),
        "local_mark".to_string(),
    ]
}

pub async fn gather(proc_path: &str) -> Result<Vec<Metric>, ()> {
    let stats = parse_ipvs_stats(proc_path).await.map_err(|err| {
        warn!("read ip_vs_stat failed"; "err" => err );
    })?;

    let mut metrics = vec![
        Metric::sum("node_ipvs_connections_total", "The total number of connections made.", stats.connections as f64),
        Metric::sum("node_ipvs_incoming_packets_total", "The total number of incoming packets.", stats.incoming_packets as f64),
        Metric::sum("node_ipvs_outgoing_packets_total", "The total number of outgoing packets.", stats.outgoing_packets as f64),
        Metric::sum("node_ipvs_incoming_bytes_total", "The total amount of incoming data.", stats.incoming_bytes as f64),
        Metric::sum("node_ipvs_outgoing_bytes_total", "The total amount of outgoing data.", stats.outgoing_bytes as f64),
    ];

    Ok(metrics)
}

// IPVSStats holds IPVS statistics, as exposed by the kernel in /proc/net/ip_vs_stats
struct IPVSStats {
    // Total count of connects.
    connections: u64,

    // Total incoming packets processed
    incoming_packets: u64,

    // Total outgoing packets processed
    outgoing_packets: u64,

    // Total incoming traffic
    incoming_bytes: u64,

    // Total outgoing traffic
    outgoing_bytes: u64,
}

async fn parse_ipvs_stats(root: &str) -> Result<IPVSStats, Error> {
    let path = &format!("{}/net/ip_vs_stat", root);
    let content = read_to_string(path).await?;
    let lines = content.lines().collect::<Vec<_>>();
    if lines.len() < 4 {
        return Err(Error::new_invalid("ip_vs_stats corrupt: too short"));
    }

    let stat_fields = lines[2].split_ascii_whitespace().collect::<Vec<_>>();
    if stat_fields.len() != 5 {
        return Err(Error::new_invalid("ip_vs_stats corrupt: unexpected number of fields"));
    }

    let connections = u64::from_str_radix(stat_fields[0], 16).map_err(Error::from)?;
    let incoming_packets = u64::from_str_radix(stat_fields[1], 16).map_err(Error::from)?;
    let outgoing_packets = u64::from_str_radix(stat_fields[2], 16).map_err(Error::from)?;
    let incoming_bytes = u64::from_str_radix(stat_fields[3], 16).map_err(Error::from)?;
    let outgoing_bytes = u64::from_str_radix(stat_fields[4], 16).map_err(Error::from)?;

    Ok(IPVSStats {
        connections,
        incoming_packets,
        outgoing_packets,
        incoming_bytes,
        outgoing_bytes,
    })
}

/// IPVSBackendStatus holds current metrics of one virtual / real address pair
struct IPVSBackendStatus {
    // The local (virtual) IP address
    local_address: String,

    // The remove (real) IP address
    remote_address: String,

    // The local (virtual) port
    local_port: u16,

    // The remove (real) port
    remote_port: u16,

    // The local firewall mark
    local_mark: String,

    // The transport protocol (TCP or UDP)
    proto: String,

    // The current number of active connections for this virtual/real address pair
    active_conn: u64,

    // The current number of inactive connections for this virtual/real address pair
    inact_conn: u64,

    // The current weight of this virtual/real address pair
    weight: u64,
}

async fn parse_ipvs_backend_status(root: &str) -> Result<Vec<IPVSBackendStatus>, Error> {
    let path = &format!("{}/net/ip_vs", root);
    let f = tokio::fs::File::open(path).await.map_err(Error::from)?;
    let r = tokio::io::BufReader::new(f);
    let mut lines = r.lines();

    let mut status = vec![];
    let mut proto = String::new();
    let mut local_mark = String::new();
    let mut local_address = String::new();
    let mut local_port = 0u16;

    while let Some(line) = lines.next_line().await.map_err(Error::from)? {
        let fields = line.split_ascii_whitespace().collect::<Vec<_>>();
        if fields.len() == 0 {
            continue;
        }

        if fields[0] == "IP" || fields[0] == "Prot" || fields[1] == "RemoteAddress:Port" {
            continue;
        }

        match fields[0] {
            "TCP" | "UDP" => {
                if fields.len() < 2 {
                    continue;
                }

                proto = fields[0].to_string();
                local_mark = "".to_string();
                let (la, lp) = parse_ip_port(fields[1])?;
                local_address = la;
                local_port = lp;
            }

            "FWM" => {
                if fields.len() < 2 {
                    continue;
                }

                proto = fields[0].to_string();
                local_mark = fields[1].to_string();
                local_port = 0;
            }

            "->" => {
                if fields.len() < 6 {
                    continue;
                }

                let (remote_address, remote_port) = parse_ip_port(fields[1])?;
                let weight = fields[3].parse()?;
                let active_conn = fields[4].parse()?;
                let inact_conn = fields[5].parse()?;

                status.push(IPVSBackendStatus {
                    local_mark: local_mark.clone(),
                    proto: proto.clone(),
                    local_address: local_address.clone(),
                    remote_address,
                    local_port,
                    remote_port,
                    active_conn,
                    inact_conn,
                    weight,
                })
            }

            _ => {}
        }
    }

    todo!()
}

fn parse_ip_port(s: &str) -> Result<(String, u16), Error> {
    let ip = {
        match s.len() {
            13 => {
                let p1 = u8::from_str_radix(&s[0..2], 16).map_err(Error::from)?;
                let p2 = u8::from_str_radix(&s[2..4], 16).map_err(Error::from)?;
                let p3 = u8::from_str_radix(&s[4..6], 16).map_err(Error::from)?;
                let p4 = u8::from_str_radix(&s[6..8], 16).map_err(Error::from)?;

                std::net::Ipv4Addr::new(p1, p2, p3, p4).to_string()
            }

            46 => {
                // ipv6
                let p1 = u16::from_str_radix(&s[1..5], 16).map_err(Error::from)?;
                let p2 = u16::from_str_radix(&s[6..10], 16).map_err(Error::from)?;
                let p3 = u16::from_str_radix(&s[11..15], 16).map_err(Error::from)?;
                let p4 = u16::from_str_radix(&s[16..20], 16).map_err(Error::from)?;
                let p5 = u16::from_str_radix(&s[21..25], 16).map_err(Error::from)?;
                let p6 = u16::from_str_radix(&s[26..30], 16).map_err(Error::from)?;
                let p7 = u16::from_str_radix(&s[31..35], 16).map_err(Error::from)?;
                let p8 = u16::from_str_radix(&s[36..40], 16).map_err(Error::from)?;

                std::net::Ipv6Addr::new(p1, p2, p3, p4, p5, p6, p7, p8).to_string()
            }
            _ => return Err(Error::new_invalid("unexpected IP:Port"))
        }
    };

    let port = &s[s.len() - 4..];
    let port = u16::from_str_radix(port, 16).map_err(Error::from)?;

    Ok((ip, port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ip_port() {
        let input = "C0A80016:0CEA";
        let (addr, port) = parse_ip_port(input).unwrap();
        assert_eq!(addr, "192.168.0.22");
        assert_eq!(port, 3306);

        let input = "[2620:0000:0000:0000:0000:0000:0000:0001]:0050";
        let (addr, port) = parse_ip_port(input).unwrap();
        assert_eq!(addr, "2620::1");
        assert_eq!(port, 80);
    }
}