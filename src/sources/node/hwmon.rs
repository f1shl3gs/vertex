use crate::{
    tags,
    sum_metric,
    gauge_metric,
    event::{Metric, MetricValue},
};
use std::path::{PathBuf};
use crate::sources::node::errors::Error;
use crate::sources::node::{read_to_string};
use std::collections::BTreeMap;
use lazy_static::lazy_static;
use regex::Regex;


pub async fn gather(sys_path: &str) -> Result<Vec<Metric>, ()> {
    let path = format!("{}/class/hwmon", sys_path);
    let mut dirs = tokio::fs::read_dir(path).await
        .map_err(|err| {
            warn!("read hwmon dir failed"; "err" => err);
        })?;

    let mut metrics = Vec::new();
    while let Some(entry) = dirs.next_entry().await.map_err(|err| {
        warn!("read next entry of hwmon dirs failed"; "err" => err);
    })? {
        let meta = entry.metadata().await.map_err(|err| {
            warn!("read metadata failed"; "err" => err);
        })?;

        let file_type = meta.file_type();
        if file_type.is_symlink() {
            match tokio::fs::read_link(entry.path()).await {
                Ok(path) => {
                    let ep = entry.path();
                    let dir = ep.to_str().unwrap();
                    match hwmon_metrics(dir).await {
                        Ok(mut ms) => metrics.append(&mut ms),
                        Err(_) => {}
                    }
                }

                Err(err) => {}
            }

            continue;
        }

        let ep = entry.path();
        let dir = ep.to_str().unwrap();
        match hwmon_metrics(dir).await {
            Ok(mut ms) => metrics.append(&mut ms),
            Err(err) => {
                warn!("hwmon_metrics error {}", err);
            }
        }
    }

    Ok(metrics)
}

async fn hwmon_metrics(dir: &str) -> Result<Vec<Metric>, Error> {
    let chip = hwmon_name(dir).await?;

    let data = {
        let result = collect_sensor_data(dir).await;
        if result.is_ok() {
            result.unwrap()
        } else {
            let path = format!("{}/device", dir);
            collect_sensor_data(&path).await?
        }
    };

    let mut metrics = Vec::new();
    if let Ok(n) = human_readable_chip_name(dir).await {
        // TODO: might we don't need to clone this
        let chip = &chip.clone();
        let chip_name = &n.clone();

        metrics.push(
            gauge_metric!(
                    "node_hwmon_chip_names",
                    "Annotation metric for human-readable chip names",
                    1f64,
                    "chip" => chip,
                    "chip_name" => chip_name
                )
        );
    }

    let chip = &chip.clone();
    for (sensor, props) in data {
        let sensor = &sensor;
        let sensor_type = match explode_sensor_filename(sensor) {
            Ok((st, _, _)) => st,
            _ => {
                warn!("unknown type {}", sensor);
                continue;
            }
        };

        match props.get("label") {
            Some(v) => {
                if v != "" {
                    metrics.push(
                        gauge_metric!(
                             "node_hwmon_sensor_label",
                             "Label for given chip and sensor",
                             1f64,
                             "label" => v,
                             "chip" => chip,
                             "sensor" => sensor
                         )
                    );
                }
            }
            _ => {}
        }

        if sensor_type == "beep_enable" {
            let mut v = 0f64;
            match props.get("") {
                Some(value) => {
                    if value == "1" {
                        v = 1.0;
                    }
                }
                None => {}
            }

            metrics.push(gauge_metric!(
                "node_hwmon_beep_enabled",
                "Hardware beep enabled",
                v,
                "chip" => chip,
                "sensor" => sensor
            ));

            continue;
        }

        if sensor_type == "vrm" {
            let v = match props.get("") {
                Some(value) => {
                    let pr = value.parse();
                    if pr.is_err() {
                        continue;
                    }

                    pr.unwrap()
                }

                None => { continue; }
            };

            metrics.push(gauge_metric!(
                "node_hwmon_voltage_regulator_version",
                "Hardware voltage regulator",
                v,
                "chip" => chip,
                "sensor" => sensor
            ));

            continue;
        }

        if sensor_type == "update_interval" {
            let pv = props.get("").unwrap_or(&"".to_string()).parse::<f64>();
            if pv.is_err() {
                continue;
            }
            let pv = pv.unwrap();

            metrics.push(gauge_metric!(
                "node_hwmon_update_interval_seconds",
                "Hardware monitor update interval",
                pv * 0.001,
                "chip" => chip,
                "sensor" => sensor
            ));

            continue;
        }

        let prefix = format!("node_hwmon_{}", sensor_type);
        for (element, value) in &props {
            if element == "label" {
                continue;
            }

            let mut name = prefix.clone();
            if element == "input" {
                // input is actually the value
                if let Some(v) = props.get("") {
                    name = name + "_input";
                }
            } else if element != "" {
                name = format!("{}_{}", name, sanitized(element));
            }

            let pv = match value.parse() {
                Ok(v) => v,
                _ => continue
            };

            // special key, fault, alarm & beep should be handed out without units
            if element == "fault" || element == "alarm" {
                let name = &name;
                let desc = &format!("Hardware sensor {} status ({})", sensor, sensor_type);

                metrics.push(gauge_metric!(
                        name,
                        desc,
                        pv,
                        "chip" => chip,
                        "sensor" => sensor
                    ));
                continue;
            }

            if element == "beep" {
                let name = name + "_enabled";
                metrics.push(gauge_metric!(
                        &name,
                        "Hardware monitor sensor has beeping enabled",
                        pv,
                        "chip" => chip,
                        "sensor" => sensor
                    ));
                continue;
            }

            // everything else should get a unit
            if sensor_type == "in" || sensor_type == "cpu" {
                let name = &format!("{}_volts", name);
                let desc = &format!("Hardware monitor for voltage ({})", element);

                metrics.push(gauge_metric!(
                        name,
                        desc,
                        pv * 0.001,
                        "chip" => chip,
                        "sensor" => sensor
                    ));
                continue;
            }

            if sensor_type == "temp" && element != "type" {
                let mut element = element;
                if element == "" {
                    element = &"input".to_string();
                }

                let name = &format!("{}_celsius", name);
                let desc = &format!("Hardware monitor for temperature ({})", sensor);
                metrics.push(gauge_metric!(
                        name,
                        desc,
                        pv * 0.001,
                        "chip" => chip,
                        "sensor" => sensor
                    ));
                continue;
            }

            if sensor_type == "curr" {
                let name = &format!("{}_amps", name);
                let desc = &format!("Hardware monitor for current ({})", sensor);
                metrics.push(gauge_metric!(
                        name,
                        desc,
                        pv * 0.001,
                        "chip" => chip,
                        "sensor" => sensor
                    ));
                continue;
            }

            if sensor_type == "energy" {
                let name = &format!("{}_joule_total", name);
                let desc = &format!("Hardware monitor for joules used so far ({})", sensor);
                metrics.push(sum_metric!(
                        name,
                        desc,
                        pv / 1000000.0,
                        "chip" => chip,
                        "sensor" => sensor
                    ));
                continue;
            }

            if sensor_type == "power" && element == "accuracy" {
                let name = &name;
                let desc = "Hardware monitor power meter accuracy, as a ratio";
                metrics.push(gauge_metric!(
                        name,
                        desc,
                        pv / 1000000.0,
                        "chip" => chip,
                        "sensor" => sensor
                    ));
                continue;
            }

            if sensor_type == "power" && (element == "average_interval" || element == "average_interval_min" || element == "average_interval_max") {
                let name = &format!("{}_seconds", name);
                let desc = &format!("Hardware monitor power usage update interval ({})", element);
                metrics.push(gauge_metric!(
                        name,
                        desc,
                        pv * 0.001,
                        "chip" => chip,
                        "sensor" => sensor
                    ));
                continue;
            }

            if sensor_type == "power" {
                let name = &(name + "_watt");
                let desc = &format!("Hardware monitor for power usage in watts ({})", element);
                metrics.push(gauge_metric!(
                        name,
                        desc,
                        pv /1000000.0,
                        "chip" => chip,
                        "sensor" => sensor
                    ));
                continue;
            }

            if sensor_type == "humidity" {
                let name = &name;
                let desc = &format!("Hardware monitor for humidity, as a ratio (multiply with 100.0 to get the humidity as a percentage) ({})", element);
                metrics.push(gauge_metric!(
                        name,
                        desc,
                        pv /1000000.0,
                        "chip" => chip,
                        "sensor" => sensor
                    ));
                continue;
            }

            if sensor_type == "fan" && (element == "input" || element == "min" || element == "max" || element == "target") {
                let name = &(name + "_rpm");
                let desc = &format!("hardware monitor for fan revolutions per minute ({})", element);
                metrics.push(gauge_metric!(
                        name,
                        desc,
                        pv,
                        "chip" => chip,
                        "sensor" => sensor
                    ));
                continue;
            }

            // fallback, just dump the metric as is
            let name = &name;
            let desc = &format!("Hardware monitor {} element {}", sensor_type, element);
            metrics.push(gauge_metric!(
                    name,
                    desc,
                    pv,
                    "chip" => chip,
                    "sensor" => sensor
                ));
        }
    }

    Ok(metrics)
}

async fn collect_sensor_data(dir: &str) -> Result<BTreeMap<String, BTreeMap<String, String>>, Error> {
    let mut dirs = tokio::fs::read_dir(dir).await?;

    let mut stats = BTreeMap::<String, BTreeMap<String, String>>::new();
    while let Some(entry) = dirs.next_entry().await? {
        let path = entry.path().clone();
        let filename = path.file_name().unwrap().to_str().unwrap();

        if let Ok((sensor, num, property)) = explode_sensor_filename(filename) {
            if !is_hwmon_sensor(sensor) {
                continue;
            }

            let v = read_to_string(entry.path()).await;

            match read_to_string(entry.path()).await {
                Ok(v) => {
                    let sensor = format!("{}{}", sensor, num);
                    if !stats.contains_key(&sensor) {
                        stats.insert(sensor.clone(), BTreeMap::new());
                    }

                    let props = stats.get_mut(&sensor).unwrap();
                    props.insert(property.to_string(), v.trim().to_string());
                }
                _ => continue
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
    for i in 0..s.len() {
        let c = s[i];
        if c >= b'0' && c <= b'9' {
            typ_end = i;
            break;
        }
    }

    // we never meet an number
    if typ_end == 0 {
        return Err(());
    }

    // consume num until we meet '_'
    for i in typ_end..s.len() {
        let c = s[i];
        if c == b'_' {
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

    Ok((&name[0..typ_end], &name[typ_end..num_end], &name[num_end + 1..]))
}

// human_readable_name is similar to the methods in
async fn human_readable_chip_name(dir: &str) -> Result<String, Error> {
    let path = format!("{}/name", dir);
    let content = read_to_string(path).await?;
    Ok(content.trim().to_string())
}

async fn hwmon_name(path: &str) -> Result<String, Error> {
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

    match tokio::fs::read_link(format!("{}/device", path)).await {
        Ok(dev_path) => {
            let dev_path = tokio::fs::canonicalize(format!("{}/device", path)).await?;
            let dev_name = dev_path.file_name().unwrap().to_str().unwrap();
            let dev_prefix = dev_path.parent().unwrap();
            let dev_type = dev_prefix.file_name().unwrap().to_str().unwrap();

            let clean_dev_name = sanitized(dev_name);
            let clean_dev_typ = sanitized(dev_type);

            if clean_dev_typ != "" && clean_dev_name != "" {
                return Ok(format!("{}_{}", clean_dev_typ, clean_dev_name))
            }

            if clean_dev_name != "" {
                return Ok(clean_dev_name);
            }
        }
        Err(err) => {}
    }

    // preference 2: is there a name file
    let name_path = format!("{}/name", path);
    match read_to_string(name_path).await {
        Ok(content) => return Ok(content.trim().to_string()),
        Err(err) => debug!("read device name failed"; "err" => err)
    }

    // it looks bad, name and device don't provide enough information
    // return a hwmon[0-9]* name
    let path = PathBuf::from(path);
    let name = path.file_name().unwrap().to_str().unwrap();

    Ok(name.trim().to_string())
}

fn is_hwmon_sensor(s: &str) -> bool {
    ["vrm", "beep_enable", "update_interval", "in", "cpu", "fan",
        "pwm", "temp", "curr", "power", "energy", "humidity",
        "intrusion", ].contains(&s)
}

lazy_static! {
    static ref HWMON_INVALID_METRIC_CHARS: Regex = Regex::new("[^a-z0-9:_]").unwrap();
}

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
        let path = "testdata/sys";

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
        assert_eq!(is_hwmon_sensor("foo"), false);
    }

    #[tokio::test]
    async fn test_collect_sensor_data() {
        let path = "testdata/sys/class/hwmon/hwmon3";
        let kvs = collect_sensor_data(path).await.unwrap();

        println!("{:?}", kvs);
    }

    #[tokio::test]
    async fn test_hwmon_name() {
        let path = "/sys/class/hwmon/hwmon2";
        let name = hwmon_name(path).await.unwrap();
        assert_eq!(name, "platform_eeepc_wmi")
    }
}
