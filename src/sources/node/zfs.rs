/// Exposes ZFS performance statistics
use std::collections::BTreeMap;

use event::{tags, Metric};
use tokio::io::AsyncBufReadExt;

use super::{read_to_string, Error};

macro_rules! parse_subsystem_metrics {
    ($metrics: expr, $root: expr, $subsystem: expr, $path: expr) => {
        let path = format!("{}/spl/kstat/zfs/{}", $root, $path);
        for (k, v) in parse_procfs_file(&path).await? {
            let k = k.replace("-", "_");
            $metrics.push(Metric::gauge(
                format!("node_{}_{}", $subsystem, k),
                k.clone(),
                v as f64,
            ))
        }
    };
}

pub async fn gather(proc_path: &str) -> Result<Vec<Metric>, Error> {
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
    for path in paths.filter_map(Result::ok) {
        let path = path.to_str().unwrap();
        let pool_name = parse_pool_name(path)?;
        let kvs = parse_pool_procfs_file(path).await?;
        for (k, v) in kvs {
            metrics.push(Metric::gauge_with_tags(
                "node_zfs_zpool_".to_owned() + &k,
                k.clone(),
                v as f64,
                tags! {
                    "zpool" => pool_name.clone()
                },
            ))
        }
    }

    let pattern = format!("{}/spl/kstat/zfs/*/objset-*", proc_path);
    let paths = glob::glob(&pattern)?;
    for path in paths.filter_map(Result::ok) {
        let path = path.to_str().unwrap();
        let kvs = parse_pool_objset_file(path).await?;
        for (k, v) in kvs {
            let fields = k.split('.').collect::<Vec<_>>();
            let k = fields[0];
            let pool_name = fields[1];
            let dataset = fields[2];

            metrics.push(Metric::gauge_with_tags(
                format!("node_zfs_zpool_dataset_{}", k),
                k.to_string(),
                v as f64,
                tags!(
                    "zpool" => pool_name.to_string(),
                    "dataset" => dataset.to_string()
                ),
            ));
        }
    }

    let pattern = format!("{}/spl/kstat/zfs/*/state", proc_path);
    let paths = glob::glob(&pattern)?;
    for path in paths.filter_map(Result::ok) {
        let path = path.to_string_lossy();
        let pool_name = parse_pool_name(path.as_ref())?;
        let kvs = parse_pool_state_file(path.as_ref()).await?;
        for (k, v) in kvs {
            let v = match v {
                true => 1f64,
                false => 0f64,
            };

            metrics.push(Metric::gauge_with_tags(
                "node_zfs_zpool_state",
                "kstat.zfs.misc.state",
                v,
                tags!(
                    "zpool" => &pool_name,
                    "state" => k
                ),
            ))
        }
    }

    Ok(metrics)
}

async fn parse_procfs_file(path: &str) -> Result<BTreeMap<String, u64>, Error> {
    let f = tokio::fs::File::open(path).await?;
    let reader = tokio::io::BufReader::new(f);
    let mut lines = reader.lines();
    let mut kvs = BTreeMap::new();

    let mut parse = false;
    while let Some(line) = lines.next_line().await? {
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
    let mut kvs = BTreeMap::new();

    let f = tokio::fs::File::open(path).await?;
    let reader = tokio::io::BufReader::new(f);
    let mut lines = reader.lines();

    let length = path.split('/').count();
    if length < 2 {
        return Err(Error::new_invalid(
            "zpool path did not return at least two elements",
        ));
    }

    let mut parse = false;
    let mut headers = vec![];
    while let Some(line) = lines.next_line().await? {
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
    let mut kvs = BTreeMap::new();
    let f = tokio::fs::File::open(path).await?;
    let reader = tokio::io::BufReader::new(f);
    let mut lines = reader.lines();

    let mut parse = false;
    let mut pool_name = String::new();
    let mut dataset_name = String::new();
    while let Some(line) = lines.next_line().await? {
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

async fn parse_pool_state_file(path: &str) -> Result<BTreeMap<String, bool>, Error> {
    let stats = [
        "online", "degraded", "faulted", "offline", "removed", "unavail",
    ];
    let actual_state = read_to_string(path).await?.trim().to_lowercase();

    let mut kvs = BTreeMap::new();

    for s in stats {
        let active = actual_state == s;
        let key = s.to_string();

        kvs.insert(key, active);
    }

    Ok(kvs)
}

fn parse_pool_name(path: &str) -> Result<String, Error> {
    let elements = path.split('/').collect::<Vec<_>>();
    let length = elements.len();
    if length < 2 {
        return Err(Error::new_invalid(
            "zpool path did not return at least two elements",
        ));
    }

    let name = elements[length - 2];
    Ok(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use glob::glob;

    #[tokio::test]
    async fn test_parse_pool_procfs_file() {
        let paths = glob("tests/fixtures/proc/spl/kstat/zfs/*/io").unwrap();
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
        let paths = glob("tests/fixtures/proc/spl/kstat/zfs/*/objset-*").unwrap();
        for path in paths.filter_map(Result::ok) {
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
        let paths = glob("tests/fixtures/proc/spl/kstat/zfs/*/state").unwrap();
        let mut handled = 0;
        for path in paths.filter_map(Result::ok) {
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
