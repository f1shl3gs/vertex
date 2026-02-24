//! Exposes statistics from /proc/net/softnet_stat
//!
//! For the proc file format details,
//! See:
//! * Linux 2.6.23 https://elixir.bootlin.com/linux/v2.6.23/source/net/core/dev.c#L2343
//! * Linux 4.17 https://elixir.bootlin.com/linux/v4.17/source/net/core/net-procfs.c#L162
//!   and https://elixir.bootlin.com/linux/v4.17/source/include/linux/netdevice.h#L2810.

use std::path::PathBuf;

use event::{Metric, tags};

use super::Error;

pub async fn gather(proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let data = std::fs::read_to_string(proc_path.join("net/softnet_stat"))?;
    let mut metrics = Vec::new();

    for (index, line) in data.lines().enumerate() {
        if let Ok(stat) = parse_softnet(line, index as u32) {
            let tags = tags!("cpu" => index);

            metrics.extend([
                Metric::sum_with_tags(
                    "node_softnet_processed_total",
                    "Number of processed packets",
                    stat.processed,
                    tags.clone(),
                ),
                Metric::sum_with_tags(
                    "node_softnet_dropped_total",
                    "Number of dropped packets",
                    stat.dropped,
                    tags.clone(),
                ),
                Metric::sum_with_tags(
                    "node_softnet_times_squeezed_total",
                    "Number of times processing packets ran out of quota",
                    stat.time_squeezed,
                    tags.clone(),
                ),
                Metric::sum_with_tags(
                    "node_softnet_cpu_collision_total",
                    "Number of collision occur while obtaining device lock while transmitting",
                    stat.cpu_collision,
                    tags.clone(),
                ),
                Metric::sum_with_tags(
                    "node_softnet_received_rps_total",
                    "Number of times cpu woken up received_rps",
                    stat.received_rps,
                    tags.clone(),
                ),
                Metric::sum_with_tags(
                    "node_softnet_flow_limit_count_total",
                    "Number of times flow limit has been reached",
                    stat.flow_limit_count,
                    tags.clone(),
                ),
                Metric::gauge_with_tags(
                    "node_softnet_backlog_len",
                    "Softnet backlog status",
                    stat.softnet_backlog_len,
                    tags,
                ),
            ]);
        }
    }

    Ok(metrics)
}

// SoftnetStat contains a single row of data from /proc/net/softnet_stat
#[derive(Debug, Default)]
struct SoftnetStat {
    // Number of processed packets
    processed: u32,
    // Number of dropped packets
    dropped: u32,
    // Number of times processing packets ran out of quota
    time_squeezed: u32,
    // Number of collision occur while obtaining device lock while transmitting.
    cpu_collision: u32,
    // Number of times cpu woken up received_rps.
    received_rps: u32,
    // number of times flow limit has been reached.
    flow_limit_count: u32,
    // Softnet backlog status.
    softnet_backlog_len: u32,
    // CPU id owning this softnet_data.
    index: u32,
    // softnet_data's Width.
    width: i16,
}

fn parse_softnet(line: &str, index: u32) -> Result<SoftnetStat, Error> {
    const MIN_COLUMNS: usize = 9;

    let parts = line.split_ascii_whitespace().collect::<Vec<_>>();

    if parts.len() < MIN_COLUMNS {
        return Err(Error::from(format!(
            "{} columns were detected, but at least {} were expected",
            parts.len(),
            MIN_COLUMNS,
        )));
    }

    let mut stat = SoftnetStat::default();
    // Linux 2.6.23 https://elixir.bootlin.com/linux/v2.6.23/source/net/core/dev.c#L2347
    if parts.len() >= MIN_COLUMNS {
        stat.processed = hex_u32(parts[0].as_bytes());
        stat.dropped = hex_u32(parts[1].as_bytes());
        stat.time_squeezed = hex_u32(parts[2].as_bytes());
        stat.cpu_collision = hex_u32(parts[8].as_bytes());
    }

    // Linux 2.6.39 https://elixir.bootlin.com/linux/v2.6.39/source/net/core/dev.c#L4086
    if parts.len() >= 10 {
        stat.received_rps = hex_u32(parts[9].as_bytes());
    }

    // Linux 4.18 https://elixir.bootlin.com/linux/v4.18/source/net/core/net-procfs.c#L162
    if parts.len() >= 11 {
        stat.flow_limit_count = hex_u32(parts[10].as_bytes());
    }

    // Linux 5.14 https://elixir.bootlin.com/linux/v5.14/source/net/core/net-procfs.c#L169
    if parts.len() >= 13 {
        stat.softnet_backlog_len = hex_u32(parts[11].as_bytes());
        stat.index = hex_u32(parts[12].as_bytes());
    } else {
        // for older kernels, create the index based on the scan line number.
        stat.index = index;
    }

    stat.width = parts.len() as i16;

    Ok(stat)
}

#[inline]
fn hex_u32(input: &[u8]) -> u32 {
    input
        .iter()
        .rev()
        .enumerate()
        .map(|(k, &v)| {
            let digit = v as char;
            (digit.to_digit(16).unwrap_or(0)) << (k * 4)
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bad_softnet_line() {
        let line = "00015c73 00020e76 F0000769 00000000\n";

        assert!(parse_softnet(line, 0).is_err());
    }

    #[test]
    fn test_parse_softnet() {
        let line = "00015c73 00020e76 F0000769 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000\n";

        let stat = parse_softnet(line, 0).unwrap();
        assert_eq!(stat.processed, 0x00015c73);
        assert_eq!(stat.dropped, 0x00020e76);
        assert_eq!(stat.time_squeezed, 0xf0000769);
    }
}
