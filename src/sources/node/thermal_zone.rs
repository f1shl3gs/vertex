//! Exposes thermal zone & cooling device statistics from /sys/class/thermal

use std::path::{Path, PathBuf};

use event::{Metric, tags};

use super::{Error, read_into, read_string};

/// ThermalStats contains info from files in /sys/class/thermal_zone<zone>
/// for a single <zone>
///
/// https://www.kernel.org/doc/Documentation/thermal/sysfs-api.txt
#[derive(Debug, PartialEq)]
struct ThermalZoneStats {
    // The name of the zone from the directory structure
    name: String,

    // The type of thermal zone
    typ: String,

    // Temperature in millidegree Celsius
    temp: i64,

    // One of the various thermal governors used for a particular zone
    policy: String,

    // Optional: One of the predefined values in [enabled, disabled]
    mode: Option<bool>,

    // Optional: millidegrees Celsius. (0 for disabled, > 1000 for enabled+value)
    passive: Option<u64>,
}

async fn thermal_zone_stats(root: &Path) -> Result<Vec<ThermalZoneStats>, Error> {
    let pattern = format!(
        "{}/class/thermal/thermal_zone[0-9]*",
        root.to_string_lossy()
    );
    let paths = glob::glob(&pattern)?;

    let mut stats = vec![];
    for path in paths.flatten() {
        stats.push(parse_thermal_zone(&path)?);
    }

    Ok(stats)
}

fn parse_thermal_zone(root: &Path) -> Result<ThermalZoneStats, Error> {
    let name = root
        .file_name()
        .unwrap()
        .to_string_lossy()
        .strip_prefix("thermal_zone")
        .unwrap()
        .to_string();

    // required attributes
    let typ = read_string(root.join("type"))?;
    let policy = read_string(root.join("policy"))?;
    let temp = read_into(root.join("temp"))?;

    // optional attributes
    let mode = match read_string(root.join("mode")) {
        Ok(content) => match content.as_str() {
            "enabled" => Some(true),
            "disabled" => Some(false),
            _ => None,
        },
        Err(_) => None,
    };

    let path = root.join("passive");
    let passive = match read_into(path) {
        Ok(v) => Some(v),
        Err(err) => {
            if err.is_not_found() {
                None
            } else {
                return Err(err);
            }
        }
    };

    Ok(ThermalZoneStats {
        name,
        typ,
        temp,
        policy,
        mode,
        passive,
    })
}

/// CoolingDeviceStats contains info from files in /sys/class/thermal/cooling_device[0-9]*
/// for a single device, https://www.kernel.org/doc/Documentation/thermal/sysfs-api.txt
#[derive(Debug, PartialEq)]
struct CoolingDeviceStats {
    // The name of the cooling device
    name: String,
    // Type of the cooling device(processor/fan/...)
    typ: String,
    // Maximum cooling state of the cooling device
    max_state: i64,
    // Current cooling state of the cooling device
    cur_state: i64,
}

async fn cooling_device_stats(root: PathBuf) -> Result<Vec<CoolingDeviceStats>, Error> {
    let pattern = format!(
        "{}/class/thermal/cooling_device[0-9]*",
        root.to_string_lossy()
    );
    let paths = glob::glob(&pattern)?;

    let mut stats = vec![];
    for path in paths.flatten() {
        let stat = parse_cooling_device_stats(path).await?;
        stats.push(stat);
    }

    Ok(stats)
}

async fn parse_cooling_device_stats(root: PathBuf) -> Result<CoolingDeviceStats, Error> {
    let name = root
        .file_name()
        .unwrap()
        .to_string_lossy()
        .strip_prefix("cooling_device")
        .unwrap()
        .to_string();

    let typ = read_string(root.join("type"))?;
    let max_state = read_into(root.join("max_state"))?;
    // cur_state can be -1, eg intel powerclamp
    // https://www.kernel.org/doc/Documentation/thermal/intel_powerclamp.txt
    let cur_state = read_into(root.join("cur_state"))?;

    Ok(CoolingDeviceStats {
        name,
        typ,
        max_state,
        cur_state,
    })
}

pub async fn gather(sys_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let stats = thermal_zone_stats(&sys_path).await?;

    let mut metrics = Vec::with_capacity(stats.len());
    for stat in stats {
        metrics.push(Metric::gauge_with_tags(
            "node_thermal_zone_temp",
            "Zone temperature in Celsius",
            stat.temp as f64 / 1000.0,
            tags!(
                "zone" => stat.name,
                "type" => stat.typ,
            ),
        ));
    }

    let stats = cooling_device_stats(sys_path).await?;
    metrics.reserve(stats.len() * 2);
    for stat in stats {
        metrics.extend([
            Metric::gauge_with_tags(
                "node_cooling_device_cur_state",
                "Current throttle state of the cooling device",
                stat.cur_state,
                tags!(
                    "name" => &stat.name,
                    "type" => &stat.typ
                ),
            ),
            Metric::gauge_with_tags(
                "node_cooling_device_max_state",
                "Maximum throttle state of the cooling device",
                stat.max_state,
                tags!(
                    "name" => stat.name,
                    "type" => stat.typ
                ),
            ),
        ])
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_thermal_zone_stats() {
        let root = PathBuf::from("tests/node/sys");
        let stats = thermal_zone_stats(&root).await.unwrap();

        assert_eq!(
            stats,
            vec![
                ThermalZoneStats {
                    name: "0".to_string(),
                    typ: "bcm2835_thermal".to_string(),
                    policy: "step_wise".to_string(),
                    temp: 49925,
                    mode: None,
                    passive: None,
                },
                ThermalZoneStats {
                    name: "1".to_string(),
                    typ: "acpitz".to_string(),
                    policy: "step_wise".to_string(),
                    temp: -44000,
                    mode: Some(true),
                    passive: Some(0),
                },
            ]
        );
    }

    #[tokio::test]
    async fn test_cooling_device_stats() {
        let root = "tests/node/sys".into();
        let stats = cooling_device_stats(root).await.unwrap();
        assert_eq!(
            stats,
            vec![
                CoolingDeviceStats {
                    name: "0".to_string(),
                    typ: "Processor".to_string(),
                    max_state: 50,
                    cur_state: 0,
                },
                CoolingDeviceStats {
                    name: "1".to_string(),
                    typ: "intel_powerclamp".to_string(),
                    max_state: 27,
                    cur_state: -1,
                },
            ]
        )
    }
}
