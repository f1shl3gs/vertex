//! Exposes disk I/O statistics
//!
//! Docs from https://www.kernel.org/doc/Documentation/iostats.txt

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use configurable::Configurable;
use event::{Metric, tags, tags::Key};
use framework::config::serde_regex;
use serde::{Deserialize, Serialize};

use super::{Error, read_into};

const DISK_SECTOR_SIZE: f64 = 512.0;

const DEVICE_KEY: Key = Key::from_static("device");

fn default_ignored() -> regex::Regex {
    regex::Regex::new("^(z?ram|loop|fd|(h|s|v|xv)d[a-z]|nvme\\d+n\\d+p)\\d+$").unwrap()
}

#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(default = "default_ignored", with = "serde_regex")]
    ignored: regex::Regex,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ignored: default_ignored(),
        }
    }
}

pub async fn gather(
    conf: Config,
    proc_path: PathBuf,
    sys_path: PathBuf,
    udev_path: PathBuf,
) -> Result<Vec<Metric>, Error> {
    let data = std::fs::read_to_string(proc_path.join("diskstats"))?;

    // https://www.kernel.org/doc/Documentation/ABI/testing/procfs-diskstats
    //
    // The /proc/diskstats file displays the I/O statistics
    // of block devices. Each line contains the following 14
    // fields:
    //
    // ==  ===================================
    // 1  major number
    // 2  minor number
    // 3  device name
    // 4  reads completed successfully
    // 5  reads merged
    // 6  sectors read
    // 7  time spent reading (ms)
    // 8  writes completed
    // 9  writes merged
    // 10  sectors written
    // 11  time spent writing (ms)
    // 12  I/Os currently in progress
    // 13  time spent doing I/Os (ms)
    // 14  weighted time spent doing I/Os (ms)
    //     ==  ===================================
    //
    //     Kernel 4.18+ appends four more fields for discard
    //     tracking putting the total at 18:
    //
    // ==  ===================================
    // 15  discards completed successfully
    // 16  discards merged
    // 17  sectors discarded
    // 18  time spent discarding
    //     ==  ===================================
    //
    // Kernel 5.5+ appends two more fields for flush requests:
    //
    // ==  =====================================
    // 19  flush requests completed successfully
    // 20  time spent flushing
    //     ==  =====================================
    //
    //     For more details refer to Documentation/admin-guide/iostats.rst
    let mut metrics = Vec::new();
    for line in data.lines() {
        // the content looks like this
        // 259       0 nvme0n1 366 0 23480 41 3 0 0 0 0 41 41 0 0 0 0
        let parts = line.split_ascii_whitespace().collect::<Vec<_>>();
        if parts.len() < 14 {
            continue;
        }

        let major = parts[0];
        let minor = parts[1];
        let device = parts[2];
        if conf.ignored.is_match(device) {
            continue;
        }

        let info = match udev_device_properties(&udev_path, major, minor) {
            Ok(info) => info,
            Err(err) => {
                debug!(
                    message = "read udev properties failed",
                    major,
                    minor,
                    %err,
                );
                continue;
            }
        };

        let serial = info
            .get("SCSI_IDENT_SERIAL")
            .or_else(|| info.get("ID_SERIAL_SHORT"))
            // used by virtio devices
            .or_else(|| info.get("ID_SERIAL"))
            .cloned()
            .unwrap_or_default();

        // stats for /sys/block/xxx/queue where xxx is a device name.
        let path = sys_path.join("block").join(device).join("queue/rotational");
        let rotational = match read_into::<_, u64, _>(&path) {
            Ok(rotational) => rotational,
            Err(err) => {
                debug!(
                    message = "read rotational of queue stats failed",
                    ?path,
                    %err
                );
                0
            }
        };

        metrics.push(Metric::gauge_with_tags(
            "node_disk_info",
            "Info of /sys/block/<block_device>.",
            1,
            tags!(
                "device" => device,
                "major" => major,
                "minor" => minor,
                "path" => info.get("ID_PATH").cloned().unwrap_or_default(),
                "wwn" => info.get("ID_WWN").cloned().unwrap_or_default(),
                "model" => info.get("ID_MODEL").cloned().unwrap_or_default(),
                "serial" => serial,
                "rotational" => rotational,
                "revision" => info.get("ID_REVISION").cloned().unwrap_or_default(),
            ),
        ));

        for (index, part) in parts.iter().skip(3).enumerate() {
            let v = part.parse::<f64>().unwrap_or(0f64);

            match index {
                0 => metrics.push(Metric::sum_with_tags(
                    "node_disk_reads_completed_total",
                    "The total number of reads completed successfully",
                    v,
                    tags!(DEVICE_KEY => device),
                )),
                1 => metrics.push(Metric::sum_with_tags(
                    "node_disk_reads_merged_total",
                    "The total number of reads merged",
                    v,
                    tags!(DEVICE_KEY => device),
                )),
                2 => metrics.push(Metric::sum_with_tags(
                    "node_disk_read_bytes_total",
                    "The total number of bytes read successfully",
                    v * DISK_SECTOR_SIZE,
                    tags!(DEVICE_KEY => device),
                )),
                3 => metrics.push(Metric::sum_with_tags(
                    "node_disk_read_time_seconds_total",
                    "The total number of seconds spent by all reads",
                    v * 0.001,
                    tags!(DEVICE_KEY => device),
                )),
                4 => metrics.push(Metric::sum_with_tags(
                    "node_disk_writes_completed_total",
                    "The total number of writes completed successfully",
                    v,
                    tags!(DEVICE_KEY => device),
                )),
                5 => metrics.push(Metric::sum_with_tags(
                    "node_disk_writes_merged_total",
                    "The number of writes merged.",
                    v,
                    tags!(DEVICE_KEY => device),
                )),
                6 => metrics.push(Metric::sum_with_tags(
                    "node_disk_written_bytes_total",
                    "The total number of bytes written successfully.",
                    v * DISK_SECTOR_SIZE,
                    tags!(DEVICE_KEY => device),
                )),
                7 => metrics.push(Metric::sum_with_tags(
                    "node_disk_write_time_seconds_total",
                    "This is the total number of seconds spent by all writes.",
                    v * 0.001,
                    tags!(DEVICE_KEY => device),
                )),
                8 => metrics.push(Metric::gauge_with_tags(
                    "node_disk_io_now",
                    "The number of I/Os currently in progress",
                    v,
                    tags!(DEVICE_KEY => device),
                )),
                9 => metrics.push(Metric::sum_with_tags(
                    "node_disk_io_time_seconds_total",
                    "Total seconds spent doing I/Os.",
                    v * 0.001,
                    tags!(DEVICE_KEY => device),
                )),
                10 => metrics.push(Metric::sum_with_tags(
                    "node_disk_io_time_weighted_seconds_total",
                    "The weighted # of seconds spent doing I/Os.",
                    v * 0.001,
                    tags!(DEVICE_KEY => device),
                )),
                11 => metrics.push(Metric::sum_with_tags(
                    "node_disk_discards_completed_total",
                    "The total number of discards completed successfully.",
                    v,
                    tags!(DEVICE_KEY => device),
                )),
                12 => metrics.push(Metric::sum_with_tags(
                    "node_disk_discards_merged_total",
                    "The total number of discards merged.",
                    v,
                    tags!(DEVICE_KEY => device),
                )),
                13 => metrics.push(Metric::sum_with_tags(
                    "node_disk_discarded_sectors_total",
                    "The total number of sectors discarded successfully.",
                    v,
                    tags!(DEVICE_KEY => device),
                )),
                14 => metrics.push(Metric::sum_with_tags(
                    "node_disk_discard_time_seconds_total",
                    "This is the total number of seconds spent by all discards.",
                    v * 0.001,
                    tags!(DEVICE_KEY => device),
                )),
                15 => metrics.push(Metric::sum_with_tags(
                    "node_disk_flush_requests_total",
                    "The total number of flush requests completed successfully",
                    v,
                    tags!(DEVICE_KEY => device),
                )),
                16 => metrics.push(Metric::sum_with_tags(
                    "node_disk_flush_requests_time_seconds_total",
                    "This is the total number of seconds spent by all flush requests.",
                    v * 0.001,
                    tags!(DEVICE_KEY => device),
                )),
                _ => {}
            }
        }

        if let Some(fs_type) = info.get("ID_FS_TYPE")
            && !fs_type.is_empty()
        {
            metrics.push(Metric::gauge_with_tags(
                "node_disk_filesystem_info",
                "Info about disk filesystem",
                1,
                tags!(
                    "device" => device,
                    "type" => fs_type,
                    "usage" => info.get("ID_FS_USAGE").cloned().unwrap_or_default(),
                    "uuid" => info.get("ID_FS_UUID").cloned().unwrap_or_default(),
                    "version" => info.get("ID_FS_VERSION").cloned().unwrap_or_default(),
                ),
            ))
        }

        if let Some(name) = info.get("DM_NAME")
            && !name.is_empty()
        {
            metrics.push(Metric::gauge_with_tags(
                "node_disk_device_mapper_info",
                "Info about disk device mapper",
                1,
                tags!(
                    "device" => device,
                    "name" => name,
                    "uuid" => info.get("DM_UUID").cloned().unwrap_or_default(),
                    "vg_name" => info.get("DM_VG_NAME").cloned().unwrap_or_default(),
                    "lv_name" => info.get("DM_LV_NAME").cloned().unwrap_or_default(),
                    "lv_layer" => info.get("DM_LV_LAYER").cloned().unwrap_or_default(),
                ),
            ))
        }

        if let Some(ata) = info.get("ID_ATA")
            && !ata.is_empty()
        {
            for (key, name, desc) in [
                (
                    "ID_ATA_WRITE_CACHE",
                    "node_disk_ata_write_cache",
                    "ATA disk has a write cache",
                ),
                (
                    "ID_ATA_WRITE_CACHE_ENABLED",
                    "node_disk_ata_write_cache_enabled",
                    "ATA disk has its write cache enabled",
                ),
                (
                    "ID_ATA_ROTATION_RATE_RPM",
                    "node_disk_ata_rotation_rate_rpm",
                    "ATA disk rotation rate in RPMs (0 for SSDs)",
                ),
            ] {
                let Some(value) = info.get(key) else {
                    debug!(message = "udev attribute does not exist", attr = key);
                    continue;
                };

                match value.parse::<f64>() {
                    Ok(value) => metrics.push(Metric::gauge_with_tags(
                        name,
                        desc,
                        value,
                        tags!("device" => device),
                    )),
                    Err(err) => {
                        warn!(
                            message = "parse ATA value failed",
                            %err
                        );

                        continue;
                    }
                }
            }
        }
    }

    Ok(metrics)
}

fn udev_device_properties(
    udev_path: &Path,
    major: &str,
    minor: &str,
) -> Result<HashMap<String, String>, Error> {
    let data = std::fs::read_to_string(udev_path.join(format!("data/b{major}:{minor}")))?;

    let mut properties = HashMap::new();
    for line in data.lines() {
        // we're only interested in device properties
        let Some(stripped) = line.strip_prefix("E:") else {
            continue;
        };

        if let Some((key, value)) = stripped.split_once("=") {
            properties.insert(key.to_string(), value.to_string());
        }
    }

    Ok(properties)
}
