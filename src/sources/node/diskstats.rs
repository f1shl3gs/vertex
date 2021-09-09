/// Exposes disk I/O statistics
///
/// Docs from https://www.kernel.org/doc/Documentation/iostats.txt

use serde::{Deserialize, Serialize};
use crate::{
    tags,
    sum_metric,
    gauge_metric,
    event::{Metric, MetricValue},
    config::{deserialize_regex, serialize_regex},
};
use tokio::io::{AsyncBufReadExt};
use crate::sources::node::errors::{Error, ErrorContext};

const DISK_SECTOR_SIZE: f64 = 512.0;

#[derive(Debug, Deserialize, Serialize)]
pub struct DiskStatsConfig {
    #[serde(deserialize_with = "deserialize_regex", serialize_with = "serialize_regex")]
    #[serde(default = "default_ignored")]
    pub ignored: regex::Regex,
}

impl Default for DiskStatsConfig {
    fn default() -> Self {
        Self {
            ignored: default_ignored()
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
        let f = tokio::fs::File::open(path).await
            .context("open diskstats failed")?;
        let reader = tokio::io::BufReader::new(f);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await? {
            let mut parts = line.split_ascii_whitespace();

            let device = parts.nth(2).unwrap();
            if self.ignored.is_match(device) {
                continue;
            }

            // the content looks like this
            // 259       0 nvme0n1 366 0 23480 41 3 0 0 0 0 41 41 0 0 0 0
            for (index, part) in parts.enumerate() {
                let v = part.parse::<f64>().unwrap_or(0f64);
                match index {
                    0 => metrics.push(gauge_metric!(
                        "node_disk_reads_completed_total",
                        "The total number of reads completed successfully",
                        v,
                        "device" => device
                    )),
                    1 => metrics.push(sum_metric!(
                        "node_disk_reads_merged_total",
                        "The total number of reads merged",
                        v,
                        "device" => device
                    )),
                    2 => metrics.push(sum_metric!(
                        "node_disk_read_bytes_total",
                        "The total number of bytes read successfully",
                        v * DISK_SECTOR_SIZE,
                        "device" => device
                    )),
                    3 => metrics.push(sum_metric!(
                        "node_disk_read_time_seconds_total",
                        "The total number of seconds spent by all reads",
                        v * 0.001,
                        "device" => device
                    )),
                    4 => metrics.push(sum_metric!(
                        "node_disk_writes_completed_total",
                        "The total number of writes completed successfully",
                        v,
                        "device" => device
                    )),
                    5 => metrics.push(sum_metric!(
                        "node_disk_writes_merged_total",
                        "The number of writes merged.",
                        v,
                        "device" => device
                    )),
                    6 => metrics.push(sum_metric!(
                        "node_disk_written_bytes_total",
                        "The total number of bytes written successfully.",
                        v * DISK_SECTOR_SIZE,
                        "device" => device
                    )),
                    7 => metrics.push(sum_metric!(
                        "node_disk_write_time_seconds_total",
                        "This is the total number of seconds spent by all writes.",
                        v * 0.001,
                        "device" => device
                    )),
                    8 => metrics.push(gauge_metric!(
                        "node_disk_io_now",
                        "The number of I/Os currently in progress",
                        v,
                        "device" => device
                    )),
                    9 => metrics.push(sum_metric!(
                        "node_disk_io_time_seconds_total",
                        "Total seconds spent doing I/Os.",
                        v * 0.001,
                        "device" => device
                    )),
                    10 => metrics.push(sum_metric!(
                        "node_disk_io_time_weighted_seconds_total",
                        "The weighted # of seconds spent doing I/Os.",
                        v * 0.001,
                        "device" => device
                    )),
                    11 => metrics.push(sum_metric!(
                        "node_disk_discards_completed_total",
                        "The total number of discards completed successfully.",
                        v,
                        "device" => device
                    )),
                    12 => metrics.push(sum_metric!(
                        "node_disk_discards_merged_total",
                        "The total number of discards merged.",
                        v,
                        "device" => device
                    )),
                    13 => metrics.push(sum_metric!(
                        "node_disk_discarded_sectors_total",
                        "The total number of sectors discarded successfully.",
                        v,
                        "device" => device
                    )),
                    14 => metrics.push(sum_metric!(
                        "node_disk_discard_time_seconds_total",
                        "This is the total number of seconds spent by all discards.",
                        v * 0.001,
                        "device" => device
                    )),
                    15 => metrics.push(sum_metric!(
                        "node_disk_flush_requests_total",
                        "The total number of flush requests completed successfully",
                        v,
                        "device" => device
                    )),
                    16 => metrics.push(sum_metric!(
                        "node_disk_flush_requests_time_seconds_total",
                        "This is the total number of seconds spent by all flush requests.",
                        v * 0.001,
                        "device" => device
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
        let proc_path = "testdata/proc";
        let collector = DiskStatsConfig {
            ignored: default_ignored()
        };

        let result = collector.gather(proc_path).await.unwrap();
        assert_ne!(result.len(), 0);
    }
}