use std::collections::HashMap;

use crate::read::read_to_string;
use crate::{Error, SysFs};

impl SysFs {
    pub async fn bonding(&self) -> Result<HashMap<String, Vec<f64>>, Error> {
        let path = self.root.join("class/net/bonding_masters");
        let mut status = HashMap::new();

        let masters = read_to_string(path).await?;

        let parts = masters.split_ascii_whitespace();
        for master in parts {
            let mut path = self.root.clone();
            path.push(format!("class/net/{}/bonding/slaves", master));

            if let Ok(slaves) = read_to_string(path).await {
                let mut sstat = vec![0f64, 0f64];
                for slave in slaves.split_ascii_whitespace() {
                    let mut path = self.root.clone();
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
                    let mut path = self.root.clone();
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stats() {
        let sysfs = SysFs::new_test();

        let stats = sysfs.bonding().await.unwrap();
        assert_ne!(stats.len(), 0);
        assert_eq!(stats.get("bond0").unwrap(), &vec![0f64, 0f64]);
        assert_eq!(stats.get("int").unwrap(), &vec![2f64, 1f64]);
        assert_eq!(stats.get("dmz").unwrap(), &vec![2f64, 2f64]);
    }
}
