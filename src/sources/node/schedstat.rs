//! Exposes task scheduler statistics from /proc/schedstat
use event::{tags, Metric};
use tokio::io::AsyncBufReadExt;

use super::{Error, ErrorContext};

#[derive(Debug, Default)]
struct Schedstat {
    cpu: String,

    running_nanoseconds: u64,
    waiting_nanoseconds: u64,
    run_time_slices: u64,
}

pub async fn gather(proc_path: &str) -> Result<Vec<Metric>, Error> {
    let stats = schedstat(proc_path)
        .await
        .context("read schedstat failed")?;

    let mut metrics = Vec::with_capacity(3 * stats.len());
    for stat in stats {
        let cpu = &stat.cpu;

        metrics.push(Metric::sum_with_tags(
            "node_schedstat_running_seconds_total",
            "Number of seconds CPU spent running a process.",
            stat.running_nanoseconds,
            tags!(
                "cpu" => cpu
            ),
        ));

        metrics.push(Metric::sum_with_tags(
            "node_schedstat_waiting_seconds_total",
            "Number of seconds spent by processing waiting for this CPU.",
            stat.waiting_nanoseconds,
            tags!(
                "cpu" => cpu,
            ),
        ));

        metrics.push(Metric::sum_with_tags(
            "node_schedstat_timeslices_total",
            "Number of timeslices executed by CPU.",
            stat.run_time_slices,
            tags!(
                "cpu" => cpu
            ),
        ))
    }

    Ok(metrics)
}

async fn schedstat(proc_path: &str) -> Result<Vec<Schedstat>, Error> {
    let path = format!("{}/schedstat", proc_path);
    let f = tokio::fs::File::open(path).await?;
    let r = tokio::io::BufReader::new(f);
    let mut lines = r.lines();

    let mut stats = Vec::new();
    while let Some(line) = lines.next_line().await? {
        if !line.starts_with("cpu") {
            continue;
        }

        let fields = line.split_ascii_whitespace().collect::<Vec<_>>();

        if fields.len() < 10 {
            continue;
        }

        let cpu = fields[0].strip_prefix("cpu").unwrap();
        let running_nanoseconds = match fields[7].parse() {
            Ok(v) => v,
            _ => continue,
        };

        let waiting_nanoseconds = match fields[8].parse() {
            Ok(v) => v,
            _ => continue,
        };

        let run_time_slices = match fields[9].parse() {
            Ok(v) => v,
            _ => continue,
        };

        stats.push(Schedstat {
            cpu: cpu.to_string(),
            running_nanoseconds,
            waiting_nanoseconds,
            run_time_slices,
        })
    }

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_schedstat() {
        let path = "tests/fixtures/proc";
        let stats = schedstat(path).await.unwrap();

        assert_ne!(stats.len(), 0);
        let stat = &stats[0];

        assert_eq!(stat.cpu, "0");
        assert_eq!(stat.running_nanoseconds, 2045936778163039);
        assert_eq!(stat.waiting_nanoseconds, 343796328169361);
        assert_eq!(stat.run_time_slices, 4767485306)
    }
}
