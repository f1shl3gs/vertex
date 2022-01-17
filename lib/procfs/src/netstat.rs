use crate::{Error, ProcFS};
use std::collections::BTreeMap;
use std::path::PathBuf;
use tokio::io::AsyncBufReadExt;

impl ProcFS {
    pub async fn netstats(&self) -> Result<BTreeMap<String, BTreeMap<String, String>>, Error> {
        let path = self.root.join("net/netstat");
        let mut net_stats = get_net_stats(path).await?;
        let path = self.root.join("net/snmp");
        let snmp_stats = get_net_stats(path).await?;
        let path = self.root.join("net/snmp6");
        let snmp6_stats = get_snmp6_stats(path).await?;

        // Merge the results of snmpStats into netStats (collisions are possible,
        // but we know that the keys are always unique for the give use case.
        for (k, v) in snmp_stats {
            net_stats.insert(k, v);
        }
        for (k, v) in snmp6_stats {
            net_stats.insert(k, v);
        }

        Ok(net_stats)
    }
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
        let protocol = names[0].strip_suffix(":").unwrap();
        stats.insert(protocol.to_string(), BTreeMap::new());
        if names.len() != values.len() {
            return Err(Error::invalid_data("mismatch field count"));
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
        let path = "fixtures/proc/net/netstat".into();

        let stats = get_net_stats(path).await.unwrap();

        let props = stats.get("TcpExt").unwrap();
        assert_eq!(props.get("DelayedACKs").unwrap(), "102471");

        let props = stats.get("IpExt").unwrap();
        assert_eq!(props.get("OutOctets").unwrap(), "2786264347");
    }

    #[tokio::test]
    async fn test_snmp_stats() {
        let path = "fixtures/proc/net/snmp".into();
        let stats = get_net_stats(path).await.unwrap();

        let props = stats.get("Udp").unwrap();
        assert_eq!(props.get("RcvbufErrors").unwrap(), "9");

        let props = stats.get("Udp").unwrap();
        assert_eq!(props.get("SndbufErrors").unwrap(), "8");
    }

    #[tokio::test]
    async fn test_snmp6_stats() {
        let path = "fixtures/proc/net/snmp6".into();
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
