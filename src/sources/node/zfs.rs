//! Exposes ZFS performance statistics

use std::collections::BTreeMap;
use std::path::PathBuf;

use event::{Metric, tags};

use super::{Error, read_string};

macro_rules! parse_subsystem_metrics {
    ($metrics: expr, $root: expr, $subsystem: expr, $path: expr) => {
        let path = $root.join("spl/kstat/zfs").join($path);
        for (k, v) in parse_procfs_file(path)? {
            $metrics.push(Metric::gauge(
                format!("node_{}_{}", $subsystem, k.replace("-", "_")),
                format!("kstat.zfs.misc.{}.{}", $path, k),
                v,
            ));
        }
    };
}

pub async fn gather(proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let mut metrics = Vec::new();

    parse_subsystem_metrics!(metrics, proc_path, "zfs_abd", "abdstats");
    parse_subsystem_metrics!(metrics, proc_path, "zfs_arc", "arcstats");
    parse_subsystem_metrics!(metrics, proc_path, "zfs_dbuf", "dbufstats");
    parse_subsystem_metrics!(metrics, proc_path, "zfs_dmu_tx", "dmu_tx");
    parse_subsystem_metrics!(metrics, proc_path, "zfs_dnode", "dnodestats");
    parse_subsystem_metrics!(metrics, proc_path, "zfs_fm", "fm");
    // vdev_cache is deprecated
    parse_subsystem_metrics!(metrics, proc_path, "zfs_vdev_cache", "vdev_cache_stats");
    parse_subsystem_metrics!(metrics, proc_path, "zfs_vdev_mirror", "vdev_mirror_stats");
    // no known consumers of the XUIO interface on Linux exist
    parse_subsystem_metrics!(metrics, proc_path, "zfs_xuio", "xuio_stats");
    parse_subsystem_metrics!(metrics, proc_path, "zfs_zfetch", "zfetchstats");
    parse_subsystem_metrics!(metrics, proc_path, "zfs_zil", "zil");

    // pool stats
    let pattern = format!("{}/spl/kstat/zfs/*/io", proc_path.to_string_lossy());
    let paths = glob::glob(&pattern)?;

    for path in paths.flatten() {
        let path = path.to_str().unwrap();
        let pool_name = parse_pool_name(path)?;
        let kvs = parse_pool_procfs_file(path)?;
        for (key, value) in kvs {
            metrics.push(Metric::gauge_with_tags(
                format!("node_zfs_zpool_{key}"),
                format!("kstat.zfs.misc.io.{key}"),
                value,
                tags! {
                    "zpool" => pool_name
                },
            ))
        }
    }

    let pattern = format!("{}/spl/kstat/zfs/*/objset-*", proc_path.to_string_lossy());
    let paths = glob::glob(&pattern)?;
    for path in paths.flatten() {
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

    let pattern = format!("{}/spl/kstat/zfs/*/state", proc_path.to_string_lossy());
    let paths = glob::glob(&pattern)?;
    for path in paths.flatten() {
        let path = path.to_string_lossy();
        let pool_name = parse_pool_name(path.as_ref())?;
        let kvs = parse_pool_state_file(path.as_ref())?;
        for (key, value) in kvs {
            metrics.push(Metric::gauge_with_tags(
                "node_zfs_zpool_state",
                "kstat.zfs.misc.state",
                value,
                tags!(
                    "zpool" => pool_name,
                    "state" => key
                ),
            ))
        }
    }

    Ok(metrics)
}

fn parse_procfs_file(path: PathBuf) -> Result<BTreeMap<String, i64>, Error> {
    let data = std::fs::read_to_string(path)?;

    let mut kvs = BTreeMap::new();
    let mut parse = false;
    for line in data.lines() {
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
        if fields[1] == "3" || fields[1] == "4" {
            let key = fields[0].to_string();
            let value = fields[2].parse().unwrap_or(0i64);
            kvs.insert(key, value);
        }
    }

    Ok(kvs)
}

fn parse_pool_procfs_file(path: &str) -> Result<BTreeMap<String, u64>, Error> {
    let length = path.split('/').count();
    if length < 2 {
        return Err(Error::from(
            "zpool path did not return at least two elements",
        ));
    }

    let data = std::fs::read_to_string(path)?;

    let mut kvs = BTreeMap::new();
    let mut parse = false;
    let mut headers = vec![];
    for line in data.lines() {
        let fields = line.trim().split_ascii_whitespace().collect::<Vec<_>>();

        if !parse && fields.len() >= 12 && fields[0] == "nread" {
            // start parsing from here
            parse = true;
            for field in fields {
                headers.push(field.to_string());
            }
            continue;
        }

        if !parse {
            continue;
        }

        for (i, field) in headers.iter().enumerate() {
            let key = field.clone();
            let value = fields[i].parse().unwrap_or(0u64);
            kvs.insert(key, value);
        }
    }

    Ok(kvs)
}

fn parse_pool_objset_file(path: PathBuf) -> Result<BTreeMap<(String, String, String), u64>, Error> {
    let data = std::fs::read_to_string(&path)?;

    let mut kvs = BTreeMap::new();
    let mut parse = false;
    let mut pool = String::new();
    let mut dataset = String::new();
    for line in data.lines() {
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

const STATS: [&str; 7] = [
    "online",
    "degraded",
    "faulted",
    "offline",
    "removed",
    "unavail",
    "suspended",
];

fn parse_pool_state_file(path: &str) -> Result<BTreeMap<&'static str, bool>, Error> {
    let actual_state = read_string(path)?.to_lowercase();

    let mut kvs = BTreeMap::new();
    for stat in STATS {
        let active = actual_state == stat;

        kvs.insert(stat, active);
    }

    Ok(kvs)
}

fn parse_pool_name(path: &str) -> Result<&str, Error> {
    let elements = path.split('/').collect::<Vec<_>>();
    let length = elements.len();
    if length < 2 {
        return Err("zpool path did not return at least two elements".into());
    }

    Ok(elements[length - 2])
}

#[cfg(test)]
mod tests {
    use super::*;
    use glob::glob;

    #[test]
    fn pool_procfs_file() {
        let paths = glob("tests/node/proc/spl/kstat/zfs/*/io").unwrap();
        let mut parsed = 0;
        for path in paths.flatten() {
            let path = path.to_str().unwrap();
            parsed += 1;
            let kvs = parse_pool_procfs_file(path).unwrap();
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
        let paths = glob("tests/node/proc/spl/kstat/zfs/*/objset-*").unwrap();
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
        let paths = glob("tests/node/proc/spl/kstat/zfs/*/state").unwrap();
        let mut handled = 0;
        for path in paths.flatten() {
            handled += 1;
            let path: &str = path.to_str().unwrap();
            let pool_name = parse_pool_name(path).unwrap();
            let kvs = parse_pool_state_file(path).unwrap();
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
