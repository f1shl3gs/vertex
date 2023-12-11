use std::collections::BTreeMap;
use std::fs::{canonicalize, read_link};
use std::path::{Path, PathBuf};

use event::{tags, Metric};
use futures::stream::{FuturesUnordered, StreamExt};
use once_cell::sync::Lazy;
use regex::Regex;

use super::{read_to_string, Error};

pub async fn gather(sys_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let mut tasks = FuturesUnordered::new();
    let mut dirs = std::fs::read_dir(sys_path.join("class/hwmon"))?;

    while let Some(Ok(entry)) = dirs.next() {
        tasks.push(tokio::spawn(async move {
            let meta = entry.metadata()?;
            let meta = match meta.file_type().is_symlink() {
                true => canonicalize(entry.path())?.metadata()?,
                false => meta,
            };

            if !meta.is_dir() {
                return Ok(vec![]);
            }

            hwmon_metrics(entry.path()).await
        }));
    }

    let mut metrics = vec![];
    while let Some(result) = tasks.next().await {
        match result {
            Ok(result) => match result {
                Ok(partial) => metrics.extend(partial),
                Err(err) => {
                    warn!(message = "gather metrics failed", ?err);
                }
            },
            Err(err) => {
                warn!(message = "spawn task failed", ?err);
            }
        }
    }

    Ok(metrics)
}

async fn hwmon_metrics(dir: PathBuf) -> Result<Vec<Metric>, Error> {
    let chip = &hwmon_name(&dir).await?;
    let data = {
        let result = collect_sensor_data(&dir).await;
        match result {
            Ok(r) => r,
            Err(_err) => collect_sensor_data(dir.join("device")).await?,
        }
    };

    let mut metrics = Vec::new();
    if let Ok(chip_name) = human_readable_chip_name(dir).await {
        metrics.push(Metric::gauge_with_tags(
            "node_hwmon_chip_names",
            "Annotation metric for human-readable chip names",
            1f64,
            tags!(
                "chip" => chip,
                "chip_name" => chip_name,
            ),
        ));
    }

    for (sensor, props) in &data {
        let sensor_type = match explode_sensor_filename(sensor) {
            Ok((st, _, _)) => st,
            _ => {
                warn!(
                    message = "unknown sensor type",
                    %sensor
                );
                continue;
            }
        };

        if let Some(label) = props.get("label") {
            if !label.is_empty() {
                metrics.push(Metric::gauge_with_tags(
                    "node_hwmon_sensor_label",
                    "Label for given chip and sensor",
                    1f64,
                    tags!(
                        "chip" => chip,
                        "label" => label,
                        "sensor" => sensor.clone(),
                    ),
                ));
            }
        }

        if sensor_type == "beep_enable" {
            let mut v = 0f64;
            if let Some(value) = props.get("") {
                if value == "1" {
                    v = 1.0;
                }
            }

            metrics.push(Metric::gauge_with_tags(
                "node_hwmon_beep_enabled",
                "Hardware beep enabled",
                v,
                tags!(
                    "chip" => chip,
                    "sensor" => sensor,
                ),
            ));

            continue;
        }

        if sensor_type == "vrm" {
            let v = match props.get("") {
                Some(value) => {
                    let pr = value.parse::<f64>();
                    if pr.is_err() {
                        continue;
                    }

                    pr.unwrap()
                }

                None => {
                    continue;
                }
            };

            metrics.push(Metric::gauge_with_tags(
                "node_hwmon_voltage_regulator_version",
                "Hardware voltage regulator",
                v,
                tags!(
                    "chip" => chip,
                    "sensor" => sensor,
                ),
            ));

            continue;
        }

        if sensor_type == "update_interval" {
            let pv = match props.get("").map(|v| v.parse::<f64>()) {
                Some(Ok(v)) => v,
                _ => continue,
            };

            metrics.push(Metric::gauge_with_tags(
                "node_hwmon_update_interval_seconds",
                "Hardware monitor update interval",
                pv * 0.001,
                tags!(
                    "chip" => chip,
                    "sensor" => sensor,
                ),
            ));

            continue;
        }

        let prefix = format!("node_hwmon_{}", sensor_type);
        for (element, value) in props {
            if element == "label" {
                continue;
            }

            let mut name = prefix.clone();
            if element == "input" {
                // input is actually the value
                if let Some(_v) = props.get("") {
                    name += "_input";
                }
            } else if !element.is_empty() {
                name = format!("{}_{}", name, sanitized(element));
            }

            let pv = match value.parse::<f64>() {
                Ok(v) => v,
                _ => continue,
            };

            // special key, fault, alarm & beep should be handed out without units
            if element == "fault" || element == "alarm" {
                metrics.push(Metric::gauge_with_tags(
                    name,
                    format!("Hardware sensor {} status ({})", sensor, sensor_type),
                    pv,
                    tags!(
                        "chip" => chip,
                        "sensor" => sensor,
                    ),
                ));
                continue;
            }

            if element == "beep" {
                metrics.push(Metric::gauge_with_tags(
                    name + "_enabled",
                    "Hardware monitor sensor has beeping enabled",
                    pv,
                    tags!(
                        "chip" => chip,
                        "sensor" => sensor,
                    ),
                ));
                continue;
            }

            // everything else should get a unit
            if sensor_type == "in" || sensor_type == "cpu" {
                metrics.push(Metric::gauge_with_tags(
                    name + "_volts",
                    format!("Hardware monitor for voltage ({})", element),
                    pv * 0.001,
                    tags!(
                        "chip" => chip,
                        "sensor" => sensor,
                    ),
                ));
                continue;
            }

            if sensor_type == "temp" && element != "type" {
                let mut element = element.as_str();
                if element.is_empty() {
                    element = "input";
                }

                metrics.push(Metric::gauge_with_tags(
                    name + "_celsius",
                    format!("Hardware monitor for temperature ({})", element),
                    pv * 0.001,
                    tags!(
                        "chip" => chip,
                        "sensor" => sensor,
                    ),
                ));
                continue;
            }

            if sensor_type == "curr" {
                metrics.push(Metric::gauge_with_tags(
                    name + "_amps",
                    format!("Hardware monitor for current ({})", sensor),
                    pv * 0.001,
                    tags!(
                        "chip" => chip,
                        "sensor" => sensor,
                    ),
                ));
                continue;
            }

            if sensor_type == "energy" {
                metrics.push(Metric::sum_with_tags(
                    name + "_joule_total",
                    format!("Hardware monitor for joules used so far ({})", sensor),
                    pv / 1000000.0,
                    tags!(
                        "chip" => chip,
                        "sensor" => sensor,
                    ),
                ));
                continue;
            }

            if sensor_type == "power" && element == "accuracy" {
                metrics.push(Metric::gauge_with_tags(
                    name,
                    "Hardware monitor power meter accuracy, as a ratio",
                    pv / 1000000.0,
                    tags!(
                        "chip" => chip,
                        "sensor" => sensor,
                    ),
                ));
                continue;
            }

            if sensor_type == "power"
                && (element == "average_interval"
                    || element == "average_interval_min"
                    || element == "average_interval_max")
            {
                metrics.push(Metric::gauge_with_tags(
                    name + "_seconds",
                    format!("Hardware monitor power usage update interval ({})", element),
                    pv * 0.001,
                    tags!(
                        "chip" => chip,
                        "sensor" => sensor,
                    ),
                ));
                continue;
            }

            if sensor_type == "power" {
                metrics.push(Metric::gauge_with_tags(
                    name + "_watt",
                    format!("Hardware monitor for power usage in watts ({})", element),
                    pv / 1000000.0,
                    tags!(
                        "chip" => chip,
                        "sensor" => sensor,
                    ),
                ));
                continue;
            }

            if sensor_type == "humidity" {
                metrics.push(Metric::gauge_with_tags(
                    name,
                    format!("Hardware monitor for humidity, as a ratio (multiply with 100.0 to get the humidity as a percentage) ({})", element),
                    pv / 1000000.0,
                    tags!(
                        "chip" => chip,
                        "sensor" => sensor,
                    ),
                ));
                continue;
            }

            if sensor_type == "fan"
                && (element == "input"
                    || element == "min"
                    || element == "max"
                    || element == "target")
            {
                metrics.push(Metric::gauge_with_tags(
                    name + "_rpm",
                    format!(
                        "hardware monitor for fan revolutions per minute ({})",
                        element
                    ),
                    pv,
                    tags!(
                        "chip" => chip,
                        "sensor" => sensor,
                    ),
                ));
                continue;
            }

            // fallback, just dump the metric as is
            metrics.push(Metric::gauge_with_tags(
                name,
                format!("Hardware monitor {} element {}", sensor_type, element),
                pv,
                tags!(
                    "chip" => chip,
                    "sensor" => sensor,
                ),
            ));
        }
    }

    Ok(metrics)
}

// This function can be optimized by parallelling sensors
async fn collect_sensor_data(
    dir: impl AsRef<Path>,
) -> Result<BTreeMap<String, BTreeMap<String, String>>, Error> {
    let dirs = std::fs::read_dir(dir)?;
    let mut stats = BTreeMap::<String, BTreeMap<String, String>>::new();

    for result in dirs {
        match result {
            Ok(entry) => {
                let filename = entry.file_name();

                if let Ok((sensor, num, property)) =
                    explode_sensor_filename(filename.to_str().unwrap())
                {
                    if !is_hwmon_sensor(sensor) {
                        continue;
                    }

                    match read_to_string(entry.path()) {
                        Ok(value) => {
                            let sensor = format!("{}{}", sensor, num);
                            stats
                                .entry(sensor)
                                .and_modify(|properties| {
                                    properties.insert(property.to_string(), value);
                                })
                                .or_default();
                        }
                        _ => continue,
                    }
                }
            }
            Err(err) => {
                warn!(
                    message = "read sensor dir failed",
                    %err
                );
            }
        }
    }

    Ok(stats)
}

// explode_sensor_filename splits a sensor name into <type><num>_<property>
fn explode_sensor_filename(name: &str) -> Result<(&str, &str, &str), ()> {
    let s = name.as_bytes();

    let mut typ_end = 0;
    let mut num_end = 0;

    // consume type
    for (i, c) in s.iter().enumerate() {
        if c.is_ascii_digit() {
            typ_end = i;
            break;
        }
    }

    // we never meet an number
    if typ_end == 0 {
        return Err(());
    }

    // consume num until we meet '_'
    for (i, c) in s.iter().enumerate() {
        if *c == b'_' {
            num_end = i;
            break;
        }
    }

    if num_end == 0 {
        return Ok((&name[0..typ_end], &name[typ_end..name.len()], ""));
    }

    // we never meet the property separator '_'
    if num_end == typ_end {
        return Err(());
    }

    Ok((
        &name[0..typ_end],
        &name[typ_end..num_end],
        &name[num_end + 1..],
    ))
}

// human_readable_name is similar to the methods in
async fn human_readable_chip_name<P: AsRef<Path>>(dir: P) -> Result<String, Error> {
    let path = dir.as_ref().join("name");
    let content = read_to_string(path)?;
    Ok(content)
}

async fn hwmon_name(path: impl AsRef<Path>) -> Result<String, Error> {
    // generate a name for a sensor path

    // sensor numbering depends on the order of linux module loading and
    // is thus unstable.
    // However the path of the device has to be stable:
    // - /sys/devices/<bus>/<device>
    // Some hardware monitors have a "name" file that exports a human readable
    // name that can be used.

    // human readable names would be bat0 or coretemp, while a path string
    // could be platform_applesmc.768

    // preference 1: construct a name based on device name, always unique

    let path = path.as_ref();
    let ap = path.join("device");
    if read_link(&ap).is_ok() {
        let dev_path = canonicalize(ap)?;
        let dev_name = dev_path.file_name().unwrap().to_str().unwrap();
        let dev_prefix = dev_path.parent().unwrap();
        let dev_type = dev_prefix.file_name().unwrap().to_str().unwrap();

        let clean_dev_name = sanitized(dev_name);
        let clean_dev_typ = sanitized(dev_type);

        if !clean_dev_typ.is_empty() && !clean_dev_name.is_empty() {
            return Ok(format!("{}_{}", clean_dev_typ, clean_dev_name));
        }

        if !clean_dev_name.is_empty() {
            return Ok(clean_dev_name);
        }
    }

    // preference 2: is there a name file
    let name_path = path.join("name");
    match read_to_string(name_path) {
        Ok(content) => return Ok(content),
        Err(err) => debug!(
            message = "read device name failed",
            %err
        ),
    }

    // it looks bad, name and device don't provide enough information
    // return a hwmon[0-9]* name
    let name = path.file_name().unwrap().to_str().unwrap();

    Ok(name.into())
}

fn is_hwmon_sensor(s: &str) -> bool {
    [
        "vrm",
        "beep_enable",
        "update_interval",
        "in",
        "cpu",
        "fan",
        "pwm",
        "temp",
        "curr",
        "power",
        "energy",
        "humidity",
        "intrusion",
    ]
    .contains(&s)
}

static HWMON_INVALID_METRIC_CHARS: Lazy<Regex> = Lazy::new(|| Regex::new("[^a-z0-9:_]").unwrap());

fn sanitized(s: &str) -> String {
    let s = s.trim();
    let s = HWMON_INVALID_METRIC_CHARS.replace_all(s, "_");

    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gather() {
        let path = "tests/fixtures/sys".into();
        let ms = gather(path).await.unwrap();
        assert_ne!(ms.len(), 0);
    }

    #[test]
    fn test_explode_sensor_filename() {
        let input = "fan1_input";

        let (typ, id, property) = explode_sensor_filename(input).unwrap();
        assert_eq!(typ, "fan");
        assert_eq!(id, "1");
        assert_eq!(property, "input");

        let input = "fan_i";
        assert!(explode_sensor_filename(input).is_err());

        let input = "pwm1";
        let (typ, id, _) = explode_sensor_filename(input).unwrap();
        assert_eq!(typ, "pwm");
        assert_eq!(id, "1");
    }

    #[test]
    fn test_is_hwmon_sensor() {
        assert!(is_hwmon_sensor("fan"));
        assert!(!is_hwmon_sensor("foo"));
    }

    #[tokio::test]
    async fn test_collect_sensor_data() {
        let path = "tests/fixtures/sys/class/hwmon/hwmon3";
        let kvs = collect_sensor_data(path).await.unwrap();

        assert_eq!(kvs.get("fan2").unwrap().get("input").unwrap(), "1098");
        assert_eq!(kvs.get("in0").unwrap().get("max").unwrap(), "1744");
    }

    #[tokio::test]
    async fn test_hwmon_name() {
        let path = "tests/fixtures/sys/class/hwmon/hwmon2";
        let name = hwmon_name(path).await.unwrap();
        assert_eq!(name, "platform_applesmc_768")
    }
}
