/// Collect metrics from /proc/meminfo

use std::{
    path::PathBuf,
    collections::HashMap,
    sync::Arc,
};
use crate::{
    event::{Metric, MetricValue},
    gauge_metric,
    sum_metric,
    sources::node::read_to_string,
};

pub async fn gather(root: Arc<String>) -> Result<Vec<Metric>, ()> {
    let mut root = PathBuf::from(root.as_ref());
    let infos = get_mem_info(root).await.map_err(|err| {
        warn!("get mem info failed"; "err" => err);
        ()
    })?;

    let mut metrics = Vec::new();
    for (k, v) in infos {
        let k = k.clone();
        if k.ends_with("_total") {
            metrics.push(sum_metric!(
                "node_memory_".to_owned() + &k,
                "Memory information field ".to_owned() + &k,
                v
            ));
        } else {
            metrics.push(gauge_metric!(
                "node_memory_".to_owned() + &k,
                "Memory information field ".to_owned() + &k,
                v
            ));
        }
    }

    Ok(metrics)
}

async fn get_mem_info(mut root: PathBuf) -> Result<HashMap<String, f64>, std::io::Error> {
    let mut path = root;
    path.push("meminfo");

    let mut infos = HashMap::new();

    let content = read_to_string(path).await?;
    let lines = content.lines();

    for line in lines {
        let parts = line.split_ascii_whitespace()
            .collect::<Vec<_>>();

        let mut fv = parts[1].parse::<f64>()
            .map_err(|e| std::io::ErrorKind::from(std::io::ErrorKind::InvalidInput))?;

        let mut key = parts[0]
            .replace(":", "")
            .replace("(", "_")
            .replace(")", "_");

        match parts.len() {
            2 => { /* no unit */ }
            3 => {
                // with unit, we presume kB
                fv *= 1024.0;
                if key.ends_with("_") {
                    key = key + "byte"
                } else {
                    key = key + "_bytes";
                }
            }
            _ => unreachable!()
        }

        infos.insert(key, fv);
    }

    Ok(infos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_mem_info() {
        let root = PathBuf::from("testdata/proc");
        let infos = get_mem_info(root).await.unwrap();

        assert_eq!(infos.get("MemTotal_bytes").unwrap(), &(15666184.0 * 1024.0));
        assert_eq!(infos.get("DirectMap2M_bytes").unwrap(), &(16039936.0 * 1024.0));
        assert_eq!(infos.get("HugePages_Total").unwrap(), &0.0);
    }
}