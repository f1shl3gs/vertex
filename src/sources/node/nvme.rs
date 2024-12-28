use std::fs::read_dir;
use std::path::PathBuf;

use event::{tags, Metric};

use super::{read_to_string, Error};

pub async fn gather(root: PathBuf) -> Result<Vec<Metric>, Error> {
    let mut metrics = Vec::new();
    let mut readdir = read_dir(root.join("class/nvme"))?;

    while let Some(Ok(entry)) = readdir.next() {
        let [device, serial, model, state, firmware_rev] = read_nvme_device(entry.path())?;

        metrics.push(Metric::gauge_with_tags(
            "node_nvme_info",
            "node_nvme_info Non-numeric data from /sys/class/nvme/<device>, value is always 1",
            1f64,
            tags!(
                "device" => device,
                "firmware_revision" => firmware_rev,
                "model" => model,
                "serial" => serial,
                "state" => state,
            ),
        ));
    }

    Ok(metrics)
}

fn read_nvme_device(root: PathBuf) -> Result<[String; 5], std::io::Error> {
    let device = root
        .file_name()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let serial = read_to_string(root.join("serial"))?;
    let model = read_to_string(root.join("model"))?;
    let state = read_to_string(root.join("state"))?;
    let firmware = read_to_string(root.join("firmware_rev"))?;

    Ok([device, serial, model, state, firmware])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_dir() {
        let path = PathBuf::from("tests/node/sys/class/nvme/nvme0");
        let mut rd = read_dir(path).unwrap();

        let mut count = 0;
        while let Some(Ok(_dir)) = rd.next() {
            count += 1;
        }

        assert_eq!(count, 4);
    }

    #[test]
    fn test_read_nvme_device() {
        let root = PathBuf::from("tests/node/sys/class/nvme/nvme0");
        let infos = read_nvme_device(root).unwrap();

        assert_eq!(infos[0], "S680HF8N190894I");
        assert_eq!(infos[1], "Samsung SSD 970 PRO 512GB");
        assert_eq!(infos[2], "live");
        assert_eq!(infos[3], "1B2QEXP7");
    }
}
