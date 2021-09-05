use std::collections::BTreeMap;
use crate::{
    gauge_metric,
    config::{deserialize_regex, serialize_regex},
    sources::node::errors::Error,
    event::{Metric, MetricValue},
};

use tokio::io::AsyncBufReadExt;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct NetstatConfig {
    #[serde(default = "default_fields")]
    #[serde(deserialize_with = "deserialize_regex", serialize_with = "serialize_regex")]
    fields: regex::Regex,
}

impl Default for NetstatConfig {
    fn default() -> Self {
        Self {
            fields: default_fields()
        }
    }
}

fn default_fields() -> Regex {
    Regex::new("^(.*_(InErrors|InErrs)|Ip_Forwarding|Ip(6|Ext)_(InOctets|OutOctets)|Icmp6?_(InMsgs|OutMsgs)|TcpExt_(Listen.*|Syncookies.*|TCPSynRetrans)|Tcp_(ActiveOpens|InSegs|OutSegs|OutRsts|PassiveOpens|RetransSegs|CurrEstab)|Udp6?_(InDatagrams|OutDatagrams|NoPorts|RcvbufErrors|SndbufErrors))$").unwrap()
}

pub async fn gather(conf: &NetstatConfig, proc_path: &str) -> Result<Vec<Metric>, ()> {
    let path = format!("{}/net/netstat", proc_path);
    let mut net_stats = get_net_stats(&path).await
        .map_err(|err| {
            warn!("read netstat failed, {}", err);
        })?;

    let path = format!("{}/net/snmp", proc_path);
    let snmp_stats = get_net_stats(&path).await
        .map_err(|err| {
            warn!("read snmp failed, {}", err);
        })?;

    let path = format!("{}/net/snmp6", proc_path);
    let snmp6_stats = get_snmp6_stats(&path).await
        .map_err(|err| {
            warn!("read snmp6 failed, {}", err);
        })?;

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
                _ => continue
            };

            if !conf.fields.is_match(&key) {
                continue;
            }


            metrics.push(gauge_metric!(
                format!("node_netstat_{}", key),
                format!("Statistic {}{}", protocol, name),
                v
            ));
        }
    }

    Ok(metrics)
}

async fn get_net_stats(path: &str) -> Result<BTreeMap<String, BTreeMap<String, String>>, Error> {
    let f = tokio::fs::File::open(path).await
        .map_err(Error::from)?;
    let r = tokio::io::BufReader::new(f);
    let mut lines = r.lines();
    let mut stats: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();

    while let Some(line) = lines.next_line().await.map_err(Error::from)? {
        let names = line.split_ascii_whitespace()
            .collect::<Vec<_>>();

        let line = match lines.next_line().await.map_err(Error::from)? {
            Some(line) => line,
            None => break
        };
        let values = line
            .split_ascii_whitespace()
            .collect::<Vec<_>>();

        // remove trailing :
        let protocol = names[0].strip_suffix(":").unwrap();
        stats.insert(protocol.to_string(), BTreeMap::new());
        if names.len() != values.len() {
            println!("names: {} {:?}", names.len(), names);
            println!("values: {} {:?}", values.len(), values);

            return Err(Error::new_invalid_with_message("mismatch field count"));
        }

        for i in 0..names.len() {
            let props = stats.get_mut(protocol).unwrap();

            props.insert(names[i].to_string(), values[i].to_string());
        }
    }

    Ok(stats)
}

async fn get_snmp6_stats(path: &str) -> Result<BTreeMap<String, BTreeMap<String, String>>, Error> {
    let f = tokio::fs::File::open(path).await
        .map_err(Error::from)?;
    let r = tokio::io::BufReader::new(f);
    let mut lines = r.lines();
    let mut stats: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();

    while let Some(line) = lines.next_line().await.map_err(Error::from)? {
        let stat = line.split_ascii_whitespace()
            .collect::<Vec<_>>();
        if stat.len() < 2 {
            continue;
        }

        // Expect to have 6 in metric name, skip line otherwise
        let index = match stat[0].find('6') {
            Some(i) => i,
            _ => continue
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
        let path = "testdata/proc/net/netstat";

        let stats = get_net_stats(path).await.unwrap();

        let props = stats.get("TcpExt").unwrap();
        assert_eq!(props.get("DelayedACKs").unwrap(), "102471");

        let props = stats.get("IpExt").unwrap();
        assert_eq!(props.get("OutOctets").unwrap(), "2786264347");
    }

    #[tokio::test]
    async fn test_snmp_stats() {
        let path = "testdata/proc/net/snmp";
        let stats = get_net_stats(path).await.unwrap();

        let props = stats.get("Udp").unwrap();
        assert_eq!(props.get("RcvbufErrors").unwrap(), "9");

        let props = stats.get("Udp").unwrap();
        assert_eq!(props.get("SndbufErrors").unwrap(), "8");
    }

    #[tokio::test]
    async fn test_snmp6_stats() {
        let path = "testdata/proc/net/snmp6";
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