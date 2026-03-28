//! Exposes ZFS performance statistics

use std::io::ErrorKind;
use std::num::ParseIntError;

use event::{Metric, tags};

use super::{Error, Paths, read_file_no_stat};

const POOL_STATS: [&str; 7] = [
    "online",
    "degraded",
    "faulted",
    "offline",
    "removed",
    "unavail",
    "suspended",
];

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let root = paths.proc().join("spl/kstat/zfs");

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
        let path = root.join(filename);
        match read_file_no_stat(&path) {
            Ok(content) => {
                for (key, value) in parse_stat_file(&content)? {
                    metrics.push(Metric::gauge(
                        format!("node_zfs_{}_{}", subsystem, key.replace("-", "_")),
                        format!("kstat.zfs.misc.{}.{}", filename, key),
                        value,
                    ));
                }
            }
            Err(err) => {
                // file not found error can occur if
                // 1. zfs module is not loaded
                // 2. zfs version does not have the feature with metrics -- ok to ignore
                if err.kind() == ErrorKind::NotFound {
                    // ZFS /proc files are added as new features to ZFS arrive,
                    // it is ok to continue
                    continue;
                }

                debug!(message = "reading zfs subsystem stats failed", ?path, %err);

                return Err(err.into());
            }
        }
    }

    for entry in root.read_dir()?.flatten() {
        let Ok(typ) = entry.file_type() else { continue };
        if !typ.is_dir() {
            continue;
        }

        let filename = entry.file_name();
        let pool = filename.to_string_lossy();
        for entry in entry.path().read_dir()?.flatten() {
            let Ok(typ) = entry.file_type() else { continue };
            if !typ.is_file() {
                continue;
            }

            match entry.file_name().to_string_lossy().as_ref() {
                "io" => {
                    let content = read_file_no_stat(entry.path())?;
                    for (key, value) in parse_pool_stats(&content)? {
                        metrics.push(Metric::gauge_with_tags(
                            format!("node_zfs_zpool_{key}"),
                            format!("kstat.zfs.misc.io.{key}"),
                            value,
                            tags! {
                                "zpool" => pool.as_ref()
                            },
                        ));
                    }
                }
                "state" => {
                    let content = read_file_no_stat(entry.path())?;
                    let actual_state = content.trim();
                    for state in POOL_STATS {
                        metrics.push(Metric::gauge_with_tags(
                            "node_zfs_zpool_state",
                            "kstat.zfs.misc.state",
                            actual_state.eq_ignore_ascii_case(state),
                            tags!(
                                "zpool" => pool.as_ref(),
                                "state" => state
                            ),
                        ));
                    }
                }
                filename => {
                    if filename.starts_with("objset-") {
                        let content = read_file_no_stat(entry.path())?;
                        let (dataset, kvs) = parse_pool_objset(&content)?;
                        for (key, value) in kvs {
                            metrics.push(Metric::gauge_with_tags(
                                format!("node_zfs_zpool_dataset_{key}"),
                                format!("kstat.zfs.misc.objset.{key}"),
                                value,
                                tags!(
                                    "zpool" => pool.as_ref(),
                                    "dataset" => dataset
                                ),
                            ));
                        }
                    }
                }
            }
        }
    }

    Ok(metrics)
}

fn parse_stat_file(content: &str) -> Result<Vec<(&str, f64)>, ParseIntError> {
    let mut kvs = Vec::new();
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

        kvs.push((fields[0], value));
    }

    Ok(kvs)
}

fn parse_pool_stats(content: &str) -> Result<Vec<(&str, u64)>, ParseIntError> {
    let mut kvs = Vec::new();
    let mut parse = false;
    let mut headers = vec![];
    for line in content.lines() {
        let fields = line.split_ascii_whitespace().collect::<Vec<_>>();

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

        kvs.reserve(headers.len());
        for (index, field) in headers.iter().enumerate() {
            let value = fields[index].parse().unwrap_or(0u64);
            kvs.push((*field, value));
        }
    }

    Ok(kvs)
}

fn parse_pool_objset(content: &str) -> Result<(&str, Vec<(&str, u64)>), Error> {
    let mut parse = false;
    let mut dataset = "";
    let mut kvs = Vec::new();
    for line in content.lines() {
        let parts = line.split_ascii_whitespace().take(3).collect::<Vec<_>>();
        if parts.len() < 3 {
            // too short
            continue;
        }

        if !parse && parts[0] == "name" && parts[1] == "type" && parts[2] == "data" {
            parse = true;
            continue;
        }

        if parts[0] == "dataset_name" {
            dataset = match line.find(parts[2]) {
                Some(index) => &line[index..],
                None => return Err(Error::Malformed("pool objset line")),
            };
            continue;
        }

        if parts[1] == "4" {
            let value = parts[2].parse::<u64>()?;
            kvs.push((parts[0], value));
        }
    }

    Ok((dataset, kvs))
}

#[cfg(test)]
mod tests {
    use glob::glob;

    use super::*;

    #[tokio::test]
    async fn smoke() {
        let paths = Paths::test();
        let metrics = collect(paths).await.unwrap();
        println!("{} / {}", metrics.len(), metrics.capacity());
    }

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
            let content = std::fs::read_to_string(path).unwrap();
            let (_dataset, kvs) = parse_pool_objset(&content).unwrap();

            assert_ne!(kvs.len(), 0);
            for (key, value) in kvs {
                if key != "writes" {
                    continue;
                }

                if value != 0u64 && value != 4u64 && value != 10u64 {
                    panic!("incorrect value parsed from procfs data")
                }
            }
        }
    }
}
