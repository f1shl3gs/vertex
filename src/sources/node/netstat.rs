use std::collections::BTreeMap;
use std::path::PathBuf;

use event::Metric;
use framework::config::serde_regex;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncBufReadExt;

use super::Error;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_fields")]
    #[serde(with = "serde_regex")]
    fields: Regex,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            fields: default_fields(),
        }
    }
}

fn default_fields() -> Regex {
    Regex::new("^(.*_(InErrors|InErrs)|Ip_Forwarding|Ip(6|Ext)_(InOctets|OutOctets)|Icmp6?_(InMsgs|OutMsgs)|TcpExt_(Listen.*|Syncookies.*|TCPSynRetrans|TCPTimeouts|TCPOFOQueue|TCPRcvQDrop)|Tcp_(ActiveOpens|InSegs|OutSegs|OutRsts|PassiveOpens|RetransSegs|CurrEstab)|Udp6?_(InDatagrams|OutDatagrams|NoPorts|RcvbufErrors|SndbufErrors))$").unwrap()
}

pub async fn gather(conf: Config, proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let mut net_stats = get_net_stats(proc_path.join("net/netstat")).await?;
    let snmp_stats = get_net_stats(proc_path.join("net/snmp")).await?;
    let snmp6_stats = get_snmp6_stats(proc_path.join("net/snmp6")).await?;

    // Merge the results of snmpStats into netStats (collisions are possible,
    // but we know that the keys are always unique for the give use case.
    for (k, v) in snmp_stats {
        net_stats.insert(k, v);
    }
    for (k, v) in snmp6_stats {
        net_stats.insert(k, v);
    }

    let mut metrics = Vec::new();
    for (protocol, stats) in net_stats {
        for (name, value) in stats {
            let key = format!("{}_{}", protocol, name);
            let v = match value.parse::<f64>() {
                Ok(v) => v,
                _ => continue,
            };

            if !conf.fields.is_match(&key) {
                continue;
            }

            metrics.push(Metric::gauge(
                format!("node_netstat_{}", key),
                format!("Statistic {}{}", protocol, name),
                v,
            ));
        }
    }

    Ok(metrics)
}

async fn get_net_stats(path: PathBuf) -> Result<BTreeMap<String, BTreeMap<String, String>>, Error> {
    let f = tokio::fs::File::open(path).await?;
    let r = tokio::io::BufReader::new(f);
    let mut lines = r.lines();
    let mut stats: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();

    while let Some(line) = lines.next_line().await? {
        let names = line.split_ascii_whitespace().collect::<Vec<_>>();

        let line = match lines.next_line().await? {
            Some(line) => line,
            None => break,
        };
        let values = line.split_ascii_whitespace().collect::<Vec<_>>();

        // remove trailing :
        let protocol = names[0].strip_suffix(':').unwrap();
        stats.insert(protocol.to_string(), BTreeMap::new());
        if names.len() != values.len() {
            return Err(Error::from("mismatch field count"));
        }

        for i in 0..names.len() {
            let props = stats.get_mut(protocol).unwrap();

            props.insert(names[i].to_string(), values[i].to_string());
        }
    }

    Ok(stats)
}

async fn get_snmp6_stats(
    path: PathBuf,
) -> Result<BTreeMap<String, BTreeMap<String, String>>, Error> {
    let f = tokio::fs::File::open(path).await?;
    let r = tokio::io::BufReader::new(f);
    let mut lines = r.lines();
    let mut stats: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();

    while let Some(line) = lines.next_line().await? {
        let stat = line.split_ascii_whitespace().collect::<Vec<_>>();
        if stat.len() < 2 {
            continue;
        }

        // Expect to have 6 in metric name, skip line otherwise
        let index = match stat[0].find('6') {
            Some(i) => i,
            _ => continue,
        };

        let protocol = &stat[0][..index + 1];
        let name = &stat[0][index + 1..];
        let props = match stats.get_mut(protocol) {
            Some(props) => props,
            _ => {
                let props = BTreeMap::new();
                stats.insert(protocol.to_string(), props);

                stats.get_mut(protocol).unwrap()
            }
        };

        props.insert(name.to_string(), stat[1].to_string());
    }

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_net_stats() {
        let path = "tests/node/proc/net/netstat".into();

        let stats = get_net_stats(path).await.unwrap();

        let props = stats.get("TcpExt").unwrap();
        assert_eq!(props.get("DelayedACKs").unwrap(), "102471");

        let props = stats.get("IpExt").unwrap();
        assert_eq!(props.get("OutOctets").unwrap(), "2786264347");
    }

    #[tokio::test]
    async fn test_snmp_stats() {
        let path = "tests/node/proc/net/snmp".into();
        let stats = get_net_stats(path).await.unwrap();

        let props = stats.get("Udp").unwrap();
        assert_eq!(props.get("RcvbufErrors").unwrap(), "9");

        let props = stats.get("Udp").unwrap();
        assert_eq!(props.get("SndbufErrors").unwrap(), "8");
    }

    #[tokio::test]
    async fn test_snmp6_stats() {
        let path = "tests/node/proc/net/snmp6".into();
        let stats = get_snmp6_stats(path).await.unwrap();

        let props = stats.get("Ip6").unwrap();
        assert_eq!(props.get("InOctets").unwrap(), "460");

        let props = stats.get("Icmp6").unwrap();
        assert_eq!(props.get("OutMsgs").unwrap(), "8");

        let props = stats.get("Udp6").unwrap();
        assert_eq!(props.get("RcvbufErrors").unwrap(), "9");

        let props = stats.get("Udp6").unwrap();
        assert_eq!(props.get("SndbufErrors").unwrap(), "8");
    }
}
