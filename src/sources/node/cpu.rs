/// Collect metrics from `/proc/stat`

use crate::{
    event::{Metric, MetricValue},
    tags,
    sum_metric,
    config::{
        serialize_regex,
        deserialize_regex,
        default_false,
        default_true,
    },
};
use std::{
    sync::Arc,
    path::PathBuf,
    collections::BTreeMap,
};
use tokio::io::AsyncBufReadExt;
use serde::{Deserialize, Serialize};

const USER_HZ: f64 = 100.0;

macro_rules! state_metric {
    ($cpu: expr, $mode: expr, $value: expr) => {
        Metric{
            name: "node_cpu_seconds_total".into(),
            description: Some("Seconds the CPUs spent in each mode".into()),
            tags: tags! (
                "mode" => $mode,
                "cpu" => $cpu
            ),
            unit: None,
            timestamp: 0,
            value: MetricValue::Gauge($value)
        }
    };
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CPUConfig {
    #[serde(default = "default_true")]
    pub guest: bool,

    #[serde(default = "default_false")]
    pub info: bool,

    #[serde(default = "default_flags_include")]
    #[serde(deserialize_with = "deserialize_regex", serialize_with = "serialize_regex")]
    pub flags_include: regex::Regex,

    #[serde(default = "default_bugs_include")]
    #[serde(deserialize_with = "deserialize_regex", serialize_with = "serialize_regex")]
    pub bugs_include: regex::Regex,
}

impl Default for CPUConfig {
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

impl CPUConfig {
    pub async fn gather(&self, proc_path: Arc<String>) -> Result<Vec<Metric>, ()> {
        let proc_path = PathBuf::from(proc_path.as_ref());
        let stats = get_cpu_stat(proc_path).await?;
        let mut metrics = Vec::with_capacity(stats.len() * 10);

        for (i, stat) in stats.iter().enumerate() {
            let cpu = &i.to_string();

            metrics.push(state_metric!(cpu, "user", stat.user));
            metrics.push(state_metric!(cpu, "nice", stat.nice));
            metrics.push(state_metric!(cpu, "system", stat.system));
            metrics.push(state_metric!(cpu, "idle", stat.idle));
            metrics.push(state_metric!(cpu, "iowait", stat.iowait));
            metrics.push(state_metric!(cpu, "irq", stat.irq));
            metrics.push(state_metric!(cpu, "softirq", stat.softirq));
            metrics.push(state_metric!(cpu, "steal", stat.steal));

            // Guest CPU is also accounted for in cpuStat.User and cpuStat.Nice,
            // expose these as separate metrics.
            if self.guest {
                metrics.push(sum_metric!(
                    "node_cpu_guest_seconds_total",
                    "Seconds the CPUs spent in guests (VMs) for each mode.",
                    stat.guest,
                    "cpu" => cpu,
                    "mode" => "user"
                ));

                metrics.push(sum_metric!(
                    "node_cpu_guest_seconds_total",
                    "Seconds the CPUs spent in guests (VMs) for each mode.",
                    stat.guest_nice,
                    "cpu" => cpu,
                    "mode" => "nice"
                ));
            }
        }

        Ok(metrics)
    }
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

async fn get_cpu_stat(proc_path: PathBuf) -> Result<Vec<CPUStat>, ()> {
    let mut path = proc_path.clone();
    path.push("stat");

    let f = tokio::fs::File::open(path).await.map_err(|_| ())?;
    let reader = tokio::io::BufReader::new(f);
    let mut lines = reader.lines();
    let mut stats = Vec::new();

    while let Some(line) = lines.next_line().await.map_err(|_| ())? {
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
                _ => unreachable!()
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
        let proc_path = PathBuf::from("testdata/proc");
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