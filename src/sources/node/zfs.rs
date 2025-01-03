//! Exposes ZFS performance statistics

use std::collections::BTreeMap;
use std::path::PathBuf;

use event::{tags, Metric};

use super::{read_string, Error};

macro_rules! parse_subsystem_metrics {
    ($metrics: expr, $root: expr, $subsystem: expr, $path: expr) => {
        let path = format!("{}/spl/kstat/zfs/{}", $root, $path);
        for (k, v) in parse_procfs_file(&path).await? {
            let k = k.replace("-", "_");
            $metrics.push(Metric::gauge(format!("node_{}_{}", $subsystem, k), k, v))
        }
    };
}

pub async fn gather(proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let proc_path = proc_path.to_string_lossy();
    let mut metrics = Vec::new();

    parse_subsystem_metrics!(metrics, proc_path, "zfs_abd", "abdstats");
    parse_subsystem_metrics!(metrics, proc_path, "zfs_arc", "arcstats");
    parse_subsystem_metrics!(metrics, proc_path, "zfs_dbuf", "dbuf_stats");
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
    let pattern = format!("{}/spl/kstat/zfs/*/io", proc_path);
    let paths = glob::glob(&pattern)?;

    for path in paths.flatten() {
        let path = path.to_str().unwrap();
        let pool_name = parse_pool_name(path)?;
        let kvs = parse_pool_procfs_file(path).await?;
        for (key, value) in kvs {
            metrics.push(Metric::gauge_with_tags(
                format!("node_zfs_zpool_{}", key),
                key,
                value,
                tags! {
                    "zpool" => pool_name.to_string()
                },
            ))
        }
    }

    let pattern = format!("{}/spl/kstat/zfs/*/objset-*", proc_path);
    let paths = glob::glob(&pattern)?;
    for path in paths.flatten() {
        let path = path.to_str().unwrap();
        let kvs = parse_pool_objset_file(path).await?;
        for (key, value) in kvs {
            let fields = key.split('.').collect::<Vec<_>>();
            let desc = fields[0].to_string();
            let pool_name = fields[1].to_string();
            let dataset = fields[2].to_string();

            metrics.push(Metric::gauge_with_tags(
                format!("node_zfs_zpool_dataset_{}", key),
                desc,
                value,
                tags!(
                    "zpool" => pool_name,
                    "dataset" => dataset
                ),
            ));
        }
    }

    let pattern = format!("{}/spl/kstat/zfs/*/state", proc_path);
    let paths = glob::glob(&pattern)?;
    for path in paths.flatten() {
        let path = path.to_string_lossy();
        let pool_name = parse_pool_name(path.as_ref())?;
        let kvs = parse_pool_state_file(path.as_ref()).await?;
        for (key, value) in kvs {
            metrics.push(Metric::gauge_with_tags(
                "node_zfs_zpool_state",
                "kstat.zfs.misc.state",
                value,
                tags!(
                    "zpool" => pool_name.to_string(),
                    "state" => key
                ),
            ))
        }
    }

    Ok(metrics)
}

async fn parse_procfs_file(path: &str) -> Result<BTreeMap<String, u64>, Error> {
    let data = std::fs::read_to_string(path)?;

    let mut kvs = BTreeMap::new();
    let mut parse = false;
    for line in data.lines() {
        let fields = line.trim().split_ascii_whitespace().collect::<Vec<_>>();

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
        if fields[1] == "4" {
            let key = fields[0].to_string();
            let value = fields[2].parse().unwrap_or(0u64);
            kvs.insert(key, value);
        }
    }

    Ok(kvs)
}

async fn parse_pool_procfs_file(path: &str) -> Result<BTreeMap<String, u64>, Error> {
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

async fn parse_pool_objset_file(path: &str) -> Result<BTreeMap<String, u64>, Error> {
    let data = std::fs::read_to_string(path)?;

    let mut kvs = BTreeMap::new();
    let mut parse = false;
    let mut pool_name = String::new();
    let mut dataset_name = String::new();
    for line in data.lines() {
        let parts = line.trim().split_ascii_whitespace().collect::<Vec<_>>();

        if !parse
            && parts.len() == 3
            && parts[0] == "name"
            && parts[1] == "type"
            && parts[2] == "data"
        {
            parse = true;
            continue;
        }

        if !parse || parts.len() < 3 {
            continue;
        }

        if parts[0] == "dataset_name" {
            let elmts = path.split('/').collect::<Vec<_>>();
            let length = elmts.len();
            pool_name = elmts[length - 2].to_string();
            dataset_name = parts[2].to_string();
            continue;
        }

        if parts[1] == "4" {
            let key = format!("{}.{}.{}", parts[0], pool_name, dataset_name);
            let value = parts[2].parse::<u64>()?;

            kvs.insert(key, value);
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

async fn parse_pool_state_file(path: &str) -> Result<BTreeMap<&'static str, bool>, Error> {
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

    #[tokio::test]
    async fn test_parse_pool_procfs_file() {
        let paths = glob("tests/node/proc/spl/kstat/zfs/*/io").unwrap();
        let mut parsed = 0;
        for path in paths.flatten() {
            let path = path.to_str().unwrap();
            parsed += 1;
            let kvs = parse_pool_procfs_file(path).await.unwrap();
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

    #[tokio::test]
    async fn test_parse_pool_objset_file() {
        let paths = glob("tests/node/proc/spl/kstat/zfs/*/objset-*").unwrap();
        for path in paths.flatten() {
            let kvs = parse_pool_objset_file(path.to_str().unwrap())
                .await
                .unwrap();

            assert_ne!(kvs.len(), 0);
            for (k, v) in kvs {
                if k != "kstat.zfs.misc.objset.writes" {
                    continue;
                }

                if v != 0u64 && v != 4u64 && v != 10u64 {
                    panic!("incorrect value parsed from procfs data")
                }
            }
        }
    }

    #[tokio::test]
    async fn test_parse_pool_state_file() {
        let paths = glob("tests/node/proc/spl/kstat/zfs/*/state").unwrap();
        let mut handled = 0;
        for path in paths.flatten() {
            handled += 1;
            let path: &str = path.to_str().unwrap();
            let pool_name = parse_pool_name(path).unwrap();
            let kvs = parse_pool_state_file(path).await.unwrap();
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
