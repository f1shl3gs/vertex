//! Exposes disk I/O statistics
//!
//! Docs from https://www.kernel.org/doc/Documentation/iostats.txt

use std::borrow::Cow;

use event::{tags, tags::Key, Metric};
use framework::config::serde_regex;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncBufReadExt;

use super::Error;

const DISK_SECTOR_SIZE: f64 = 512.0;

const DEVICE_KEY: Key = Key::from_static_str("device");

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DiskStatsConfig {
    #[serde(with = "serde_regex")]
    #[serde(default = "default_ignored")]
    pub ignored: regex::Regex,
}

impl Default for DiskStatsConfig {
    fn default() -> Self {
        Self {
            ignored: default_ignored(),
        }
    }
}

pub fn default_ignored() -> regex::Regex {
    regex::Regex::new("^(ram|loop|fd|(h|s|v|xv)d[a-z]|nvme\\d+n\\d+p)\\d+$").unwrap()
}

impl DiskStatsConfig {
    pub async fn gather(&self, root: &str) -> Result<Vec<Metric>, Error> {
        let mut metrics = Vec::new();
        let path = &format!("{}/diskstats", root);
        let f = tokio::fs::File::open(path).await?;
        let reader = tokio::io::BufReader::new(f);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await? {
            let mut parts = line.split_ascii_whitespace();
            let device = {
                let device = parts.nth(2).unwrap();
                if self.ignored.is_match(device) {
                    continue;
                }

                Cow::from(device.to_string())
            };

            // the content looks like this
            // 259       0 nvme0n1 366 0 23480 41 3 0 0 0 0 41 41 0 0 0 0
            for (index, part) in parts.enumerate() {
                let v = part.parse::<f64>().unwrap_or(0f64);
                match index {
                    0 => metrics.push(Metric::gauge_with_tags(
                        "node_disk_reads_completed_total",
                        "The total number of reads completed successfully",
                        v,
                        tags!(
                            DEVICE_KEY => device.clone()
                        ),
                    )),
                    1 => metrics.push(Metric::sum_with_tags(
                        "node_disk_reads_merged_total",
                        "The total number of reads merged",
                        v,
                        tags!(
                            DEVICE_KEY => device.clone()
                        ),
                    )),
                    2 => metrics.push(Metric::sum_with_tags(
                        "node_disk_read_bytes_total",
                        "The total number of bytes read successfully",
                        v * DISK_SECTOR_SIZE,
                        tags!(
                            DEVICE_KEY => device.clone()
                        ),
                    )),
                    3 => metrics.push(Metric::sum_with_tags(
                        "node_disk_read_time_seconds_total",
                        "The total number of seconds spent by all reads",
                        v * 0.001,
                        tags!(
                            DEVICE_KEY => device.clone()
                        ),
                    )),
                    4 => metrics.push(Metric::sum_with_tags(
                        "node_disk_writes_completed_total",
                        "The total number of writes completed successfully",
                        v,
                        tags!(
                            DEVICE_KEY => device.clone()
                        ),
                    )),
                    5 => metrics.push(Metric::sum_with_tags(
                        "node_disk_writes_merged_total",
                        "The number of writes merged.",
                        v,
                        tags!(
                            DEVICE_KEY => device.clone()
                        ),
                    )),
                    6 => metrics.push(Metric::sum_with_tags(
                        "node_disk_written_bytes_total",
                        "The total number of bytes written successfully.",
                        v * DISK_SECTOR_SIZE,
                        tags!(
                            DEVICE_KEY => device.clone()
                        ),
                    )),
                    7 => metrics.push(Metric::sum_with_tags(
                        "node_disk_write_time_seconds_total",
                        "This is the total number of seconds spent by all writes.",
                        v * 0.001,
                        tags!(
                            DEVICE_KEY => device.clone()
                        ),
                    )),
                    8 => metrics.push(Metric::gauge_with_tags(
                        "node_disk_io_now",
                        "The number of I/Os currently in progress",
                        v,
                        tags!(
                            DEVICE_KEY => device.clone()
                        ),
                    )),
                    9 => metrics.push(Metric::sum_with_tags(
                        "node_disk_io_time_seconds_total",
                        "Total seconds spent doing I/Os.",
                        v * 0.001,
                        tags!(
                            DEVICE_KEY => device.clone()
                        ),
                    )),
                    10 => metrics.push(Metric::sum_with_tags(
                        "node_disk_io_time_weighted_seconds_total",
                        "The weighted # of seconds spent doing I/Os.",
                        v * 0.001,
                        tags!(
                            DEVICE_KEY => device.clone()
                        ),
                    )),
                    11 => metrics.push(Metric::sum_with_tags(
                        "node_disk_discards_completed_total",
                        "The total number of discards completed successfully.",
                        v,
                        tags!(
                            DEVICE_KEY => device.clone()
                        ),
                    )),
                    12 => metrics.push(Metric::sum_with_tags(
                        "node_disk_discards_merged_total",
                        "The total number of discards merged.",
                        v,
                        tags!(
                            DEVICE_KEY => device.clone()
                        ),
                    )),
                    13 => metrics.push(Metric::sum_with_tags(
                        "node_disk_discarded_sectors_total",
                        "The total number of sectors discarded successfully.",
                        v,
                        tags!(
                            DEVICE_KEY => device.clone()
                        ),
                    )),
                    14 => metrics.push(Metric::sum_with_tags(
                        "node_disk_discard_time_seconds_total",
                        "This is the total number of seconds spent by all discards.",
                        v * 0.001,
                        tags!(
                            DEVICE_KEY => device.clone()
                        ),
                    )),
                    15 => metrics.push(Metric::sum_with_tags(
                        "node_disk_flush_requests_total",
                        "The total number of flush requests completed successfully",
                        v,
                        tags!(
                            DEVICE_KEY => device.clone()
                        ),
                    )),
                    16 => metrics.push(Metric::sum_with_tags(
                        "node_disk_flush_requests_time_seconds_total",
                        "This is the total number of seconds spent by all flush requests.",
                        v * 0.001,
                        tags!(
                            DEVICE_KEY => device.clone()
                        ),
                    )),
                    _ => {}
                }
            }
        }

        Ok(metrics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gather() {
        let proc_path = "tests/fixtures/proc";
        let collector = DiskStatsConfig {
            ignored: default_ignored(),
        };

        let result = collector.gather(proc_path).await.unwrap();
        assert_ne!(result.len(), 0);
    }
}
