//! Exposes task scheduler statistics from /proc/schedstat

use std::path::PathBuf;

use event::{Metric, tags};

use super::Error;

#[derive(Debug, Default)]
struct SchedStat<'a> {
    cpu: &'a str,

    running_nanoseconds: u64,
    waiting_nanoseconds: u64,
    run_time_slices: u64,
}

pub async fn gather(proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let content = std::fs::read_to_string(proc_path.join("schedstat"))?;

    let mut metrics = Vec::new();
    for line in content.lines() {
        let Some(stripped) = line.strip_prefix("cpu") else {
            continue;
        };

        let Some(stat) = parse_sched_state(stripped) else {
            warn!(message = "invalid schedstat line", line);
            continue;
        };

        let tags = tags!("cpu" => stat.cpu);
        metrics.extend([
            Metric::sum_with_tags(
                "node_schedstat_running_seconds_total",
                "Number of seconds CPU spent running a process.",
                stat.running_nanoseconds,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_schedstat_waiting_seconds_total",
                "Number of seconds spent by processing waiting for this CPU.",
                stat.waiting_nanoseconds,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_schedstat_timeslices_total",
                "Number of timeslices executed by CPU.",
                stat.run_time_slices,
                tags,
            ),
        ]);
    }

    Ok(metrics)
}

fn parse_sched_state(line: &str) -> Option<SchedStat<'_>> {
    let fields = line.split_ascii_whitespace().collect::<Vec<_>>();
    if fields.len() < 10 {
        return None;
    }

    let Ok(running_nanoseconds) = fields[7].parse() else {
        return None;
    };

    let Ok(waiting_nanoseconds) = fields[8].parse() else {
        return None;
    };

    let Ok(run_time_slices) = fields[9].parse() else {
        return None;
    };

    Some(SchedStat {
        cpu: fields[0],
        running_nanoseconds,
        waiting_nanoseconds,
        run_time_slices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let line = "0 498494191 0 3533438552 2553969831 3853684107 2465731542 2045936778163039 343796328169361 4767485306";
        let stat = parse_sched_state(line).unwrap();

        assert_eq!(stat.cpu, "0");
        assert_eq!(stat.running_nanoseconds, 2045936778163039);
        assert_eq!(stat.waiting_nanoseconds, 343796328169361);
        assert_eq!(stat.run_time_slices, 4767485306)
    }
}
