//! Collect metrics from `/proc/stat`

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use configurable::Configurable;
use event::{Metric, tags};
use framework::config::{default_true, serde_regex};
use serde::{Deserialize, Serialize};

use super::{Error, read_into};

const USER_HZ: f64 = 100.0;

#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
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
                "mode" => $mode,
                "cpu" => $cpu
            )
        )
    };
}

pub async fn gather(
    conf: Config,
    proc_path: PathBuf,
    sys_path: PathBuf,
) -> Result<Vec<Metric>, Error> {
    let mut metrics = Vec::new();

    if conf.info {
        let content = std::fs::read_to_string(proc_path.join("cpuinfo"))?;
        let infos = parse_cpu_info(&content);

        for info in infos {
            metrics.push(Metric::gauge_with_tags(
                "node_cpu_info",
                "CPU information from /proc/cpuinfo",
                1,
                tags!(
                    "package" => info.physical_id,
                    "core" => info.core_id,
                    "cpu" => info.processor,
                    "vendor" => info.vendor_id,
                    "family" => info.cpu_family,
                    "model" => info.model,
                    "model_name" => info.model_name,
                    "microcode" => info.microcode,
                    "stepping" => info.stepping,
                    "cachesize" => info.cache_size,
                ),
            ));
        }
    }

    let stats = get_cpu_stat(proc_path)?;
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
                        "cpu" => cpu,
                        "mode" => "user",
                    ),
                ),
                Metric::sum_with_tags(
                    "node_cpu_guest_seconds_total",
                    "Seconds the CPUs spent in guests (VMs) for each mode.",
                    stat.guest_nice,
                    tags!(
                        "cpu" => cpu,
                        "mode" => "nice"
                    ),
                ),
            ]);
        }
    }

    let pattern = format!(
        "{}/devices/system/cpu/cpu[0-9]*",
        sys_path.to_string_lossy()
    );
    let mut paths = glob::glob(pattern.as_str())?;

    let mut package_throttles = BTreeMap::<u64, u64>::new();
    let mut package_core_throttles = BTreeMap::<u64, BTreeMap<u64, u64>>::new();
    while let Some(Ok(path)) = paths.next() {
        let Ok(physical_package_id) = read_into(path.join("topology/physical_package_id")) else {
            continue;
        };
        let Ok(core_id) = read_into(path.join("topology/core_id")) else {
            continue;
        };

        let Ok(core_throttle_count) = read_into(path.join("thermal_throttle/core_throttle_count"))
        else {
            continue;
        };
        let counts = package_core_throttles
            .entry(physical_package_id)
            .or_default();
        counts.insert(core_id, core_throttle_count);

        let Ok(package_throttle_count) =
            read_into(path.join("thermal_throttle/package_throttle_count"))
        else {
            continue;
        };
        package_throttles.insert(physical_package_id, package_throttle_count);
    }

    for (physical_package_id, package_throttle_count) in package_throttles {
        metrics.push(Metric::sum_with_tags(
            "node_cpu_package_throttles_total",
            "Number of times this CPU package has been throttled",
            package_throttle_count,
            tags!(
                "package" => physical_package_id,
            ),
        ));
    }
    for (physical_package_id, counts) in package_core_throttles {
        for (core_id, count) in counts {
            metrics.push(Metric::sum_with_tags(
                "node_cpu_core_throttles_total",
                "Number of times this CPU core has been throttled",
                count,
                tags!(
                    "package" => physical_package_id,
                    "core" => core_id,
                ),
            ));
        }
    }

    match std::fs::read_to_string(sys_path.join("devices/system/cpu/isolated")) {
        Ok(content) => {
            let cpus = parse_cpu_range(&content)?;

            for cpu in cpus {
                metrics.push(Metric::gauge_with_tags(
                    "node_cpu_isolated",
                    "Whether each core is isolated, information from /sys/devices/system/cpu/isolated",
                    1,
                    tags!(
                        "cpu" => cpu,
                    )
                ));
            }
        }
        Err(err) => {
            if err.kind() != std::io::ErrorKind::NotFound {
                warn!(
                    message = "unable to get isolated cpus",
                    %err
                );
            }
        }
    }

    Ok(metrics)
}

#[derive(Debug, Default)]
struct CpuInfo<'a> {
    physical_id: &'a str,
    core_id: &'a str,
    processor: &'a str,
    vendor_id: &'a str,
    cpu_family: &'a str,
    model: &'a str,
    model_name: &'a str,
    microcode: &'a str,
    stepping: &'a str,
    cache_size: &'a str,
}

fn parse_cpu_info(content: &str) -> Vec<CpuInfo<'_>> {
    let mut infos = Vec::new();
    let mut cpu_info = CpuInfo::default();

    for line in content.lines() {
        let Some((key, value)) = line.split_once("\t: ") else {
            if line.is_empty() {
                infos.push(cpu_info);
                cpu_info = CpuInfo::default();
            }

            continue;
        };

        match key {
            "physical id" => {
                cpu_info.physical_id = value;
            }
            "core id\t" => {
                cpu_info.core_id = value;
            }
            "processor" => {
                cpu_info.processor = value;
            }
            "vendor" | "vendor_id" => {
                cpu_info.vendor_id = value;
            }
            "cpu family" => {
                cpu_info.cpu_family = value;
            }
            "model\t" => {
                cpu_info.model = value;
            }
            "model name" => {
                cpu_info.model_name = value;
            }
            "microcode" => {
                cpu_info.microcode = value;
            }
            "stepping" => {
                cpu_info.stepping = value;
            }
            "cache size" => {
                cpu_info.cache_size = value;
            }
            _ => {}
        }
    }

    infos
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

fn get_cpu_stat(proc_path: PathBuf) -> Result<Vec<CPUStat>, Error> {
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

fn get_isolated_cpus(root: &Path) -> Result<Vec<u16>, Error> {
    let content = std::fs::read_to_string(root.join("devices/system/cpu/isolated"))?;

    parse_cpu_range(&content)
}

fn parse_cpu_range(input: &str) -> Result<Vec<u16>, Error> {
    let mut cpus = Vec::new();

    let parts = input.trim().split(',');
    for part in parts {
        if part.is_empty() {
            continue;
        }

        match part.split_once('-') {
            Some((start, end)) => {
                let start = start
                    .parse::<u16>()
                    .map_err(|_| Error::Malformed("start of cpu range"))?;
                let end = end
                    .parse::<u16>()
                    .map_err(|_| Error::Malformed("end of cpu range"))?;

                for c in start..=end {
                    cpus.push(c);
                }
            }
            None => match part.parse::<u16>() {
                Ok(v) => cpus.push(v),
                Err(_) => return Err(Error::Malformed("cpu range")),
            },
        }
    }

    Ok(cpus)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_stats() {
        let proc_path = PathBuf::from("tests/node/proc");
        let stats = get_cpu_stat(proc_path).unwrap();

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

    #[test]
    fn cpu_info() {
        let content = std::fs::read_to_string("tests/node/proc/cpuinfo").unwrap();
        let infos = parse_cpu_info(&content);

        println!("{:#?}", infos)
    }
}
