use std::path::PathBuf;

use event::{tags, Metric};

use super::{read_to_string, Error};

pub async fn gather(root: &str) -> Result<Vec<Metric>, Error> {
    let mut path = PathBuf::from(root);
    path.push("class/nvme");

    let mut metrics = Vec::new();
    let mut readdir = tokio::fs::read_dir(path).await?;

    while let Some(dir) = readdir.next_entry().await? {
        let infos = read_nvme_device(dir.path()).await?;

        metrics.push(Metric::gauge_with_tags(
            "node_nvme_info",
            "node_nvme_info Non-numeric data from /sys/class/nvme/<device>, value is always 1",
            1f64,
            tags!(
                "serial" => &infos[0],
                "model" => &infos[1],
                "state" => &infos[2],
                "firmware_rev" => &infos[3]
            ),
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

    Ok(vec![serial, model, state, firmware])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_dir() {
        let path = PathBuf::from("tests/fixtures/sys/class/nvme/nvme0");
        let mut rd = tokio::fs::read_dir(path).await.unwrap();

        let mut count = 0;
        while let Some(_dir) = rd.next_entry().await.unwrap() {
            count += 1;
        }

        assert_eq!(count, 4);
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
