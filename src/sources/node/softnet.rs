//! Exposes statistics from /proc/net/softnet_stat
//!
//! For the proc file format details,
//! See:
//! * Linux 2.6.23 https://elixir.bootlin.com/linux/v2.6.23/source/net/core/dev.c#L2343
//! * Linux 4.17 https://elixir.bootlin.com/linux/v4.17/source/net/core/net-procfs.c#L162
//!   and https://elixir.bootlin.com/linux/v4.17/source/include/linux/netdevice.h#L2810.

use std::path::PathBuf;

use event::{tags, Metric};
use tokio::io::AsyncBufReadExt;

use super::Error;

// SoftnetStat contains a single row of data from /proc/net/softnet_stat
struct SoftnetStat {
    // Number of processed packets
    processed: u32,

    // Number of dropped packets
    dropped: u32,

    // Number of times processing packets ran out of quota
    time_squeezed: u32,
}

pub async fn gather(proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let file = tokio::fs::File::open(proc_path.join("net/softnet_stat"))
        .await
        .map_err(|err| Error::Io {
            msg: "open softnet_stat failed".into(),
            err,
        })?;
    let r = tokio::io::BufReader::new(file);
    let mut lines = r.lines();

    let mut metrics = Vec::new();
    let mut n = 0;
    while let Some(line) = lines.next_line().await? {
        if let Ok(stat) = parse_softnet(&line) {
            let tags = tags!("cpu" => n);
            n += 1;

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
                    tags,
                ),
            ]);
        }
    }

    Ok(metrics)
}

fn parse_softnet(s: &str) -> Result<SoftnetStat, Error> {
    const MIN_COLUMNS: usize = 9;
    let parts = s.split_ascii_whitespace().collect::<Vec<_>>();

    if parts.len() < MIN_COLUMNS {
        return Err(Error::from(format!(
            "{} columns were detected, but at least {} were expected",
            parts.len(),
            MIN_COLUMNS,
        )));
    }

    let processed = hex_u32(parts[0].as_bytes());
    let dropped = hex_u32(parts[1].as_bytes());
    let time_squeezed = hex_u32(parts[2].as_bytes());

    Ok(SoftnetStat {
        processed,
        dropped,
        time_squeezed,
    })
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

        assert!(parse_softnet(line).is_err());
    }

    #[test]
    fn test_parse_softnet() {
        let line = "00015c73 00020e76 F0000769 00000000 00000000 00000000 00000000 00000000 00000000 00000000 00000000\n";

        let stat = parse_softnet(line).unwrap();
        assert_eq!(stat.processed, 0x00015c73);
        assert_eq!(stat.dropped, 0x00020e76);
        assert_eq!(stat.time_squeezed, 0xf0000769);
    }
}
