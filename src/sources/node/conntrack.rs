use std::path::PathBuf;

use event::Metric;
use tokio::io::AsyncBufReadExt;

use super::{read_into, Error};

/// Shows conntrack statistics (does nothing if no `/proc/sys/net/netfilter/` present)
///
/// Maybe we can fetch conntrack statistics from netlink api
/// https://github.com/torvalds/linux/blob/master/net/netfilter/nf_conntrack_netlink.c
/// https://github.com/ti-mo/conntrack/blob/5b022d74eb6f79d2ddbddd0100e93b3aeeadfff8/conn.go#L465
pub async fn gather(proc_path: &str) -> Result<Vec<Metric>, Error> {
    let mut path = PathBuf::from(proc_path);
    path.push("sys/net/netfilter/nf_conntrack_count");
    let count = read_into::<_, u64, _>(path).await?;

    let mut path = PathBuf::from(proc_path);
    path.push("sys/net/netfilter/nf_conntrack_max");
    let max = read_into::<_, u64, _>(path).await?;

    let stats = get_conntrack_statistics(proc_path).await?;

    let statistic = stats
        .iter()
        .fold(ConntrackStatEntry::default(), |mut stat, ent| {
            stat.found += ent.found;
            stat.invalid += ent.invalid;
            stat.ignore += ent.ignore;
            stat.insert += ent.insert;
            stat.insert_failed += ent.insert_failed;
            stat.drop += ent.drop;
            stat.early_drop += ent.early_drop;
            stat.search_restart += ent.search_restart;

            stat
        });

    Ok(vec![
        Metric::gauge(
            "node_nf_conntrack_entries",
            "Number of currently allocated flow entries for connection tracking.",
            count as f64,
        ),
        Metric::gauge(
            "node_nf_conntrack_entries_limit",
            "Maximum size of connection tracking table.",
            max as f64,
        ),
        Metric::gauge(
            "node_nf_conntrack_stat_found",
            "Number of searched entries which were successful.",
            statistic.found as f64,
        ),
        Metric::gauge(
            "node_nf_conntrack_stat_invalid",
            "Number of packets seen which can not be tracked.",
            statistic.invalid as f64,
        ),
        Metric::gauge(
            "node_nf_conntrack_stat_ignore",
            "Number of packets seen which are already connected to a conntrack entry.",
            statistic.ignore as f64
        ),
        Metric::gauge(
            "node_nf_conntrack_stat_insert",
            "Number of entries inserted into the list.",
            statistic.insert,
        ),
        Metric::gauge(
            "node_nf_conntrack_stat_insert_failed",
            "Number of entries for which list insertion was attempted but failed.",
            statistic.insert_failed,
        ),
        Metric::gauge(
            "node_nf_conntrack_stat_drop",
            "Number of packets dropped due to conntrack failure.",
            statistic.drop,
        ),
        Metric::gauge(
            "node_nf_conntrack_stat_early_drop",
            "Number of dropped conntrack entries to make room for new ones, if maximum table size was reached.",
            statistic.early_drop,
        ),
        Metric::gauge(
            "node_nf_conntrack_stat_search_restart",
            "Number of conntrack table lookups which had to be restarted due to hashtable resizes.",
            statistic.search_restart,
        ),
    ])
}

// A ConntrackStatEntry represents one line from net/stat/nf_conntrack
// and contains netfilter conntrack statistics at one CPU core
#[allow(dead_code)]
#[derive(Debug, Default)]
struct ConntrackStatEntry {
    entries: u64,
    found: u64,
    invalid: u64,
    ignore: u64,
    insert: u64,
    insert_failed: u64,
    drop: u64,
    early_drop: u64,
    search_restart: u64,
}

#[inline]
fn hex_u64(input: &[u8]) -> Result<u64, Error> {
    let res = input
        .iter()
        .rev()
        .enumerate()
        .map(|(k, &v)| {
            let digit = v as char;
            (digit.to_digit(16).unwrap_or(0) as u64) << (k * 4)
        })
        .sum();

    Ok(res)
}

impl ConntrackStatEntry {
    fn new(line: &str) -> Result<Self, Error> {
        let parts = line.split_ascii_whitespace().collect::<Vec<_>>();
        if parts.len() != 17 {
            return Err(Error::from("No processor were found"));
        }

        let entries = hex_u64(parts[0].as_bytes())?;
        let found = hex_u64(parts[2].as_bytes())?;
        let invalid = hex_u64(parts[4].as_bytes())?;
        let ignore = hex_u64(parts[5].as_bytes())?;
        let insert = hex_u64(parts[8].as_bytes())?;
        let insert_failed = hex_u64(parts[9].as_bytes())?;
        let drop = hex_u64(parts[10].as_bytes())?;
        let early_drop = hex_u64(parts[11].as_bytes())?;
        let search_restart = hex_u64(parts[16].as_bytes())?;

        Ok(Self {
            entries,
            found,
            invalid,
            ignore,
            insert,
            insert_failed,
            drop,
            early_drop,
            search_restart,
        })
    }
}

async fn get_conntrack_statistics(proc_path: &str) -> Result<Vec<ConntrackStatEntry>, Error> {
    let path = format!("{}/net/stat/nf_conntrack", proc_path);
    let f = tokio::fs::File::open(path).await?;
    let r = tokio::io::BufReader::new(f);
    let mut lines = r.lines();

    let mut first = true;
    let mut stats = Vec::new();

    while let Some(line) = lines.next_line().await? {
        if first {
            first = false;
            continue;
        }

        if let Ok(ent) = ConntrackStatEntry::new(&line) {
            stats.push(ent);
        }
    }

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conntrack_new() {
        let line = "0000000a  00000000 00000000 00000000 00000005 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000  00000000 00000000 00000000 00000004";
        let ent = ConntrackStatEntry::new(line).unwrap();

        assert_eq!(ent.search_restart, 4)
    }

    #[test]
    fn test_hex_u64() {
        let v = hex_u64(b"0000000a").unwrap();
        assert_eq!(v, 10u64)
    }
}
