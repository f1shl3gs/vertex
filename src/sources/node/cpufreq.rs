use std::path::{Path, PathBuf};

use event::{Metric, tags};

use super::{Error, Paths, read_into, read_sys_file};

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let stats = get_cpu_freq_stat(paths.sys())?;

    let mut metrics = Vec::with_capacity(stats.len() * 6);
    for (cpu, stat) in stats {
        if let Some(v) = stat.current_frequency {
            metrics.push(Metric::gauge_with_tags(
                "node_cpu_frequency_hertz",
                "Current cpu thread frequency in hertz.",
                v * 1000,
                tags!(
                    "cpu" => cpu,
                ),
            ));
        }

        if let Some(v) = stat.minimum_frequency {
            metrics.push(Metric::gauge_with_tags(
                "node_cpu_frequency_min_hertz",
                "Minimum cpu thread frequency in hertz.",
                v * 1000,
                tags!(
                    "cpu" => cpu,
                ),
            ));
        }

        if let Some(v) = stat.maximum_frequency {
            metrics.push(Metric::gauge_with_tags(
                "node_cpu_frequency_max_hertz",
                "Maximum CPU thread frequency in hertz.",
                v * 1000,
                tags!(
                    "cpu" => cpu,
                ),
            ))
        }

        if let Some(v) = stat.scaling_current_frequency {
            metrics.push(Metric::gauge_with_tags(
                "node_cpu_scaling_frequency_hertz",
                "Current scaled CPU thread frequency in hertz.",
                v * 1000,
                tags!(
                    "cpu" => cpu,
                ),
            ))
        }

        if let Some(v) = stat.scaling_minimum_frequency {
            metrics.push(Metric::gauge_with_tags(
                "node_cpu_scaling_frequency_min_hertz",
                "Minimum scaled CPU thread frequency in hertz.",
                v as f64 * 1000.0,
                tags!(
                    "cpu" => cpu,
                ),
            ));
        }

        if let Some(v) = stat.scaling_maximum_frequency {
            metrics.push(Metric::gauge_with_tags(
                "node_cpu_scaling_frequency_max_hertz",
                "Maximum scaled CPU thread frequency in hertz.",
                v as f64 * 1000.0,
                tags!(
                    "cpu" => cpu,
                ),
            ));
        }

        if !stat.governor.is_empty() {
            for governor in stat.available_governors.split_ascii_whitespace() {
                metrics.push(Metric::gauge_with_tags(
                    "node_cpu_scaling_governor",
                    "Current enabled CPU frequency governor.",
                    governor == stat.governor,
                    tags!(
                        "cpu" => cpu,
                        "governor" =>  governor
                    ),
                ))
            }
        }
    }

    Ok(metrics)
}

#[derive(Default, Debug, PartialEq)]
struct Stat {
    current_frequency: Option<u64>,
    minimum_frequency: Option<u64>,
    maximum_frequency: Option<u64>,
    transition_latency: Option<u64>,
    scaling_current_frequency: Option<u64>,
    scaling_minimum_frequency: Option<u64>,
    scaling_maximum_frequency: Option<u64>,

    available_governors: String,
    driver: String,
    governor: String,
    related_cpus: String,
    set_speed: String,
}

fn get_cpu_freq_stat(root: &Path) -> Result<Vec<(usize, Stat)>, Error> {
    let mut stats = Vec::new();
    let root = root.join("devices/system/cpu");

    for entry in root.read_dir()?.flatten() {
        let filename = entry.file_name();
        let filename = filename.to_string_lossy();
        let Some(stripped) = filename.strip_prefix("cpu") else {
            continue;
        };

        let Ok(index) = stripped.parse::<usize>() else {
            continue;
        };

        let stat = parse_cpu_freq_cpu_info(entry.path().join("cpufreq"))?;
        stats.push((index, stat))
    }

    Ok(stats)
}

fn parse_cpu_freq_cpu_info(root: PathBuf) -> Result<Stat, Error> {
    let current_frequency = read_into(root.join("cpuinfo_cur_freq")).ok();
    // AMD CPU do have theos two files
    let maximum_frequency = read_into(root.join("cpuinfo_max_freq")).ok();
    let minimum_frequency = read_into(root.join("cpuinfo_min_freq")).ok();

    let transition_latency = read_into(root.join("cpuinfo_transition_latency")).ok();
    let scaling_current_frequency = read_into(root.join("scaling_cur_freq")).ok();
    let scaling_maximum_frequency = read_into(root.join("scaling_max_freq")).ok();
    let scaling_minimum_frequency = read_into(root.join("scaling_min_freq")).ok();

    let available_governors =
        read_sys_file(root.join("scaling_available_governors")).unwrap_or_default();
    let driver = read_sys_file(root.join("scaling_driver")).unwrap_or_default();
    let governor = read_sys_file(root.join("scaling_governor")).unwrap_or_default();
    let related_cpus = read_sys_file(root.join("related_cpus")).unwrap_or_default();
    let set_speed = read_sys_file(root.join("scaling_setspeed")).unwrap_or_default();

    Ok(Stat {
        current_frequency,
        minimum_frequency,
        maximum_frequency,
        transition_latency,
        scaling_current_frequency,
        scaling_minimum_frequency,
        scaling_maximum_frequency,
        available_governors,
        driver,
        governor,
        related_cpus,
        set_speed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn smoke() {
        let paths = Paths::test();
        let metrics = collect(paths).await.unwrap();
        assert_ne!(metrics.len(), 0);
    }

    #[test]
    fn cpu_freq_stat() {
        let sys_path = Path::new("tests/node/fixtures/sys");
        let stats = get_cpu_freq_stat(sys_path).unwrap();

        assert_eq!(
            stats[0],
            (
                0,
                Stat {
                    current_frequency: None,
                    minimum_frequency: Some(800000),
                    maximum_frequency: Some(2400000),
                    transition_latency: Some(0),
                    scaling_current_frequency: Some(1219917),
                    scaling_minimum_frequency: Some(800000),
                    scaling_maximum_frequency: Some(2400000),
                    available_governors: "performance powersave".into(),
                    driver: "intel_pstate".into(),
                    governor: "powersave".into(),
                    related_cpus: "0".into(),
                    set_speed: "<unsupported>".into(),
                }
            )
        );

        assert_eq!(
            stats[1],
            (
                1,
                Stat {
                    current_frequency: Some(1200195),
                    minimum_frequency: Some(1200000),
                    maximum_frequency: Some(3300000),
                    transition_latency: Some(4294967295),
                    scaling_current_frequency: None,
                    scaling_minimum_frequency: Some(1200000),
                    scaling_maximum_frequency: Some(3300000),
                    available_governors: "performance powersave".into(),
                    driver: "intel_pstate".into(),
                    governor: "powersave".into(),
                    related_cpus: "1".into(),
                    set_speed: "<unsupported>".into(),
                }
            )
        )
    }
}
