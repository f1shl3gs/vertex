use std::path::PathBuf;

use event::{Metric, tags};

use super::{Error, read_into, read_string};

pub async fn gather(sys_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let stats = get_cpu_freq_stat(sys_path).await?;
    let mut metrics = Vec::with_capacity(stats.len() * 6);

    for stat in stats {
        if let Some(v) = stat.current_frequency {
            metrics.push(Metric::gauge_with_tags(
                "node_cpu_frequency_hertz",
                "Current cpu thread frequency in hertz.",
                v * 1000,
                tags!(
                    "cpu" => stat.index,
                ),
            ));
        }

        if let Some(v) = stat.minimum_frequency {
            metrics.push(Metric::gauge_with_tags(
                "node_cpu_frequency_min_hertz",
                "Minimum cpu thread frequency in hertz.",
                v * 1000,
                tags!(
                    "cpu" => stat.index,
                ),
            ));
        }

        if let Some(v) = stat.maximum_frequency {
            metrics.push(Metric::gauge_with_tags(
                "node_cpu_frequency_max_hertz",
                "Maximum CPU thread frequency in hertz.",
                v * 1000,
                tags!(
                    "cpu" => stat.index,
                ),
            ))
        }

        if let Some(v) = stat.scaling_current_frequency {
            metrics.push(Metric::gauge_with_tags(
                "node_cpu_scaling_frequency_hertz",
                "Current scaled CPU thread frequency in hertz.",
                v * 1000,
                tags!(
                    "cpu" => stat.index,
                ),
            ))
        }

        if let Some(v) = stat.scaling_minimum_frequency {
            metrics.push(Metric::gauge_with_tags(
                "node_cpu_scaling_frequency_min_hertz",
                "Minimum scaled CPU thread frequency in hertz.",
                v as f64 * 1000.0,
                tags!(
                    "cpu" => stat.index,
                ),
            ));
        }

        if let Some(v) = stat.scaling_maximum_frequency {
            metrics.push(Metric::gauge_with_tags(
                "node_cpu_scaling_frequency_max_hertz",
                "Maximum scaled CPU thread frequency in hertz.",
                v as f64 * 1000.0,
                tags!(
                    "cpu" => stat.index,
                ),
            ));
        }

        if !stat.governor.is_empty() {
            for governor in stat.available_governors.split_ascii_whitespace() {
                metrics.push(Metric::gauge_with_tags(
                    "node_cpu_scaling_governor",
                    "Current enabled CPU frequency governor",
                    governor == stat.governor,
                    tags!(
                        "cpu" => stat.index,
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
    index: usize,

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

async fn get_cpu_freq_stat(sys_path: PathBuf) -> Result<Vec<Stat>, Error> {
    let cpus = glob::glob(&format!(
        "{}/devices/system/cpu/cpu[0-9]*",
        sys_path.to_string_lossy()
    ))?;

    let mut stats = Vec::new();
    for path in cpus.flatten() {
        let stat = parse_cpu_freq_cpu_info(path).await?;
        stats.push(stat)
    }

    Ok(stats)
}

async fn parse_cpu_freq_cpu_info(root: PathBuf) -> Result<Stat, Error> {
    let index = root
        .file_name()
        .unwrap()
        .to_string_lossy()
        .strip_prefix("cpu")
        .map(|x| x.parse())
        .transpose()?
        .unwrap_or(0);

    let mut stat = Stat {
        index,
        ..Default::default()
    };

    stat.current_frequency = read_into(root.join("cpufreq/cpuinfo_cur_freq")).ok();
    // AMD CPU do have theos two files
    stat.maximum_frequency = read_into(root.join("cpufreq/cpuinfo_max_freq")).ok();
    stat.minimum_frequency = read_into(root.join("cpufreq/cpuinfo_min_freq")).ok();

    stat.transition_latency = read_into(root.join("cpufreq/cpuinfo_transition_latency")).ok();
    stat.scaling_current_frequency = read_into(root.join("cpufreq/scaling_cur_freq")).ok();
    stat.scaling_maximum_frequency = read_into(root.join("cpufreq/scaling_max_freq")).ok();
    stat.scaling_minimum_frequency = read_into(root.join("cpufreq/scaling_min_freq")).ok();

    stat.available_governors =
        read_string(root.join("cpufreq/scaling_available_governors")).unwrap_or_default();
    stat.driver = read_string(root.join("cpufreq/scaling_driver")).unwrap_or_default();
    stat.governor = read_string(root.join("cpufreq/scaling_governor")).unwrap_or_default();
    stat.related_cpus = read_string(root.join("cpufreq/related_cpus")).unwrap_or_default();
    stat.set_speed = read_string(root.join("cpufreq/scaling_setspeed")).unwrap_or_default();

    Ok(stat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_cpu_freq_stat() {
        let sys_path = "tests/node/sys";
        let stats = get_cpu_freq_stat(sys_path.into()).await.unwrap();

        assert_eq!(
            stats[0],
            Stat {
                index: 0,
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
        );

        assert_eq!(
            stats[1],
            Stat {
                index: 1,
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
    }
}
