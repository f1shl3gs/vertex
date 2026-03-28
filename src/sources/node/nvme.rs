use std::path::PathBuf;
use std::sync::LazyLock;

use event::{Metric, tags};
use regex::Regex;

use super::{Error, Paths, read_into, read_sys_file};

static NAMESPACE_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"nvme\d+c\d+n(\d+)"#).unwrap());

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let mut metrics = Vec::new();

    for entry in paths.sys().join("class/nvme").read_dir()?.flatten() {
        let [device, serial, model, state, firmware_rev, cntlid] = read_nvme_device(entry.path())?;

        metrics.push(Metric::gauge_with_tags(
            "node_nvme_info",
            "Non-numeric data from /sys/class/nvme/<device>, value is always 1",
            1,
            tags!(
                "cntlid" => cntlid,
                "device" => &device,
                "firmware_revision" => firmware_rev,
                "model" => model,
                "serial" => serial,
                "state" => state,
            ),
        ));

        let dirs = std::fs::read_dir(entry.path())?;
        for entry in dirs.flatten() {
            if !entry.file_type()?.is_dir() {
                continue;
            }

            let filename = entry.file_name();
            let filename = filename.to_string_lossy();
            let Some(captures) = NAMESPACE_PATTERN.captures(filename.as_ref()) else {
                continue;
            };

            let Some(id) = captures.get(1) else {
                continue;
            };

            let ana_state =
                read_sys_file(entry.path().join("ana_state")).unwrap_or_else(|_| "unknown".into());
            let size: f64 = read_into(entry.path().join("size"))?;
            let logical_block_size: f64 = read_into(entry.path().join("queue/logical_block_size"))?;
            let nuse: f64 = read_into(entry.path().join("nuse"))?;

            metrics.extend([
                Metric::gauge_with_tags(
                    "node_nvme_namespace_info",
                    "Information about NVMe namespaces. Value is always 1",
                    1,
                    tags!(
                        "ana_state" => &ana_state,
                        "device" => &device,
                        "nsid" => id.as_str(),
                    )
                ),
                Metric::gauge_with_tags(
                    "node_nvme_namespace_capacity_bytes",
                    "Capacity of the NVMe namespace in bytes. Computed as namespace_size * namespace_logical_block_size",
                    size * logical_block_size,
                    tags!(
                        "device" => &device,
                        "nsid" => id.as_str(),
                    )
                ),
                Metric::gauge_with_tags(
                    "node_nvme_namespace_size_bytes",
                    "Size of the NVMe namespace in bytes. Available in /sys/class/nvme/<device>/<namespace>/size",
                    size * logical_block_size,
                    tags!(
                        "device" => &device,
                        "nsid" => id.as_str(),
                    )
                ),
                Metric::gauge_with_tags(
                    "node_nvme_namespace_used_bytes",
                    "Used space of the NVMe namespace in bytes. Available in /sys/class/nvme/<device>/<namespace>/nuse",
                    nuse * logical_block_size,
                    tags!(
                        "device" => &device,
                        "nsid" => id.as_str(),
                    )
                ),
                Metric::gauge_with_tags(
                    "node_nvme_namespace_logical_block_size_bytes",
                    "Logical block size of the NVMe namespace in bytes. Usually 4Kb. Available in /sys/class/nvme/<device>/<namespace>/queue/logical_block_size",
                    logical_block_size,
                    tags!(
                        "device" => &device,
                        "nsid" => id.as_str(),
                    )
                )
            ]);
        }
    }

    Ok(metrics)
}

fn read_nvme_device(root: PathBuf) -> Result<[String; 6], std::io::Error> {
    let device = root
        .file_name()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let serial = read_sys_file(root.join("serial"))?;
    let model = read_sys_file(root.join("model"))?;
    let state = read_sys_file(root.join("state"))?;
    let firmware = read_sys_file(root.join("firmware_rev"))?;
    let cntlid = read_sys_file(root.join("cntlid"))?;

    Ok([device, serial, model, state, firmware, cntlid])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read() {
        let root = PathBuf::from("tests/node/fixtures/sys/class/nvme/nvme0");
        let infos = read_nvme_device(root).unwrap();

        assert_eq!(infos[0], "nvme0");
        assert_eq!(infos[1], "S680HF8N190894I");
        assert_eq!(infos[2], "Samsung SSD 970 PRO 512GB");
        assert_eq!(infos[3], "live");
        assert_eq!(infos[4], "1B2QEXP7");
    }
}
