use std::path::PathBuf;

use event::{Metric, tags};

use super::{Error, read_string};

pub async fn gather(sys_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let dirs = std::fs::read_dir(sys_path.join("class/nvme"))?;

    let mut metrics = Vec::new();
    for entry in dirs.flatten() {
        let [device, serial, model, state, firmware_rev] = read_nvme_device(entry.path())?;

        metrics.push(Metric::gauge_with_tags(
            "node_nvme_info",
            "Non-numeric data from /sys/class/nvme/<device>, value is always 1",
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

    let serial = read_string(root.join("serial"))?;
    let model = read_string(root.join("model"))?;
    let state = read_string(root.join("state"))?;
    let firmware = read_string(root.join("firmware_rev"))?;

    Ok([device, serial, model, state, firmware])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_nvme_device() {
        let root = PathBuf::from("tests/node/sys/class/nvme/nvme0");
        let infos = read_nvme_device(root).unwrap();

        assert_eq!(infos[0], "nvme0");
        assert_eq!(infos[1], "S680HF8N190894I");
        assert_eq!(infos[2], "Samsung SSD 970 PRO 512GB");
        assert_eq!(infos[3], "live");
        assert_eq!(infos[4], "1B2QEXP7");
    }
}
