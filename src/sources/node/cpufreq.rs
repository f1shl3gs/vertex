use crate::{
    tags,
    gauge_metric,
    sources::node::{
        read_into,
        errors::{
            Error, ErrorContext
        },
    },
    event::{Metric, MetricValue},
};

use std::{
    io,
};

pub async fn gather(sys_path: &str) -> Result<Vec<Metric>, Error> {
    let stats = get_cpu_freq_stat(sys_path).await?;
    let mut metrics = Vec::with_capacity(stats.len() * 6);

    for stat in stats {
        let cpu = &stat.name;

        if let Some(v) = stat.current_frequency {
            metrics.push(gauge_metric!(
                "node_cpu_frequency_hertz",
                "Current cpu thread frequency in hertz.",
                v as f64 * 1000.0,
                "cpu" => cpu
            ));
        }

        if let Some(v) = stat.minimum_frequency {
            metrics.push(gauge_metric!(
                "node_cpu_frequency_min_hertz",
                "Minimum cpu thread frequency in hertz.",
                v as f64 * 1000.0,
                "cpu" => cpu
            ));
        }

        if let Some(v) = stat.maximum_frequency {
            metrics.push(gauge_metric!(
                "node_cpu_frequency_max_hertz",
                "Maximum cpu thread frequency in hertz.",
                v as f64 * 1000.0,
                "cpu" => cpu
            ))
        }

        if let Some(v) = stat.scaling_current_frequency {
            metrics.push(gauge_metric!(
                "node_cpu_scaling_frequency_hertz",
                "Current scaled CPU thread frequency in hertz.",
                v as f64 * 1000.0,
                "cpu" => cpu
            ))
        }

        if let Some(v) = stat.scaling_minimum_frequency {
            metrics.push(gauge_metric!(
                "node_cpu_scaling_frequency_min_hertz",
                "Minimum scaled CPU thread frequency in hertz.",
                v as f64 * 1000.0,
                "cpu" => cpu
            ));
        }

        if let Some(v) = stat.scaling_maximum_frequency {
            metrics.push(gauge_metric!(
                "node_cpu_scaling_frequency_max_hertz",
                "Maximum scaled CPU thread frequency in hertz.",
                v as f64 * 1000.0,
                "cpu" => cpu
            ));
        }
    }

    Ok(metrics)
}

#[derive(Default, Debug, PartialEq)]
struct Stat {
    name: String,

    current_frequency: Option<u64>,
    minimum_frequency: Option<u64>,
    maximum_frequency: Option<u64>,
    transition_latency: Option<u64>,
    scaling_current_frequency: Option<u64>,
    scaling_minimum_frequency: Option<u64>,
    scaling_maximum_frequency: Option<u64>,
}

async fn get_cpu_freq_stat(sys_path: &str) -> Result<Vec<Stat>, Error> {
    let cpus = glob::glob(&format!("{}/devices/system/cpu/cpu[0-9]*", sys_path))
        .map_err(|err| {
            Error::from(io::Error::new(io::ErrorKind::InvalidData, err))
        })
        .context("no cpu files were found")?;

    let mut stats = Vec::new();

    for entry in cpus {
        match entry {
            Ok(path) => {
                let cp = path.to_str().unwrap();
                let mut stat = parse_cpu_freq_cpu_info(cp).await?;

                // this looks terrible
                stat.name = path.file_name().unwrap().to_str().unwrap().replace("cpu", "");

                stats.push(stat)
            }

            Err(err) => {
                println!("err {}", err)
            }
        }
    }

    Ok(stats)
}

async fn parse_cpu_freq_cpu_info(cpu_path: &str) -> Result<Stat, Error> {
    let mut stat = Stat::default();

    let path = format!("{}/cpufreq/cpuinfo_cur_freq", cpu_path);
    stat.current_frequency = read_into(path).await.ok();

    let path = format!("{}/cpufreq/cpuinfo_max_freq", cpu_path);
    stat.maximum_frequency = read_into(path).await.ok();

    let path = format!("{}/cpufreq/cpuinfo_min_freq", cpu_path);
    stat.minimum_frequency = read_into(path).await.ok();

    let path = format!("{}/cpufreq/cpuinfo_transition_latency", cpu_path);
    stat.transition_latency = read_into(path).await.ok();

    let path = format!("{}/cpufreq/scaling_cur_freq", cpu_path);
    stat.scaling_current_frequency = read_into(path).await.ok();

    let path = format!("{}/cpufreq/scaling_max_freq", cpu_path);
    stat.scaling_maximum_frequency = read_into(path).await.ok();

    let path = format!("{}/cpufreq/scaling_min_freq", cpu_path);
    stat.scaling_minimum_frequency = read_into(path).await.ok();

    Ok(stat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_cpu_freq_cpu_info() {
        let cpu_path = "testdata/sys/devices/system/cpu/cpu0";
        let v = parse_cpu_freq_cpu_info(cpu_path).await.unwrap();
        println!("{:?}", v);
    }

    #[tokio::test]
    async fn test_get_cpu_freq_stat() {
        let sys_path = "testdata/sys";
        let stats = get_cpu_freq_stat(sys_path).await.unwrap();

        assert_eq!(stats[0], Stat {
            name: "0".to_string(),
            current_frequency: None,
            minimum_frequency: Some(800000),
            maximum_frequency: Some(2400000),
            transition_latency: Some(0),
            scaling_current_frequency: Some(1219917),
            scaling_minimum_frequency: Some(800000),
            scaling_maximum_frequency: Some(2400000)
        });

        assert_eq!(stats[1], Stat {
            name: "1".to_string(),
            current_frequency: Some(1200195),
            minimum_frequency: Some(1200000),
            maximum_frequency: Some(3300000),
            transition_latency: Some(4294967295),
            scaling_current_frequency: None,
            scaling_minimum_frequency: Some(1200000),
            scaling_maximum_frequency: Some(3300000)
        })
    }
}