use std::{
    path::PathBuf,
    collections::BTreeMap,
};
use crate::{
    event::{Metric, MetricValue},
    gauge_metric,
    tags,
    sources::node::read_to_string,
};

pub async fn gather(root: &str) -> Result<Vec<Metric>, ()> {
    let mut path = PathBuf::from(root);
    path.push("class/nvme");

    let mut metrics = Vec::new();
    let mut readdir = tokio::fs::read_dir(path).await.map_err(|err| {
        warn!("read nvme root dir failed"; "err" => err);
    })?;
    while let Some(dir) = readdir.next_entry().await.map_err(|err| {
        warn!("readdir nvme dir entries failed"; "err" => err);
    })? {
        let infos = read_nvme_device(dir.path()).await.map_err(|_| ())?;

        metrics.push(gauge_metric!(
                    "node_nvme_info",
                    "node_nvme_info Non-numeric data from /sys/class/nvme/<device>, value is always 1",
                    1f64,
                    "serial" => infos[0].clone(),
                    "model" => infos[1].clone(),
                    "state" => infos[2].clone(),
                    "firmware_rev" => infos[3].clone()
                ));
    }

    Ok(metrics)
}

async fn read_nvme_device(root: PathBuf) -> Result<Vec<String>, std::io::Error> {
    let mut path = root.clone();
    path.push("serial");
    let serial = read_to_string(path).await?.trim_end().to_string();

    let mut path = root.clone();
    path.push("model");
    let model = read_to_string(path).await?.trim_end().to_string();

    let mut path = root.clone();
    path.push("state");
    let state = read_to_string(path).await?.trim_end().to_string();

    let mut path = root.clone();
    path.push("firmware_rev");
    let firmware = read_to_string(path).await?.trim_end().to_string();

    Ok(vec![
        serial,
        model,
        state,
        firmware,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_dir() {
        let path = PathBuf::from("testdata/sys/class/nvme/nvme0");
        let mut rd = tokio::fs::read_dir(path).await.unwrap();

        while let Some(dir) = rd.next_entry().await.unwrap() {
            println!("{:?}", dir);
        }
    }

    #[tokio::test]
    async fn test_read_nvme_device() {
        let root = PathBuf::from("testdata/sys/class/nvme/nvme0");
        let infos = read_nvme_device(root).await.unwrap();

        assert_eq!(infos[0], "S680HF8N190894I");
        assert_eq!(infos[1], "Samsung SSD 970 PRO 512GB");
        assert_eq!(infos[2], "live");
        assert_eq!(infos[3], "1B2QEXP7");
    }
}