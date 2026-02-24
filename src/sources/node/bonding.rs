use std::collections::HashMap;
use std::path::PathBuf;

use event::{Metric, tags};

use super::{Error, read_string};

pub async fn gather(sys_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let stats = read_bonding_stats(sys_path).await?;

    let mut metrics = Vec::with_capacity(stats.len() * 2);
    for (master, status) in stats {
        let tags = tags!("master" => master);

        metrics.extend([
            Metric::gauge_with_tags(
                "node_bonding_slaves",
                "Number of configured slaves per bonding interface.",
                status[0],
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_bonding_active",
                "Number of active slaves per bonding interface.",
                status[1],
                tags,
            ),
        ]);
    }

    Ok(metrics)
}

async fn read_bonding_stats(sys_path: PathBuf) -> Result<HashMap<String, Vec<f64>>, Error> {
    let masters = read_string(sys_path.join("class/net/bonding_masters"))?;

    let mut status = HashMap::new();
    let parts = masters.split_ascii_whitespace();
    for master in parts {
        let path = sys_path.join(format!("class/net/{master}/bonding/slaves"));

        if let Ok(slaves) = read_string(path) {
            let mut sstat = vec![0f64, 0f64];
            for slave in slaves.split_ascii_whitespace() {
                let path = sys_path.join(format!(
                    "class/net/{master}/lower_{slave}/bonding_slave/mii_status",
                ));
                if let Ok(state) = read_string(path) {
                    sstat[0] += 1f64;
                    if state == "up" {
                        sstat[1] += 1f64;
                    }
                }

                // some older? kernels use slave_ prefix
                let path = sys_path.join(format!(
                    "class/net/{master}/slave_{slave}/bonding_slave/mii_status",
                ));
                if let Ok(state) = read_string(path) {
                    sstat[0] += 1f64;
                    if state == "up" {
                        sstat[1] += 1f64;
                    }
                }
            }

            status.insert(master.to_string(), sstat);
        } else {
            continue;
        }
    }

    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_bonding_stats() {
        let path = PathBuf::from("tests/bonding/sys");
        let stats = read_bonding_stats(path).await.unwrap();

        assert_ne!(stats.len(), 0);
        assert_eq!(stats.get("bond0").unwrap(), &vec![0f64, 0f64]);
        assert_eq!(stats.get("int").unwrap(), &vec![2f64, 1f64]);
        assert_eq!(stats.get("dmz").unwrap(), &vec![2f64, 2f64]);
    }
}
