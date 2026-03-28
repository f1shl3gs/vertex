//! Expose metrics from /proc/net/stat
//!
//! https://github.com/prometheus/node_exporter/pull/1771

use event::{Metric, tags};

use super::{Error, Paths, read_file_no_stat};

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let mut metrics = Vec::new();

    for entry in paths.proc().join("net/stat").read_dir()?.flatten() {
        let Ok(typ) = entry.file_type() else { continue };
        if !typ.is_file() {
            continue;
        }

        let filename = entry.file_name();
        let subsystem = filename.to_string_lossy();

        let content = read_file_no_stat(entry.path())?;
        let stats = parse_net_stat(&content)?;

        for (key, stats) in stats {
            for (cpu, value) in stats.into_iter().enumerate() {
                metrics.push(Metric::sum_with_tags(
                    format!("node_lnstat_{}_total", key),
                    "linux network cache stats",
                    value,
                    tags!(
                        "cpu" => cpu,
                        "subsystem" => subsystem.as_ref(),
                    ),
                ))
            }
        }
    }

    Ok(metrics)
}

fn parse_net_stat(content: &str) -> Result<Vec<(&str, Vec<u64>)>, Error> {
    let mut lines = content.lines();

    let Some(line) = lines.next() else {
        return Err(Error::NoData);
    };

    let mut values = line
        .split_ascii_whitespace()
        .map(|part| (part, Vec::new()))
        .collect::<Vec<_>>();

    for line in lines {
        let parts = line
            .split_ascii_whitespace()
            .map(|p| u64::from_str_radix(p, 16))
            .collect::<Result<Vec<_>, _>>()?;

        for (index, value) in parts.into_iter().enumerate() {
            values[index].1.push(value);
        }
    }

    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arp_cache() {
        let want = vec![
            ("entries", vec![20, 20]),
            ("allocs", vec![1, 13]),
            ("destroys", vec![2, 14]),
            ("hash_grows", vec![3, 15]),
            ("lookups", vec![4, 16]),
            ("hits", vec![5, 17]),
            ("res_failed", vec![6, 18]),
            ("rcv_probes_mcast", vec![7, 19]),
            ("rcv_probes_ucast", vec![8, 20]),
            ("periodic_gc_runs", vec![9, 21]),
            ("forced_gc_runs", vec![10, 22]),
            ("unresolved_discards", vec![11, 23]),
            ("table_fulls", vec![12, 24]),
        ];

        let content =
            std::fs::read_to_string("tests/node/fixtures/proc/net/stat/arp_cache").unwrap();
        let stats = parse_net_stat(&content).unwrap();

        assert_eq!(want, stats)
    }

    #[test]
    fn ndisc_cache() {
        let want = vec![
            ("entries", vec![36, 36]),
            ("allocs", vec![240, 252]),
            ("destroys", vec![241, 253]),
            ("hash_grows", vec![242, 254]),
            ("lookups", vec![243, 255]),
            ("hits", vec![244, 256]),
            ("res_failed", vec![245, 257]),
            ("rcv_probes_mcast", vec![246, 258]),
            ("rcv_probes_ucast", vec![247, 259]),
            ("periodic_gc_runs", vec![248, 260]),
            ("forced_gc_runs", vec![249, 261]),
            ("unresolved_discards", vec![250, 262]),
            ("table_fulls", vec![251, 263]),
        ];

        let content =
            std::fs::read_to_string("tests/node/fixtures/proc/net/stat/ndisc_cache").unwrap();
        let stats = parse_net_stat(&content).unwrap();

        assert_eq!(want, stats)
    }
}
