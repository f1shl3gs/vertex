use std::path::Path;

use event::Metric;

use super::{Error, Paths, read_file_no_stat, read_into};

/// Shows conntrack statistics (does nothing if no `/proc/sys/net/netfilter/` present)
///
/// Maybe we can fetch conntrack statistics from netlink api
/// https://github.com/torvalds/linux/blob/master/net/netfilter/nf_conntrack_netlink.c
/// https://github.com/ti-mo/conntrack/blob/5b022d74eb6f79d2ddbddd0100e93b3aeeadfff8/conn.go#L465
pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let root = paths.proc().join("sys/net/netfilter");

    let count = read_into::<_, u64, _>(root.join("nf_conntrack_count"))?;
    let max = read_into::<_, u64, _>(root.join("nf_conntrack_max"))?;

    let stats = get_conntrack_statistics(paths.proc())?;

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
            count,
        ),
        Metric::gauge(
            "node_nf_conntrack_entries_limit",
            "Maximum size of connection tracking table.",
            max,
        ),
        Metric::gauge(
            "node_nf_conntrack_stat_found",
            "Number of searched entries which were successful.",
            statistic.found,
        ),
        Metric::gauge(
            "node_nf_conntrack_stat_invalid",
            "Number of packets seen which can not be tracked.",
            statistic.invalid,
        ),
        Metric::gauge(
            "node_nf_conntrack_stat_ignore",
            "Number of packets seen which are already connected to a conntrack entry.",
            statistic.ignore,
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

/// A ConntrackStatEntry represents one line from net/stat/nf_conntrack
/// and contains netfilter conntrack statistics at one CPU core
#[cfg_attr(test, derive(PartialEq))]
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

fn parse_stat_entry(line: &str) -> Result<ConntrackStatEntry, Error> {
    let parts = line.split_ascii_whitespace().take(17).collect::<Vec<_>>();
    if parts.len() != 17 {
        return Err(Error::Malformed("conntrack stat entry"));
    }

    let entries = u64::from_str_radix(parts[0], 16)?;
    let found = u64::from_str_radix(parts[2], 16)?;
    let invalid = u64::from_str_radix(parts[4], 16)?;
    let ignore = u64::from_str_radix(parts[5], 16)?;
    let insert = u64::from_str_radix(parts[8], 16)?;
    let insert_failed = u64::from_str_radix(parts[9], 16)?;
    let drop = u64::from_str_radix(parts[10], 16)?;
    let early_drop = u64::from_str_radix(parts[11], 16)?;
    let search_restart = u64::from_str_radix(parts[16], 16)?;

    Ok(ConntrackStatEntry {
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

fn get_conntrack_statistics(root: &Path) -> Result<Vec<ConntrackStatEntry>, Error> {
    let content = read_file_no_stat(root.join("net/stat/nf_conntrack"))?;

    let mut entries = Vec::new();
    for line in content.lines().skip(1) {
        if let Ok(entry) = parse_stat_entry(line) {
            entries.push(entry);
        }
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn smoke() {
        let paths = Paths::test();
        let metrics = collect(paths).await.unwrap();
        assert_ne!(metrics.len(), 0);
    }

    #[test]
    fn parse() {
        let content = r#"entries  searched found new invalid ignore delete delete_list insert insert_failed drop early_drop icmp_error  expect_new expect_create expect_delete search_restart
00000021  00000000 00000000 00000000 00000003 0000588a 00000000 00000000 00000000 00000000 00000000 00000000 00000000  00000000 00000000 00000000 00000000
00000021  00000000 00000000 00000000 00000002 000056a4 00000000 00000000 00000000 00000000 00000000 00000000 00000000  00000000 00000000 00000000 00000002
00000021  00000000 00000000 00000000 00000001 000058d4 00000000 00000000 00000000 00000000 00000000 00000000 00000000  00000000 00000000 00000000 00000001
00000021  00000000 00000000 00000000 0000002f 00005688 00000000 00000000 00000000 00000000 00000000 00000000 00000000  00000000 00000000 00000000 00000004
"#;
        let mut entries = Vec::new();
        for line in content.lines().skip(1) {
            entries.push(parse_stat_entry(line).unwrap());
        }

        assert_eq!(entries.len(), 4);
        assert_eq!(
            vec![
                ConntrackStatEntry {
                    entries: 33,
                    found: 0,
                    invalid: 3,
                    ignore: 22666,
                    insert: 0,
                    insert_failed: 0,
                    drop: 0,
                    early_drop: 0,
                    search_restart: 0,
                },
                ConntrackStatEntry {
                    entries: 33,
                    found: 0,
                    invalid: 2,
                    ignore: 22180,
                    insert: 0,
                    insert_failed: 0,
                    drop: 0,
                    early_drop: 0,
                    search_restart: 2,
                },
                ConntrackStatEntry {
                    entries: 33,
                    found: 0,
                    invalid: 1,
                    ignore: 22740,
                    insert: 0,
                    insert_failed: 0,
                    drop: 0,
                    early_drop: 0,
                    search_restart: 1,
                },
                ConntrackStatEntry {
                    entries: 33,
                    found: 0,
                    invalid: 47,
                    ignore: 22152,
                    insert: 0,
                    insert_failed: 0,
                    drop: 0,
                    early_drop: 0,
                    search_restart: 4,
                },
            ],
            entries
        );
    }
}
