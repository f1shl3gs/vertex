use std::collections::BTreeMap;
use std::fs::{canonicalize, read_link};
use std::path::{Path, PathBuf};

use event::{tags, Metric};
use tokio::task::JoinSet;

use super::{read_string, Error};

pub async fn gather(sys_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let dirs = std::fs::read_dir(sys_path.join("class/hwmon"))?;

    let mut tasks = JoinSet::new();
    for entry in dirs.flatten() {
        let meta = entry.metadata()?;
        let meta = match meta.file_type().is_symlink() {
            true => canonicalize(entry.path())?.metadata()?,
            false => meta,
        };

        if !meta.is_dir() {
            continue;
        }

        tasks.spawn(async move { hwmon_metrics(entry.path()) });
    }

    let mut metrics = vec![];
    while let Some(Ok(result)) = tasks.join_next().await {
        match result {
            Ok(partial) => metrics.extend(partial),
            Err(err) => {
                warn!(message = "gather metrics failed", %err);
            }
        }
    }

    Ok(metrics)
}

fn hwmon_metrics(dir: PathBuf) -> Result<Vec<Metric>, Error> {
    let chip = &hwmon_name(&dir)?;

    let mut data = collect_sensor_data(&dir)?;
    let dev_path = dir.join("device");
    if std::fs::exists(&dev_path)? {
        let device_data = collect_sensor_data(&dev_path)?;

        for (key, dev_props) in device_data {
            match data.get_mut(&key) {
                Some(dst) => {
                    for (k, v) in dev_props {
                        dst.insert(k, v);
                    }
                }
                None => {
                    data.insert(key, dev_props);
                }
            }
        }
    }

    let mut metrics = Vec::new();
    if let Ok(chip_name) = human_readable_chip_name(&dir) {
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

    // format all sensors.
    for (sensor, props) in &data {
        let sensor_type = match explode_sensor_filename(sensor) {
            Ok((st, _, _)) => st,
            _ => "",
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
            let v = matches!(props.get(""), Some(v) if v == "1");

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
            let value = match props.get("") {
                Some(value) => match value.parse::<f64>() {
                    Ok(v) => v,
                    Err(_err) => continue,
                },
                None => {
                    continue;
                }
            };

            metrics.push(Metric::gauge_with_tags(
                "node_hwmon_voltage_regulator_version",
                "Hardware voltage regulator",
                value,
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
                if props.contains_key("") {
                    name += "_input";
                }
            } else if !element.is_empty() {
                name = format!("{}_{}", name, sanitized(element));
            }

            let pv = match value.parse::<f64>() {
                Ok(v) => v,
                _ => continue,
            };

            // special elements, fault, alarm & beep should be handed out without units
            if element == "fault" || element == "alarm" {
                metrics.push(Metric::gauge_with_tags(
                    name,
                    format!("Hardware sensor {} status ({})", element, sensor_type),
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
                    format!("Hardware monitor for current ({})", element),
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
                        "Hardware monitor for fan revolutions per minute ({})",
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
fn collect_sensor_data(
    dir: impl AsRef<Path>,
) -> Result<BTreeMap<String, BTreeMap<String, String>>, Error> {
    let dirs = std::fs::read_dir(dir)?;
    let mut stats = BTreeMap::<String, BTreeMap<String, String>>::new();

    for entry in dirs.flatten() {
        let filename = entry.file_name();

        if let Ok((sensor, num, property)) = explode_sensor_filename(filename.to_str().unwrap()) {
            if !is_hwmon_sensor(sensor) {
                continue;
            }

            match read_string(entry.path()) {
                Ok(value) => {
                    let sensor = format!("{}{}", sensor, if num.is_empty() { "0" } else { num });
                    stats
                        .entry(sensor)
                        .or_default()
                        .insert(property.to_string(), value);
                }
                _ => continue,
            }
        }
    }

    Ok(stats)
}

// explode_sensor_filename splits a sensor name into <type><num>_<property>
fn explode_sensor_filename(name: &str) -> Result<(&str, &str, &str), ()> {
    let input = name.as_bytes();

    let mut found_underscore = false;
    let mut num_start = 0;
    while num_start < input.len() {
        if input[num_start].is_ascii_digit() {
            break;
        }

        if input[num_start] == b'_' {
            found_underscore = true;
        }

        num_start += 1;
    }
    if num_start >= input.len() {
        // num is not found
        if found_underscore {
            return Err(());
        }

        return Ok((&name[..num_start], &name[num_start..], &name[num_start..]));
    }

    let mut num_end = num_start;
    while num_end < input.len() {
        if !input[num_end].is_ascii_digit() {
            break;
        }

        num_end += 1;
    }
    if num_end >= input.len() {
        return Ok((
            &name[0..num_start],
            &name[num_start..num_end],
            &name[num_end..],
        ));
    }

    Ok((
        &name[0..num_start],
        &name[num_start..num_end],
        &name[num_end + 1..],
    ))
}

// human_readable_name is similar to the methods in
fn human_readable_chip_name(dir: &Path) -> Result<String, Error> {
    let content = read_string(dir.join("name"))?;
    Ok(content)
}

fn hwmon_name(path: &Path) -> Result<String, Error> {
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

    let dev_path = path.join("device");
    if read_link(&dev_path).is_ok() {
        let dev_path = canonicalize(dev_path)?;
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
    match read_string(path.join("name")) {
        Ok(content) => return Ok(content),
        Err(err) => debug!(
            message = "read device name failed",
            %err
        ),
    }

    // it looks bad, name and device don't provide enough information
    // return a hwmon[0-9]* name
    let name = path.file_name().unwrap().to_string_lossy().to_string();

    Ok(name)
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

fn sanitized(s: &str) -> String {
    let mut buf = s.to_string();
    for ch in unsafe { buf.as_bytes_mut() } {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() || *ch == b':' {
            continue;
        }

        // convert to lower case
        if ch.is_ascii_uppercase() {
            // A: 65
            // a: 97
            *ch += 32;
            continue;
        }

        *ch = b'_';
    }

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_names() {
        for (input, want) in [
            ("nvme", "nvme"),
            ("nct6775.656", "nct6775_656"),
            ("0000:0c:00.0", "0000:0c:00_0"),
            ("asus-ec-sensors", "asus_ec_sensors"),
            ("bEeP", "beep"),
        ] {
            let got = sanitized(input);
            assert_eq!(got, want, "input: {}", input);
        }
    }

    #[tokio::test]
    async fn test_gather() {
        let path = "tests/node/sys".into();
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

    #[test]
    fn test_collect_sensor_data() {
        let path = "tests/node/sys/class/hwmon/hwmon3";
        let kvs = collect_sensor_data(path).unwrap();

        assert_eq!(kvs.get("fan2").unwrap().get("input").unwrap(), "1098");
        assert_eq!(kvs.get("in0").unwrap().get("max").unwrap(), "1744");
    }

    #[test]
    fn test_hwmon_name() {
        let path = PathBuf::from("tests/node/sys/class/hwmon/hwmon2");
        let name = hwmon_name(&path).unwrap();
        assert_eq!(name, "platform_applesmc_768")
    }
}
