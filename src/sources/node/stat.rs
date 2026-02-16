//! Exposes various statistics from /proc/stat. This includes boot time, forks and interrupts.

use std::path::PathBuf;

use event::Metric;

use super::Error;

pub async fn gather(proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let stat = read_stat(proc_path).await?;

    Ok(vec![
        Metric::sum(
            "node_intr_total",
            "Total number of interrupts serviced.",
            stat.intr,
        ),
        Metric::sum(
            "node_context_switches_total",
            "Total number of context switches.",
            stat.ctxt,
        ),
        Metric::sum("node_forks_total", "Total number of forks.", stat.forks),
        Metric::gauge(
            "node_boot_time_seconds",
            "Node boot time, in unixtime.",
            stat.btime,
        ),
        Metric::gauge(
            "node_procs_running",
            "Number of processes in runnable state.",
            stat.procs_running,
        ),
        Metric::gauge(
            "node_procs_blocked",
            "Number of processes blocked waiting for I/O to complete.",
            stat.procs_blocked,
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

async fn read_stat(proc_path: PathBuf) -> Result<Stat, Error> {
    let data = std::fs::read_to_string(proc_path.join("stat"))?;

    let mut stat = Stat::default();
    for line in data.lines() {
        if let Some(stripped) = line.strip_prefix("ctxt ") {
            stat.ctxt = stripped.parse().unwrap_or(0u64);
            continue;
        }

        if let Some(stripped) = line.strip_prefix("btime ") {
            stat.btime = stripped.parse().unwrap_or(0u64);
            continue;
        }

        if let Some(stripped) = line.strip_prefix("intr ") {
            let mut parts = stripped.split_ascii_whitespace();
            if let Some(part) = parts.next() {
                stat.intr = part.parse::<u64>().unwrap_or(0u64);
            }

            continue;
        }

        if let Some(stripped) = line.strip_prefix("processes ") {
            stat.forks = stripped.parse().unwrap_or(0u64);
            continue;
        }

        if let Some(stripped) = line.strip_prefix("procs_running ") {
            stat.procs_running = stripped.parse().unwrap_or(0u64);
            continue;
        }

        if let Some(stripped) = line.strip_prefix("procs_blocked ") {
            stat.procs_blocked = stripped.parse().unwrap_or(0u64);
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
        let proc = "tests/node/proc".into();
        let stat = read_stat(proc).await.unwrap();

        assert_eq!(stat.ctxt, 38014093);
        assert_eq!(stat.btime, 1418183276);
        assert_eq!(stat.forks, 26442);
        assert_eq!(stat.procs_running, 2);
        assert_eq!(stat.procs_blocked, 1);
    }
}
