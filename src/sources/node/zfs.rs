//! Exposes ZFS performance statistics

use std::collections::BTreeMap;
use std::num::ParseIntError;
use std::path::{Path, PathBuf};

use event::{Metric, tags};

use super::{Error, Paths, read_string};

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let mut metrics = Vec::new();

    for (subsystem, filename) in [
        ("abd", "abdstats"),
        ("arc", "arcstats"),
        ("dbuf", "dbufstats"),
        ("dmu_tx", "dmu_tx"),
        ("dnode", "dnodestats"),
        ("fm", "fm"),
        // vdev_cache is deprecated
        ("vdev_cache", "vdev_cache_stats"),
        ("vdev_mirror", "vdev_mirror_stats"),
        // no known consumers of the XUIO interface on Linux exist
        ("xuio", "xuio_stats"),
        ("zfetch", "zfetchstats"),
        ("zil", "zil"),
    ] {
        let path = paths.proc().join("spl/kstat/zfs").join(filename);
        let content = std::fs::read_to_string(path)?;

        for (key, value) in parse_stat_file(&content)? {
            metrics.push(Metric::gauge(
                format!("node_zfs_{}_{}", subsystem, key.replace("-", "_")),
                format!("kstat.zfs.misc.{}.{}", filename, key),
                value,
            ));
        }
    }

    // pool stats
    let pattern = format!("{}/spl/kstat/zfs/*/io", paths.proc().to_string_lossy());
    let matched = glob::glob(&pattern)?;
    for path in matched.flatten() {
        let pool_name = path
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy();

        let content = std::fs::read_to_string(&path)?;
        for (key, value) in parse_pool_stats(&content)? {
            metrics.push(Metric::gauge_with_tags(
                format!("node_zfs_zpool_{key}"),
                format!("kstat.zfs.misc.io.{key}"),
                value,
                tags! {
                    "zpool" => pool_name.as_ref()
                },
            ))
        }
    }

    let pattern = format!(
        "{}/spl/kstat/zfs/*/objset-*",
        paths.proc().to_string_lossy()
    );
    let matched = glob::glob(&pattern)?;
    for path in matched.flatten() {
        let kvs = parse_pool_objset_file(path)?;
        for ((key, pool, dataset), value) in kvs {
            metrics.push(Metric::gauge_with_tags(
                format!("node_zfs_zpool_dataset_{key}"),
                format!("kstat.zfs.misc.objset.{key}"),
                value,
                tags!(
                    "zpool" => pool,
                    "dataset" => dataset
                ),
            ));
        }
    }

    let pattern = format!("{}/spl/kstat/zfs/*/state", paths.proc().to_string_lossy());
    let paths = glob::glob(&pattern)?;
    for path in paths.flatten() {
        let pool_name = path
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy();
        let kvs = parse_pool_state_file(&path)?;
        for (key, value) in kvs {
            metrics.push(Metric::gauge_with_tags(
                "node_zfs_zpool_state",
                "kstat.zfs.misc.state",
                value,
                tags!(
                    "zpool" => pool_name.as_ref(),
                    "state" => key
                ),
            ))
        }
    }

    Ok(metrics)
}

fn parse_stat_file(content: &str) -> Result<BTreeMap<&str, f64>, ParseIntError> {
    let mut kvs = BTreeMap::new();
    let mut parse = false;
    for line in content.lines() {
        let fields = line.split_ascii_whitespace().collect::<Vec<_>>();

        if !parse
            && fields.len() == 3
            && fields[0] == "name"
            && fields[1] == "type"
            && fields[2] == "data"
        {
            // start parsing from here.
            parse = true;
            continue;
        }

        // kstat data type (column 2) should be KSTAT_DATA_UINT64, otherwise ignore
        // TODO: when other KSTAT_DATA_* types arrive, much of this will need to be restructured
        let value = match fields[1] {
            "3" => fields[2].parse::<i64>()? as f64,
            "4" => fields[2].parse::<u64>()? as f64,
            _ => continue,
        };

        kvs.insert(fields[0], value);
    }

    Ok(kvs)
}

fn parse_pool_stats(content: &str) -> Result<BTreeMap<&str, u64>, Error> {
    let mut kvs = BTreeMap::new();
    let mut parse = false;
    let mut headers = vec![];
    for line in content.lines() {
        let fields = line.trim().split_ascii_whitespace().collect::<Vec<_>>();

        if !parse && fields.len() >= 12 && fields[0] == "nread" {
            // start parsing from here
            parse = true;
            for field in fields {
                headers.push(field);
            }
            continue;
        }

        if !parse {
            continue;
        }

        for (index, field) in headers.iter().enumerate() {
            let value = fields[index].parse().unwrap_or(0u64);
            kvs.insert(*field, value);
        }
    }

    Ok(kvs)
}

fn parse_pool_objset_file(path: PathBuf) -> Result<BTreeMap<(String, String, String), u64>, Error> {
    let content = std::fs::read_to_string(&path)?;

    let mut kvs = BTreeMap::new();
    let mut parse = false;
    let mut pool = String::new();
    let mut dataset = String::new();
    for line in content.lines() {
        let parts = line.split_ascii_whitespace().collect::<Vec<_>>();
        if !parse && parts[0] == "name" && parts[1] == "type" && parts[2] == "data" {
            parse = true;
            continue;
        }

        if !parse || parts.len() < 3 {
            continue;
        }

        if parts[0] == "dataset_name" {
            let path = path.to_string_lossy();
            let elmts = path.split('/').collect::<Vec<_>>();
            let length = elmts.len();
            pool = elmts[length - 2].to_string();
            dataset = match line.find(parts[2]) {
                Some(index) => line[index..].to_string(),
                None => return Err(Error::Malformed("pool objset line")),
            };
            continue;
        }

        if parts[1] == "4" {
            let value = parts[2].parse::<u64>()?;
            kvs.insert((parts[0].to_string(), pool.clone(), dataset.clone()), value);
        }
    }

    Ok(kvs)
}

fn parse_pool_state_file(path: &Path) -> Result<BTreeMap<&'static str, bool>, Error> {
    const STATS: [&str; 7] = [
        "online",
        "degraded",
        "faulted",
        "offline",
        "removed",
        "unavail",
        "suspended",
    ];

    let actual_state = read_string(path)?.to_lowercase();

    let mut kvs = BTreeMap::new();
    for stat in STATS {
        kvs.insert(stat, actual_state == stat);
    }

    Ok(kvs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glob::glob;

    #[test]
    fn pool_procfs_file() {
        let paths = glob("tests/node/fixtures/proc/spl/kstat/zfs/*/io").unwrap();
        let mut parsed = 0;
        for path in paths.flatten() {
            parsed += 1;

            let content = std::fs::read_to_string(path).unwrap();
            let kvs = parse_pool_stats(&content).unwrap();
            assert_ne!(kvs.len(), 0);

            for (k, v) in kvs {
                if k != "kstat.zfs.misc.io.nread" {
                    continue;
                }

                if v != 1884160 && v != 2826240 {
                    panic!("incorrect value parsed from procfs data")
                }
            }
        }

        assert_eq!(parsed, 2);
    }

    #[test]
    fn pool_objset_file() {
        let paths = glob("tests/node/fixtures/proc/spl/kstat/zfs/*/objset-*").unwrap();
        for path in paths.flatten() {
            let kvs = parse_pool_objset_file(path).unwrap();

            assert_ne!(kvs.len(), 0);
            for ((key, _pool, _write), v) in kvs {
                if key != "writes" {
                    continue;
                }

                if v != 0u64 && v != 4u64 && v != 10u64 {
                    panic!("incorrect value parsed from procfs data")
                }
            }
        }
    }

    #[test]
    fn pool_state_file() {
        let paths = glob("tests/node/fixtures/proc/spl/kstat/zfs/*/state").unwrap();
        let mut handled = 0;
        for path in paths.flatten() {
            handled += 1;
            let pool_name = path
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_string_lossy();
            let kvs = parse_pool_state_file(&path).unwrap();
            assert_ne!(kvs.len(), 0);

            for (state, active) in kvs {
                if pool_name == "pool1" {
                    if !active && state == "online" {
                        panic!("incorrect parsed value for online state")
                    }

                    if active && state != "online" {
                        panic!("incorrect parsed value for online state")
                    }
                }

                if pool_name == "poolz1" {
                    if !active && state == "degraded" {
                        panic!("incorrect parsed value for degraded state")
                    }

                    if active && state != "degraded" {
                        panic!("incorrect parsed value for degraded state")
                    }
                }
            }
        }

        assert_eq!(handled, 2);
    }
}
