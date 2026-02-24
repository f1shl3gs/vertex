use std::path::PathBuf;

use event::{Metric, tags};

use super::{read_into, read_string};

pub async fn gather(sys_path: PathBuf) -> Result<Vec<Metric>, crate::Error> {
    let dirs = sys_path.join("class/watchdog").read_dir()?;

    let mut metrics = Vec::new();
    for entry in dirs.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let stat = parse_watchdog(entry.path())?;

        if let Some(bootstatus) = stat.bootstatus {
            metrics.push(Metric::gauge_with_tags(
                "node_watchdog_bootstatus",
                "Value of /sys/class/watchdog/<watchdog>/bootstatus",
                bootstatus,
                tags!(
                    "name" => &name,
                ),
            ));
        }

        if let Some(fw_version) = stat.fw_version {
            metrics.push(Metric::gauge_with_tags(
                "node_watchdog_fw_version",
                "Value of /sys/class/watchdog/<watchdog>/fw_version",
                fw_version,
                tags!( "name" => &name ),
            ));
        }

        if let Some(nowayout) = stat.nowayout {
            metrics.push(Metric::gauge_with_tags(
                "node_watchdog_nowayout",
                "Value of /sys/class/watchdog/<watchdog>/nowayout",
                nowayout,
                tags!( "name" => &name ),
            ));
        }

        if let Some(timeleft) = stat.timeleft {
            metrics.push(Metric::gauge_with_tags(
                "node_watchdog_timeleft_seconds",
                "Value of /sys/class/watchdog/<watchdog>/timeleft",
                timeleft,
                tags!( "name" => &name ),
            ));
        }

        if let Some(timeout) = stat.timeout {
            metrics.push(Metric::gauge_with_tags(
                "node_watchdog_timeout_seconds",
                "Value of /sys/class/watchdog/<watchdog>/timeout",
                timeout,
                tags!( "name" => &name ),
            ));
        }

        if let Some(pretimeout) = stat.pretimeout {
            metrics.push(Metric::gauge_with_tags(
                "node_watchdog_pretimeout_seconds",
                "Value of /sys/class/watchdog/<watchdog>/pretimeout",
                pretimeout,
                tags!( "name" => &name ),
            ));
        }

        if let Some(access_cs0) = stat.access_cs0 {
            metrics.push(Metric::gauge_with_tags(
                "node_watchdog_access_cs0",
                "Value of /sys/class/watchdog/<watchdog>/access_cs0",
                access_cs0,
                tags!( "name" => &name ),
            ));
        }

        metrics.push(Metric::gauge_with_tags(
            "node_watchdog_info",
            "Info of /sys/class/watchdog/<watchdog>",
            1,
            tags!(
                "name" => name,
                "options" => stat.options.unwrap_or_default(),
                "identity" => stat.identity.unwrap_or_default(),
                "state" => stat.state.unwrap_or_default(),
                "status" => stat.status.unwrap_or_default(),
                "pretimeout_governor" => stat.pretimeout_governor.unwrap_or_default(),
            ),
        ))
    }

    Ok(metrics)
}

#[derive(Debug, Default)]
struct Stat {
    bootstatus: Option<i64>,             // /sys/class/watchdog/<Name>/bootstatus
    options: Option<String>,             // /sys/class/watchdog/<Name>/options
    fw_version: Option<i64>,             // /sys/class/watchdog/<Name>/fw_version
    identity: Option<String>,            // /sys/class/watchdog/<Name>/identity
    nowayout: Option<i64>,               // /sys/class/watchdog/<Name>/nowayout
    state: Option<String>,               // /sys/class/watchdog/<Name>/state
    status: Option<String>,              // /sys/class/watchdog/<Name>/status
    timeleft: Option<i64>,               // /sys/class/watchdog/<Name>/timeleft
    timeout: Option<i64>,                // /sys/class/watchdog/<Name>/timeout
    pretimeout: Option<i64>,             // /sys/class/watchdog/<Name>/pretimeout
    pretimeout_governor: Option<String>, // /sys/class/watchdog/<Name>/pretimeout_governor
    access_cs0: Option<i64>,             // /sys/class/watchdog/<Name>/access_cs0
}

fn parse_watchdog(path: PathBuf) -> Result<Stat, crate::Error> {
    let dirs = path.read_dir()?;

    let mut stat = Stat::default();
    for entry in dirs {
        let Ok(entry) = entry else {
            continue;
        };

        match entry.file_name().to_string_lossy().as_ref() {
            "bootstatus" => stat.bootstatus = read_into(entry.path()).ok(),
            "options" => stat.options = read_string(entry.path()).ok(),
            "fw_version" => stat.fw_version = read_into(entry.path()).ok(),
            "identity" => stat.identity = read_string(entry.path()).ok(),
            "nowayout" => stat.nowayout = read_into(entry.path()).ok(),
            "state" => stat.state = read_string(entry.path()).ok(),
            "status" => stat.status = read_string(entry.path()).ok(),
            "timeleft" => stat.timeleft = read_into(entry.path()).ok(),
            "timeout" => stat.timeout = read_into(entry.path()).ok(),
            "pretimeout" => stat.pretimeout = read_into(entry.path()).ok(),
            "pretimeout_governor" => stat.pretimeout_governor = read_string(entry.path()).ok(),
            "access_cs0" => stat.access_cs0 = read_into(entry.path()).ok(),
            _ => continue,
        }
    }

    Ok(stat)
}
