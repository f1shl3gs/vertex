use std::ffi::CStr;
use std::path::{Path, PathBuf};

use event::{Metric, tags};

use super::{Error, read_string};

/// Exposes the current system time
pub async fn gather(sys_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let local_now = chrono::Local::now();
    let offset = local_now.offset().local_minus_utc() as f64;
    let now_sec = local_now.timestamp_nanos_opt().unwrap() as f64 / 1e9;

    // TODO: we should and possibly get TZ with chrono, cause
    // the offset is already got
    let tz = libc_timezone();

    let mut metrics = vec![
        Metric::gauge(
            "node_time_seconds",
            "System time in seconds since epoch (1970)",
            now_sec,
        ),
        Metric::gauge_with_tags(
            "node_time_zone_offset_seconds",
            "System time zone offset in seconds",
            offset,
            tags!(
                "time_zone" => tz,
            ),
        ),
    ];

    let sources = parse_clock_sources(&sys_path)?;

    for source in sources {
        for available in source.available {
            metrics.push(Metric::gauge_with_tags(
                "node_time_clocksource_available_info",
                "Available clocksources read from '/sys/devices/system/clocksource'.",
                1,
                tags!(
                    "device" => &source.name,
                    "clocksource" => available,
                ),
            ));
        }

        metrics.push(Metric::gauge_with_tags(
            "node_time_clocksource_current_info",
            "Current clocksource read from '/sys/devices/system/clocksource'.",
            1,
            tags!(
                "device" => source.name,
                "clocksource" => source.current,
            ),
        ))
    }

    Ok(metrics)
}

fn libc_timezone() -> String {
    unsafe {
        // https://github.com/rust-lang/libc/issues/1848
        #[cfg_attr(target_env = "musl", allow(deprecated))]
        let sec = 0 as libc::time_t;
        let mut out = std::mem::zeroed();

        if libc::localtime_r(&sec, &mut out).is_null() {
            panic!(
                "syscall localtime_r failed, {}",
                std::io::Error::last_os_error()
            );
        }

        let tz: &CStr = CStr::from_ptr(out.tm_zone);

        tz.to_str().unwrap().to_string()
    }
}

#[derive(Debug)]
struct ClockSource {
    name: String,
    available: Vec<String>,
    current: String,
}

fn parse_clock_sources(root: &Path) -> Result<Vec<ClockSource>, Error> {
    let dirs = std::fs::read_dir(root.join("devices/system/clocksource"))?;

    let mut sources = vec![];
    for entry in dirs.flatten() {
        let name = match entry
            .file_name()
            .to_string_lossy()
            .strip_prefix("clocksource")
        {
            Some(name) => name.to_string(),
            None => continue,
        };

        let path = entry.path();
        let data = read_string(path.join("available_clocksource"))?;
        let available = data
            .split_ascii_whitespace()
            .map(String::from)
            .collect::<Vec<_>>();

        let current = read_string(path.join("current_clocksource"))?;

        sources.push(ClockSource {
            name,
            available,
            current,
        })
    }

    Ok(sources)
}

#[cfg(test)]
mod tests {
    use std::ffi::CStr;
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn libc_localtime_r() {
        unsafe {
            let sec = 0i64;
            let sec = sec as libc::time_t;
            let mut out = std::mem::zeroed();

            if libc::localtime_r(&sec, &mut out).is_null() {
                panic!("xx")
            }

            let tz: &CStr = CStr::from_ptr(out.tm_zone);
            tz.to_str().unwrap();
        }
    }

    #[test]
    fn clocksource() {
        let path = PathBuf::from("tests/node/sys");
        let sources = parse_clock_sources(&path).unwrap();
        assert_eq!(sources.len(), 1);

        assert_eq!(sources[0].name, "0");
        assert_eq!(sources[0].current, "tsc");
        assert_eq!(
            sources[0].available,
            vec!["tsc".to_string(), "hpet".to_string(), "acpi_pm".to_string()]
        );
    }
}
