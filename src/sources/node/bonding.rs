use std::collections::HashMap;
use std::path::PathBuf;

use event::{tags, Metric};

use super::read_to_string;
use super::{Error, ErrorContext};

pub async fn gather(sys_path: &str) -> Result<Vec<Metric>, Error> {
    let path = PathBuf::from(sys_path);
    let stats = read_bonding_stats(path)
        .await
        .context("read bonding stats failed")?;

    let mut metrics = Vec::with_capacity(stats.len() * 2);

    for (master, status) in stats {
        metrics.push(Metric::gauge_with_tags(
            "node_bonding_slaves",
            "Number of configured slaves per bonding interface.",
            status[0],
            tags!(
                "master" => &master,
            ),
        ));
        metrics.push(Metric::gauge_with_tags(
            "node_bonding_active",
            "Number of active slaves per bonding interface.",
            status[1],
            tags!(
                "master" => &master
            ),
        ));
    }

    Ok(metrics)
}

async fn read_bonding_stats(sys_path: PathBuf) -> Result<HashMap<String, Vec<f64>>, Error> {
    let mut path = sys_path.clone();
    path.push("class/net/bonding_masters");

    let mut status = HashMap::new();

    let masters = read_to_string(path).await?;

    let parts = masters.split_ascii_whitespace();
    for master in parts {
        let mut path = sys_path.clone();
        path.push(format!("class/net/{}/bonding/slaves", master));

        if let Ok(slaves) = read_to_string(path).await {
            let mut sstat = vec![0f64, 0f64];
            for slave in slaves.split_ascii_whitespace() {
                let mut path = sys_path.clone();
                path.push(format!(
                    "class/net/{}/lower_{}/bonding_slave/mii_status",
                    master, slave
                ));

                if let Ok(state) = read_to_string(path).await {
                    sstat[0] += 1f64;
                    if state.trim() == "up" {
                        sstat[1] += 1f64;
                    }
                }

                // some older? kernels use slave_ prefix
                let mut path = sys_path.clone();
                path.push(format!(
                    "class/net/{}/slave_{}/bonding_slave/mii_status",
                    master, slave
                ));

                if let Ok(state) = read_to_string(path).await {
                    sstat[0] += 1f64;
                    if state.trim() == "up" {
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
        let path = PathBuf::from("tests/fixtures/bonding/sys");
        let stats = read_bonding_stats(path).await.unwrap();

        assert_ne!(stats.len(), 0);
        assert_eq!(stats.get("bond0").unwrap(), &vec![0f64, 0f64]);
        assert_eq!(stats.get("int").unwrap(), &vec![2f64, 1f64]);
        assert_eq!(stats.get("dmz").unwrap(), &vec![2f64, 2f64]);
    }
}
