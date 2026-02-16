use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use configurable::Configurable;
use event::Metric;
use event::tags::Tags;
use serde::{Deserialize, Serialize};

use super::Error;

#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_labels")]
    labels: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            labels: default_labels(),
        }
    }
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

pub async fn gather(conf: Config, proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let stats = parse_ipvs_stats(&proc_path)?;

    let mut metrics = vec![
        Metric::sum(
            "node_ipvs_connections_total",
            "The total number of connections made.",
            stats.connections,
        ),
        Metric::sum(
            "node_ipvs_incoming_packets_total",
            "The total number of incoming packets.",
            stats.incoming_packets,
        ),
        Metric::sum(
            "node_ipvs_outgoing_packets_total",
            "The total number of outgoing packets.",
            stats.outgoing_packets,
        ),
        Metric::sum(
            "node_ipvs_incoming_bytes_total",
            "The total amount of incoming data.",
            stats.incoming_bytes,
        ),
        Metric::sum(
            "node_ipvs_outgoing_bytes_total",
            "The total amount of outgoing data.",
            stats.outgoing_bytes,
        ),
    ];

    let backends = parse_ipvs_backend_status(proc_path)?;
    let mut sums = BTreeMap::new();
    let mut label_values = BTreeMap::new();
    for backend in backends {
        let local_address = if backend.local_address != "<nil>" {
            backend.local_address
        } else {
            String::new()
        };

        let mut kv = Vec::with_capacity(conf.labels.len());
        for label in &conf.labels {
            let lv = match label.as_str() {
                "local_address" => local_address.clone(),
                "local_port" => backend.local_port.to_string(),
                "remote_address" => backend.remote_address.clone(),
                "remote_port" => backend.remote_port.to_string(),
                "proto" => backend.proto.clone(),
                "local_mark" => backend.local_mark.clone(),
                _ => "".to_string(),
            };

            kv.push(lv)
        }

        let key = kv.join("-");
        let status = sums
            .entry(key.clone())
            .or_insert_with(IPVSBackendStatus::default);

        status.active_conn += backend.active_conn;
        status.inact_conn += backend.inact_conn;
        status.weight += backend.weight;
        label_values.insert(key, kv);
    }

    for (k, status) in &sums {
        let kv = match label_values.get(k) {
            Some(kv) => kv,
            None => continue,
        };

        let mut tags = Tags::with_capacity(conf.labels.len());
        for (i, key) in conf.labels.iter().enumerate() {
            tags.insert(key, kv[i].clone());
        }

        metrics.extend([
            Metric::gauge_with_tags(
                "node_ipvs_backend_connections_active",
                "The current active connections by local and remote address.",
                status.active_conn,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_ipvs_backend_connections_inactive",
                "The current inactive connections by local and remote address.",
                status.inact_conn,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_ipvs_backend_weight",
                "The current backend weight by local and remote address.",
                status.weight,
                tags,
            ),
        ])
    }

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

fn parse_ipvs_stats(root: &Path) -> Result<IPVSStats, Error> {
    let data = std::fs::read_to_string(root.join("net/ip_vs_stats"))?;
    let lines = data.lines().collect::<Vec<_>>();
    if lines.len() < 4 {
        return Err("ip_vs_stats corrupt: too short".into());
    }

    let fields = lines[2].split_ascii_whitespace().collect::<Vec<_>>();
    if fields.len() != 5 {
        return Err("ip_vs_stats corrupt: unexpected number of fields".into());
    }

    let connections = u64::from_str_radix(fields[0], 16)?;
    let incoming_packets = u64::from_str_radix(fields[1], 16)?;
    let outgoing_packets = u64::from_str_radix(fields[2], 16)?;
    let incoming_bytes = u64::from_str_radix(fields[3], 16)?;
    let outgoing_bytes = u64::from_str_radix(fields[4], 16)?;

    Ok(IPVSStats {
        connections,
        incoming_packets,
        outgoing_packets,
        incoming_bytes,
        outgoing_bytes,
    })
}

/// IPVSBackendStatus holds current metrics of one virtual / real address pair
#[derive(Default)]
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

fn parse_ipvs_backend_status(root: PathBuf) -> Result<Vec<IPVSBackendStatus>, Error> {
    let data = std::fs::read_to_string(root.join("net/ip_vs"))?;

    let mut status = vec![];
    let mut proto = String::new();
    let mut local_mark = String::new();
    let mut local_address = String::new();
    let mut local_port = 0u16;

    for line in data.lines() {
        let fields = line.split_ascii_whitespace().collect::<Vec<_>>();
        if fields.is_empty() {
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
                local_address.clear();
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

    Ok(status)
}

fn parse_ip_port(s: &str) -> Result<(String, u16), Error> {
    let ip = {
        match s.len() {
            13 => {
                let p1 = u8::from_str_radix(&s[0..2], 16)?;
                let p2 = u8::from_str_radix(&s[2..4], 16)?;
                let p3 = u8::from_str_radix(&s[4..6], 16)?;
                let p4 = u8::from_str_radix(&s[6..8], 16)?;

                std::net::Ipv4Addr::new(p1, p2, p3, p4).to_string()
            }

            46 => {
                // ipv6
                let p1 = u16::from_str_radix(&s[1..5], 16)?;
                let p2 = u16::from_str_radix(&s[6..10], 16)?;
                let p3 = u16::from_str_radix(&s[11..15], 16)?;
                let p4 = u16::from_str_radix(&s[16..20], 16)?;
                let p5 = u16::from_str_radix(&s[21..25], 16)?;
                let p6 = u16::from_str_radix(&s[26..30], 16)?;
                let p7 = u16::from_str_radix(&s[31..35], 16)?;
                let p8 = u16::from_str_radix(&s[36..40], 16)?;

                std::net::Ipv6Addr::new(p1, p2, p3, p4, p5, p6, p7, p8).to_string()
            }
            _ => return Err(Error::from("unexpected IP:Port")),
        }
    };

    let port = &s[s.len() - 4..];
    let port = u16::from_str_radix(port, 16)?;

    Ok((ip, port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
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
