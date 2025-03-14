//! Collect metrics from `/proc/stat`

use std::path::PathBuf;

use event::{Metric, tags, tags::Key};
use framework::config::{default_true, serde_regex};
use serde::{Deserialize, Serialize};

use super::Error;

const USER_HZ: f64 = 100.0;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "default_true")]
    guest: bool,

    #[serde(default)]
    info: bool,

    #[serde(default = "default_flags_include")]
    #[serde(with = "serde_regex")]
    flags_include: regex::Regex,

    #[serde(default = "default_bugs_include")]
    #[serde(with = "serde_regex")]
    bugs_include: regex::Regex,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            guest: true,
            info: false,
            flags_include: default_flags_include(),
            bugs_include: default_bugs_include(),
        }
    }
}

fn default_flags_include() -> regex::Regex {
    regex::Regex::new(".*").unwrap()
}

fn default_bugs_include() -> regex::Regex {
    regex::Regex::new(".*").unwrap()
}

macro_rules! state_metric {
    ($cpu: expr, $mode: expr, $value: expr) => {
        Metric::sum_with_tags(
            "node_cpu_seconds_total",
            "Seconds the CPUs spent in each mode",
            $value,
            tags! (
                Key::from_static("mode") => $mode,
                Key::from_static("cpu") => $cpu as i64
            )
        )
    };
}

pub async fn gather(conf: Config, proc_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let stats = get_cpu_stat(proc_path).await?;

    let mut metrics = Vec::with_capacity(stats.len() * 10);
    for (cpu, stat) in stats.iter().enumerate() {
        metrics.extend([
            state_metric!(cpu, "user", stat.user),
            state_metric!(cpu, "nice", stat.nice),
            state_metric!(cpu, "system", stat.system),
            state_metric!(cpu, "idle", stat.idle),
            state_metric!(cpu, "iowait", stat.iowait),
            state_metric!(cpu, "irq", stat.irq),
            state_metric!(cpu, "softirq", stat.softirq),
            state_metric!(cpu, "steal", stat.steal),
        ]);

        // Guest CPU is also accounted for in cpuStat.User and cpuStat.Nice,
        // expose these as separate metrics.
        if conf.guest {
            metrics.extend([
                Metric::sum_with_tags(
                    "node_cpu_guest_seconds_total",
                    "Seconds the CPUs spent in guests (VMs) for each mode.",
                    stat.guest,
                    tags!(
                        Key::from_static("cpu") => cpu as i64,
                        Key::from_static("mode") => "user",
                    ),
                ),
                Metric::sum_with_tags(
                    "node_cpu_guest_seconds_total",
                    "Seconds the CPUs spent in guests (VMs) for each mode.",
                    stat.guest_nice,
                    tags!(
                        Key::from_static("cpu") => cpu as i64,
                        Key::from_static("mode") => "nice"
                    ),
                ),
            ]);
        }
    }

    Ok(metrics)
}

#[derive(Default)]
struct CPUStat {
    user: f64,
    nice: f64,
    system: f64,
    idle: f64,
    iowait: f64,
    irq: f64,
    softirq: f64,
    steal: f64,
    guest: f64,
    guest_nice: f64,
}

async fn get_cpu_stat(proc_path: PathBuf) -> Result<Vec<CPUStat>, Error> {
    let data = std::fs::read_to_string(proc_path.join("stat"))?;

    let mut stats = Vec::new();
    for line in data.lines() {
        if !line.starts_with("cpu") {
            continue;
        }

        if line.starts_with("cpu ") {
            continue;
        }

        let parts = line.split_ascii_whitespace();
        let mut stat = CPUStat::default();

        for (index, part) in parts.enumerate().skip(1) {
            let v = part.parse().unwrap_or(0f64) / USER_HZ;

            match index {
                1 => stat.user = v,
                2 => stat.nice = v,
                3 => stat.system = v,
                4 => stat.idle = v,
                5 => stat.iowait = v,
                6 => stat.irq = v,
                7 => stat.softirq = v,
                8 => stat.steal = v,
                9 => stat.guest = v,
                10 => stat.guest_nice = v,
                _ => unreachable!(),
            }
        }

        stats.push(stat);
    }

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_cpu_stats() {
        let proc_path = PathBuf::from("tests/node/proc");
        let stats = get_cpu_stat(proc_path).await.unwrap();

        assert_eq!(stats.len(), 8);
        assert_eq!(31f64 / USER_HZ, stats[7].softirq);
        assert_eq!(1f64 / USER_HZ, stats[0].irq);
        assert_eq!(47869f64 / USER_HZ, stats[1].user);
        assert_eq!(23f64 / USER_HZ, stats[1].nice);
        assert_eq!(15916f64 / USER_HZ, stats[2].system);
        assert_eq!(1113230f64 / USER_HZ, stats[3].idle);
        assert_eq!(217f64 / USER_HZ, stats[4].iowait);
        assert_eq!(0f64 / USER_HZ, stats[5].irq);
        assert_eq!(29f64 / USER_HZ, stats[6].softirq);
        assert_eq!(0f64, stats[7].steal);
        assert_eq!(0f64, stats[7].guest);
        assert_eq!(0f64, stats[7].guest_nice);
    }
}
