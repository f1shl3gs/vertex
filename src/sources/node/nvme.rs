use std::{
    path::PathBuf,
};
use super::{read_to_string, Error, ErrorContext};
use event::{tags, gauge_metric, Metric};

pub async fn gather(root: &str) -> Result<Vec<Metric>, Error> {
    let mut path = PathBuf::from(root);
    path.push("class/nvme");

    let mut metrics = Vec::new();
    let mut readdir = tokio::fs::read_dir(path).await
        .context("read nvme root dir failed")?;

    while let Some(dir) = readdir.next_entry().await
        .context("readdir nvme dir entries failed")?
    {
        let infos = read_nvme_device(dir.path()).await?;

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
        let path = PathBuf::from("tests/fixtures/sys/class/nvme/nvme0");
        let mut rd = tokio::fs::read_dir(path).await.unwrap();

        while let Some(dir) = rd.next_entry().await.unwrap() {
            println!("{:?}", dir);
        }
    }

    #[tokio::test]
    async fn test_read_nvme_device() {
        let root = PathBuf::from("tests/fixtures/sys/class/nvme/nvme0");
        let infos = read_nvme_device(root).await.unwrap();

        assert_eq!(infos[0], "S680HF8N190894I");
        assert_eq!(infos[1], "Samsung SSD 970 PRO 512GB");
        assert_eq!(infos[2], "live");
        assert_eq!(infos[3], "1B2QEXP7");
    }
}