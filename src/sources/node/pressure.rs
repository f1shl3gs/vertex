#![allow(dead_code)]

//! The PSI / pressure interface is described at
//!   https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/tree/Documentation/accounting/psi.txt
//! Each resource (cpu, io, memory, ...) is exposed as a single file.
//! Each file may contain up to two lines, one for "some" pressure and one for "full" pressure.
//! Each line contains several averages (over n seconds) and a total in µs.
//!
//! Example io pressure file:
//! > some avg10=0.06 avg60=0.21 avg300=0.99 total=8537362
//! > full avg10=0.00 avg60=0.13 avg300=0.96 total=8183134

use std::path::PathBuf;

use event::Metric;

use super::Error;

pub async fn gather(proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let cpu = psi_stats(proc_path.join("pressure/cpu")).await?;
    let io = psi_stats(proc_path.join("pressure/io")).await?;
    let memory = psi_stats(proc_path.join("pressure/memory")).await?;
    let irq = psi_stats(proc_path.join("pressure/irq")).await?;

    let mut metrics = Vec::with_capacity(6);
    if let Some(some) = cpu.some {
        metrics.push(Metric::sum(
            "node_pressure_cpu_waiting_seconds_total",
            "Total time in seconds that processes have waited for CPU time",
            some.total as f64 / 1000.0 / 1000.0,
        ));
    }

    if let Some(some) = io.some {
        metrics.push(Metric::sum(
            "node_pressure_io_waiting_seconds_total",
            "Total time in seconds that processes have waited due to IO congestion",
            some.total as f64 / 1000.0 / 1000.0,
        ));
    }

    if let Some(full) = io.full {
        metrics.push(Metric::sum(
            "node_pressure_io_stalled_seconds_total",
            "Total time in seconds no process could make progress due to IO congestion",
            full.total as f64 / 1000.0 / 1000.0,
        ));
    }

    if let Some(some) = memory.some {
        metrics.push(Metric::sum(
            "node_pressure_memory_waiting_seconds_total",
            "Total time in seconds that processes have waited for memory",
            some.total as f64 / 1000.0 / 1000.0,
        ));
    }

    if let Some(full) = memory.full {
        metrics.push(Metric::sum(
            "node_pressure_memory_stalled_seconds_total",
            "Total time in seconds no process could make progress due to memory congestion",
            full.total as f64 / 1000.0 / 1000.0,
        ));
    }

    if let Some(full) = irq.full {
        metrics.push(Metric::sum(
            "node_pressure_irq_stalled_seconds_total",
            "Total time in seconds no process could make progress due to IRQ congestion",
            full.total as f64 / 1000.0 / 1000.0,
        ));
    }

    Ok(metrics)
}

/// PSIStat is a single line of values as returned by /proc/pressure/*
/// The avg entries are averages over n seconds, as a percentage
/// The total line is in microseconds
struct PSIStat {
    avg10: f64,
    avg60: f64,
    avg300: f64,
    total: u64,
}

/// PSIStats represent pressure stall information from /proc/pressure/*
/// some indicates the share of time in which at least some tasks are stalled
/// full indicates the share of time in which all non-idle tasks are stalled simultaneously
struct PSIStats {
    some: Option<PSIStat>,
    full: Option<PSIStat>,
}

async fn psi_stats(path: PathBuf) -> Result<PSIStats, Error> {
    let data = std::fs::read_to_string(path)?;
    let mut stats = PSIStats {
        some: None,
        full: None,
    };

    for line in data.lines() {
        let stat = parse_psi_stat(line)?;

        if line.starts_with("some") {
            stats.some = Some(stat);
            continue;
        }

        if line.starts_with("full") {
            stats.full = Some(stat);
        }
    }

    Ok(stats)
}

fn parse_psi_stat(line: &str) -> Result<PSIStat, Error> {
    // some of full
    let parts = line.split_ascii_whitespace().collect::<Vec<_>>();

    if parts.len() != 5 {
        return Err(Error::from("malformed psi stat line"));
    }

    let avg10 = parts[1].strip_prefix("avg10=").unwrap().parse()?;
    let avg60 = parts[2].strip_prefix("avg60=").unwrap().parse()?;
    let avg300 = parts[3].strip_prefix("avg300=").unwrap().parse()?;
    let total = parts[4].strip_prefix("total=").unwrap().parse()?;

    Ok(PSIStat {
        avg10,
        avg60,
        avg300,
        total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_psi_stat() {
        let line = "full avg10=0.20 avg60=3.00 avg300=4.95 total=25";
        let stat = parse_psi_stat(line).unwrap();
        assert_eq!(stat.avg10, 0.20);
        assert_eq!(stat.avg60, 3.00);
        assert_eq!(stat.avg300, 4.95);
        assert_eq!(stat.total, 25);
    }
}
