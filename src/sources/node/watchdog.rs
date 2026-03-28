use std::path::PathBuf;

use event::{Metric, tags};

use super::{Paths, read_sys_file};

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, crate::Error> {
    let root = paths.sys().join("class/watchdog");
    let mut metrics = Vec::new();

    for entry in root.read_dir()?.flatten() {
        let filename = entry.file_name();
        let name = filename.to_string_lossy();
        let stat = parse_watchdog(entry.path())?;

        if let Some(value) = stat.bootstatus {
            metrics.push(Metric::gauge_with_tags(
                "node_watchdog_bootstatus",
                "Value of /sys/class/watchdog/<watchdog>/bootstatus",
                value,
                tags!(
                    "name" => name.as_ref(),
                ),
            ));
        }

        if let Some(value) = stat.fw_version {
            metrics.push(Metric::gauge_with_tags(
                "node_watchdog_fw_version",
                "Value of /sys/class/watchdog/<watchdog>/fw_version",
                value,
                tags!("name" => name.as_ref()),
            ));
        }

        if let Some(value) = stat.nowayout {
            metrics.push(Metric::gauge_with_tags(
                "node_watchdog_nowayout",
                "Value of /sys/class/watchdog/<watchdog>/nowayout",
                value,
                tags!("name" => name.as_ref()),
            ));
        }

        if let Some(value) = stat.timeleft {
            metrics.push(Metric::gauge_with_tags(
                "node_watchdog_timeleft_seconds",
                "Value of /sys/class/watchdog/<watchdog>/timeleft",
                value,
                tags!("name" => name.as_ref()),
            ));
        }

        if let Some(value) = stat.timeout {
            metrics.push(Metric::gauge_with_tags(
                "node_watchdog_timeout_seconds",
                "Value of /sys/class/watchdog/<watchdog>/timeout",
                value,
                tags!("name" => name.as_ref()),
            ));
        }

        if let Some(value) = stat.pretimeout {
            metrics.push(Metric::gauge_with_tags(
                "node_watchdog_pretimeout_seconds",
                "Value of /sys/class/watchdog/<watchdog>/pretimeout",
                value,
                tags!("name" => name.as_ref()),
            ));
        }

        if let Some(value) = stat.access_cs0 {
            metrics.push(Metric::gauge_with_tags(
                "node_watchdog_access_cs0",
                "Value of /sys/class/watchdog/<watchdog>/access_cs0",
                value,
                tags!("name" => name.as_ref()),
            ));
        }

        metrics.push(Metric::gauge_with_tags(
            "node_watchdog_info",
            "Info of /sys/class/watchdog/<watchdog>",
            1,
            tags!(
                "name" => name.as_ref(),
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

#[cfg_attr(test, derive(PartialEq))]
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

fn parse_watchdog(root: PathBuf) -> Result<Stat, crate::Error> {
    let mut stat = Stat::default();

    let mut path = root.join("bootstatus");
    for (filename, dst) in [
        ("bootstatus", &mut stat.bootstatus),
        ("fw_version", &mut stat.fw_version),
        ("nowayout", &mut stat.nowayout),
        ("timeleft", &mut stat.timeleft),
        ("timeout", &mut stat.timeout),
        ("pretimeout", &mut stat.pretimeout),
        ("access_cs0", &mut stat.access_cs0),
    ] {
        path.set_file_name(filename);
        let Ok(content) = read_sys_file(&path) else {
            continue;
        };
        *dst = content.parse::<i64>().ok();
    }

    for (filename, dst) in [
        ("options", &mut stat.options),
        ("identity", &mut stat.identity),
        ("state", &mut stat.state),
        ("status", &mut stat.status),
        ("pretimeout_governor", &mut stat.pretimeout_governor),
    ] {
        path.set_file_name(filename);
        if let Ok(content) = read_sys_file(&path) {
            *dst = Some(content);
        }
    }

    Ok(stat)
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
    fn parse() {
        let root = PathBuf::from("tests/node/fixtures/sys/class/watchdog/watchdog0");
        let got = parse_watchdog(root).unwrap();
        let want = Stat {
            bootstatus: Some(1),
            options: Some("0x8380".to_string()),
            fw_version: Some(2),
            identity: Some("Software Watchdog".to_string()),
            nowayout: Some(0),
            state: Some("active".to_string()),
            status: Some("0x8000".to_string()),
            timeleft: Some(300),
            timeout: Some(60),
            pretimeout: Some(120),
            pretimeout_governor: Some("noop".to_string()),
            access_cs0: Some(0),
        };
        assert_eq!(got, want);

        let root = PathBuf::from("tests/node/fixtures/sys/class/watchdog/watchdog1");
        let got = parse_watchdog(root).unwrap();
        let want = Stat::default();
        assert_eq!(got, want);
    }
}
