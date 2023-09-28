//! Exposes various statistics from /proc/stat. This includes boot time, forks and interrupts.

use event::Metric;

use super::{read_to_string, Error};

pub async fn gather(proc_path: &str) -> Result<Vec<Metric>, Error> {
    let stat = read_stat(proc_path).await?;

    Ok(vec![
        Metric::sum(
            "node_intr_total",
            "Total number of interrupts serviced.",
            stat.intr as f64,
        ),
        Metric::sum(
            "node_context_switches_total",
            "Total number of context switches.",
            stat.ctxt as f64,
        ),
        Metric::sum(
            "node_forks_total",
            "Total number of forks.",
            stat.forks as f64,
        ),
        Metric::gauge(
            "node_boot_time_seconds",
            "Node boot time, in unixtime.",
            stat.btime as f64,
        ),
        Metric::gauge(
            "node_procs_running",
            "Number of processes in runnable state.",
            stat.procs_running as f64,
        ),
        Metric::gauge(
            "node_procs_blocked",
            "Number of processes blocked waiting for I/O to complete.",
            stat.procs_blocked as f64,
        ),
    ])
}

#[derive(Default)]
struct Stat {
    intr: u64,
    ctxt: u64,
    forks: u64,
    btime: u64,
    procs_running: u64,
    procs_blocked: u64,
}

async fn read_stat(proc_path: &str) -> Result<Stat, Error> {
    let path = format!("{}/stat", proc_path);
    let content = read_to_string(path).await?;

    let mut stat = Stat::default();
    for line in content.lines() {
        if line.starts_with("ctxt ") {
            stat.ctxt = line.strip_prefix("ctxt ").unwrap().parse().unwrap_or(0u64);
            continue;
        }

        if line.starts_with("btime ") {
            stat.btime = line.strip_prefix("btime ").unwrap().parse().unwrap_or(0);
            continue;
        }

        if line.starts_with("processes ") {
            stat.forks = line
                .strip_prefix("processes ")
                .unwrap()
                .parse()
                .unwrap_or(0);
            continue;
        }

        if line.starts_with("procs_running ") {
            stat.procs_running = line
                .strip_prefix("procs_running ")
                .unwrap()
                .parse()
                .unwrap_or(0);
            continue;
        }

        if line.starts_with("procs_blocked ") {
            stat.procs_blocked = line
                .strip_prefix("procs_blocked ")
                .unwrap()
                .parse()
                .unwrap_or(0);
            continue;
        }
    }

    Ok(stat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_stat() {
        let proc = "tests/fixtures/proc";
        let stat = read_stat(proc).await.unwrap();

        assert_eq!(stat.ctxt, 38014093);
        assert_eq!(stat.btime, 1418183276);
        assert_eq!(stat.forks, 26442);
        assert_eq!(stat.procs_running, 2);
        assert_eq!(stat.procs_blocked, 1);
    }
}
