//! Collect metrics from /proc/meminfo

use std::{collections::HashMap, path::PathBuf};

use event::Metric;

use super::Error;

pub async fn gather(proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let infos = get_mem_info(proc_path).await?;

    let mut metrics = Vec::with_capacity(infos.len());
    for (key, value) in infos {
        if key.ends_with("_total") {
            metrics.push(Metric::sum(
                format!("node_memory_{key}"),
                format!("Memory information field {key}"),
                value,
            ));
        } else {
            metrics.push(Metric::gauge(
                format!("node_memory_{key}"),
                format!("Memory information field {key}"),
                value,
            ));
        }
    }

    Ok(metrics)
}

async fn get_mem_info(root: PathBuf) -> Result<HashMap<String, f64>, std::io::Error> {
    let data = std::fs::read_to_string(root.join("meminfo"))?;

    let mut infos = HashMap::new();
    for line in data.lines() {
        let mut parts = line.split_ascii_whitespace();

        let Some(key) = parts.next() else { continue };

        if let Some(value) = parts.next() {
            let value = value
                .parse::<f64>()
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;

            let mut key = key.replace(':', "").replace(['(', ')'], "_");

            let value = match parts.next() {
                Some(_) => {
                    if key.ends_with('_') {
                        key += "bytes"
                    } else {
                        key += "_bytes";
                    }

                    value * 1024.0
                }
                None => value,
            };

            infos.insert(key, value);
        }
    }

    Ok(infos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_mem_info() {
        let root = PathBuf::from("tests/node/proc");
        let infos = get_mem_info(root).await.unwrap();

        assert_eq!(infos.get("MemTotal_bytes").unwrap(), &(15666184.0 * 1024.0));
        assert_eq!(
            infos.get("DirectMap2M_bytes").unwrap(),
            &(16039936.0 * 1024.0)
        );
        assert_eq!(*infos.get("Active_bytes").unwrap(), 6761276.0 * 1024.0);
        assert_eq!(infos.get("HugePages_Total").unwrap(), &0.0);
    }
}
