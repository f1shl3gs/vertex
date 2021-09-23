/// Exposes ZFS performance statistics

use crate::event::Metric;
use crate::sources::node::errors::Error;
use tokio::io::AsyncBufReadExt;
use std::collections::BTreeMap;

macro_rules! parse_subsystem_metrics {
    ($metrics: expr, $root: expr, $subsystem: expr, $path: expr) => {
        let path = format!("{}/{}", $root, $subsystem);
        for (k, v) in parse_procfs_file(&path).await? {
            $metrics.push(Metric::gauge(
                format!("node_zfs_{}", k),
                "todo",
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
    let pattern = format!("{}/*/io", proc_path);
    let paths = glob::glob(&pattern)?;
    for result in paths {
        if let Ok(e) = result {}
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
        let fields = line
            .trim()
            .split_ascii_whitespace()
            .collect::<Vec<_>>();

        if !parse && fields.len() == 3 && fields[0] == "name" && fields[1] == "type" && fields[2] == "data" {
            // start parsing from here.
            parse = true;
            continue;
        }

        // kstat data type (column 2) should be KSTAT_DATA_UINT64, otherwise ignore
        // TODO: when other KSTAT_DATA_* types arrive, much of this will need to be restructured
        if fields[1] == "4" {
            let key = format!("kstat.zfs.misc.{}.{}", "ext", fields[0]);
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

    let zps = path.split("/")
        .collect::<Vec<_>>();
    let length = zps.len();
    if length < 2 {
        return Err(Error::new_invalid("zpool path did not return at least two elements"));
    }

    let zpool_file = zps[length - 1];

    let mut parse = false;
    let mut headers = vec![];
    while let Some(line) = lines.next_line().await? {
        let fields = line
            .trim()
            .split_ascii_whitespace()
            .collect::<Vec<_>>();

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

        for (i, header) in headers.iter().enumerate() {
            let key = format!("kstat.zfs.misc.{}.{}", zpool_file, header);
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
    let mut pool_name = "";
    let mut dataset_name = "";
    while let Some(line) = lines.next_line().await? {
        let parts = line
            .trim()
            .split_ascii_whitespace()
            .collect::<Vec<_>>();

        if !parse && parts.len() == 3 && parts[0] == "name" && parts[1] == "type" && parts[2] == "data" {
            parse = true;
            continue;
        }

        if !parse || parts.len() < 3 {
            continue;
        }

        if parts[0] == "dataset_name" {
            let elmts = path.split("/").collect::<Vec<_>>();
            let length = elmts.len();
            pool_name = elmts[length - 2].clone();
            dataset_name = parts[2].clone();
            continue;
        }

        if parts[1] == "4" {
            let key = format!("kstat.zfs.misc.objset.{}", parts[0]);
            let value = parts[2].parse::<u64>()?;

            kvs.insert(key, value);
        }
    }

    Ok(kvs)
}

async fn parse_pool_state_file(path: &str) -> Result<BTreeMap<String, u64>, Error> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use glob::glob;

    #[tokio::test]
    async fn test_parse_pool_procfs_file() {
        let paths = glob("testdata/proc/spl/kstat/zfs/*/io").unwrap();
        let mut parsed = 0;
        for path in paths {
            if let Ok(path) = path {
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
        }

        assert_eq!(parsed, 2);
    }

    #[tokio::test]
    async fn test_parse_pool_objset_file() {}

    #[tokio::test]
    async fn test_parse_state_file() {}
}