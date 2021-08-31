/// Shows conntrack statistics (does nothing if no `/proc/sys/net/netfilter/` present)
///
/// Maybe we can fetch conntrack statistics from netlink api
/// https://github.com/torvalds/linux/blob/master/net/netfilter/nf_conntrack_netlink.c
/// https://github.com/ti-mo/conntrack/blob/5b022d74eb6f79d2ddbddd0100e93b3aeeadfff8/conn.go#L465

use std::{
    sync::Arc,
    io,
};

use crate::{
    tags,
    gauge_metric,
    event::{Metric, MetricValue},
    sources::node::{
        errors::Error,
        read_to_string, read_to_f64,
    },
};
use std::{
    path::PathBuf,
    collections::BTreeMap,
};
use std::option::Option::Some;

pub async fn gather(proc_path: &str) -> Result<Vec<Metric>, ()> {
    let mut path = PathBuf::from(proc_path);
    path.push("sys/net/netfilter/nf_conntrack_count");
    let count = read_to_f64(path).await.map_err(|err| {
        if !err.is_not_found() {
            warn!("read conntrack count failed"; "err" => err);
        }
    })?;

    let mut path = PathBuf::from(proc_path);
    path.push("sys/net/netfilter/nf_conntrack_max");
    let max = read_to_f64(path).await.map_err(|err| {
        if !err.is_not_found() {
            warn!("read conntrack max failed"; "err" => err);
        }
    })?;

    let stats = get_conntrack_statistics(&proc_path).await.
        map_err(|err| {
            warn!("get conntrack statistics failed"; "err" => err);
        })?;

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
        gauge_metric!(
            "node_nf_conntrack_entries",
            "Number of currently allocated flow entries for connection tracking.",
            count,
        ),
        gauge_metric!(
            "node_nf_conntrack_entries_limit",
            "Maximum size of connection tracking table.",
            max,
        ),
        gauge_metric!(
            "node_nf_conntrack_stat_found",
            "Number of searched entries which were successful.",
            statistic.found as f64,
        ),
        gauge_metric!(
            "node_nf_conntrack_stat_invalid",
            "Number of packets seen which can not be tracked.",
            statistic.invalid as f64,
        ),
        gauge_metric!(
            "node_nf_conntrack_stat_ignore",
            "Number of packets seen which are already connected to a conntrack entry.",
            statistic.ignore as f64
        ),
        gauge_metric!(
            "node_nf_conntrack_stat_insert",
            "Number of entries inserted into the list.",
            statistic.insert as f64
        ),
        gauge_metric!(
            "node_nf_conntrack_stat_insert_failed",
            "Number of entries for which list insertion was attempted but failed.",
            statistic.insert_failed as f64
        ),
        gauge_metric!(
            "node_nf_conntrack_stat_drop",
            "Number of packets dropped due to conntrack failure.",
            statistic.drop as f64
        ),
        gauge_metric!(
            "node_nf_conntrack_stat_early_drop",
            "Number of dropped conntrack entries to make room for new ones, if maximum table size was reached.",
            statistic.early_drop as f64
        ),
        gauge_metric!(
            "node_nf_conntrack_stat_search_restart",
            "Number of conntrack table lookups which had to be restarted due to hashtable resizes.",
            statistic.search_restart as f64,
        ),
    ])
}

// A ConntrackStatEntry represents one line from net/stat/nf_conntrack
// and contains netfilter conntrack statistics at one CPU core
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
    let res = input.iter().rev().enumerate().map(|(k, &v)| {
        let digit = v as char;
        (digit.to_digit(16).unwrap_or(0) as u64) << (k * 4)
    }).sum();

    Ok(res)
}

impl ConntrackStatEntry {
    fn new(line: &str) -> Result<Self, Error> {
        let parts = line.split_ascii_whitespace().collect::<Vec<_>>();
        if parts.len() != 17 {
            let inner = io::Error::from(io::ErrorKind::InvalidData);
            return Err(Error::from(inner).with_message("No processors were found"));
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
    let mut path = PathBuf::from(proc_path);
    path.push("net/stat/nf_conntrack");

    let content = read_to_string(path).await
        .map_err(|err| Error::from(err))?;

    let mut stats = Vec::new();
    let mut lines = content.lines().skip(1);
    while let Some(line) = lines.next() {
        if let Ok(ent) = ConntrackStatEntry::new(line) {
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