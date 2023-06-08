use super::{read_to_string, Error};
use event::{tags, Metric};
use framework::config::serde_regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// PowerSupply contains info from files in /sys/class/power_supply for
/// a single power supply
#[derive(Debug, Default, PartialEq)]
struct PowerSupply {
    // Power Supply Name
    name: String,
    // /sys/class/power_supply/<name>/authentic
    authentic: Option<i64>,
    // /sys/class/power_supply/<name>/calibrate
    calibrate: Option<i64>,
    // /sys/class/power_supply/<name>/capacity
    capacity: Option<i64>,
    // /sys/class/power_supply/<name>/capacity_alert_max
    capacity_alert_max: Option<i64>,
    // /sys/class/power_supply/<name>/capacity_alert_min
    capacity_alert_min: Option<i64>,
    // /sys/class/power_supply/<name>/capacity_level
    capacity_level: String,
    // /sys/class/power_supply/<name>/charge_avg
    charge_avg: Option<i64>,
    // /sys/class/power_supply/<name>/charge_control_limit
    charge_control_limit: Option<i64>,
    // /sys/class/power_supply/<name>/charge_control_limit_max
    charge_control_limit_max: Option<i64>,
    // /sys/class/power_supply/<name>/charge_counter
    charge_counter: Option<i64>,
    // /sys/class/power_supply/<name>/charge_empty
    charge_empty: Option<i64>,
    // /sys/class/power_supply/<name>/charge_empty_design
    charge_empty_design: Option<i64>,
    // /sys/class/power_supply/<name>/charge_full
    charge_full: Option<i64>,
    // /sys/class/power_supply/<name>/charge_full_design
    charge_full_design: Option<i64>,
    // /sys/class/power_supply/<name>/charge_now
    charge_now: Option<i64>,
    // /sys/class/power_supply/<name>/charge_term_current
    charge_term_current: Option<i64>,
    // /sys/class/power_supply/<name>/charge_type
    charge_type: String,
    // /sys/class/power_supply/<name>/constant_charge_current
    constant_charge_current: Option<i64>,
    // /sys/class/power_supply/<name>/constant_charge_current_max
    constant_charge_current_max: Option<i64>,
    // /sys/class/power_supply/<name>/constant_charge_voltage
    constant_charge_voltage: Option<i64>,
    // /sys/class/power_supply/<name>/constant_charge_voltage_max
    constant_charge_voltage_max: Option<i64>,
    // /sys/class/power_supply/<name>/current_avg
    current_avg: Option<i64>,
    // /sys/class/power_supply/<name>/current_boot
    current_boot: Option<i64>,
    // /sys/class/power_supply/<name>/current_max
    current_max: Option<i64>,
    // /sys/class/power_supply/<name>/current_now
    current_now: Option<i64>,
    // /sys/class/power_supply/<name>/cycle_count
    cycle_count: Option<i64>,
    // /sys/class/power_supply/<name>/energy_avg
    energy_avg: Option<i64>,
    // /sys/class/power_supply/<name>/energy_empty
    energy_empty: Option<i64>,
    // /sys/class/power_supply/<name>/energy_empty_design
    energy_empty_design: Option<i64>,
    // /sys/class/power_supply/<name>/energy_full
    energy_full: Option<i64>,
    // /sys/class/power_supply/<name>/energy_full_design
    energy_full_design: Option<i64>,
    // /sys/class/power_supply/<name>/energy_now
    energy_now: Option<i64>,
    // /sys/class/power_supply/<name>/health
    health: String,
    // /sys/class/power_supply/<name>/input_current_limit
    input_current_limit: Option<i64>,
    // /sys/class/power_supply/<name>/manufacturer
    manufacturer: String,
    // /sys/class/power_supply/<name>/model_name
    model_name: String,
    // /sys/class/power_supply/<name>/online
    online: Option<i64>,
    // /sys/class/power_supply/<name>/power_avg
    power_avg: Option<i64>,
    // /sys/class/power_supply/<name>/power_now
    power_now: Option<i64>,
    // /sys/class/power_supply/<name>/precharge_current
    precharge_current: Option<i64>,
    // /sys/class/power_supply/<name>/present
    present: Option<i64>,
    // /sys/class/power_supply/<name>/scope
    scope: String,
    // /sys/class/power_supply/<name>/serial_number
    serial_number: String,
    // /sys/class/power_supply/<name>/status
    status: String,
    // /sys/class/power_supply/<name>/technology
    technology: String,
    // /sys/class/power_supply/<name>/temp
    temp: Option<i64>,
    // /sys/class/power_supply/<name>/temp_alert_max
    temp_alert_max: Option<i64>,
    // /sys/class/power_supply/<name>/temp_alert_min
    temp_alert_min: Option<i64>,
    // /sys/class/power_supply/<name>/temp_ambient
    temp_ambient: Option<i64>,
    // /sys/class/power_supply/<name>/temp_ambient_max
    temp_ambient_max: Option<i64>,
    // /sys/class/power_supply/<name>/temp_ambient_min
    temp_ambient_min: Option<i64>,
    // /sys/class/power_supply/<name>/temp_max
    temp_max: Option<i64>,
    // /sys/class/power_supply/<name>/temp_min
    temp_min: Option<i64>,
    // /sys/class/power_supply/<name>/time_to_empty_avg
    time_to_empty_avg: Option<i64>,
    // /sys/class/power_supply/<name>/time_to_empty_now
    time_to_empty_now: Option<i64>,
    // /sys/class/power_supply/<name>/time_to_full_avg
    time_to_full_avg: Option<i64>,
    // /sys/class/power_supply/<name>/time_to_full_now
    time_to_full_now: Option<i64>,
    // /sys/class/power_supply/<name>/type
    typ: String,
    // /sys/class/power_supply/<name>/usb_type
    usb_type: String,
    // /sys/class/power_supply/<name>/voltage_avg
    voltage_avg: Option<i64>,
    // /sys/class/power_supply/<name>/voltage_boot
    voltage_boot: Option<i64>,
    // /sys/class/power_supply/<name>/voltage_max
    voltage_max: Option<i64>,
    // /sys/class/power_supply/<name>/voltage_max_design
    voltage_max_design: Option<i64>,
    // /sys/class/power_supply/<name>/voltage_min
    voltage_min: Option<i64>,
    // /sys/class/power_supply/<name>/voltage_min_design
    voltage_min_design: Option<i64>,
    // /sys/class/power_supply/<name>/voltage_now
    voltage_now: Option<i64>,
    // /sys/class/power_supply/<name>/voltage_ocv
    voltage_ocv: Option<i64>,
}

/// power_supply_class returns info for all power supplies read
/// from /sys/class/power_supply
async fn power_supply_class<P: AsRef<Path>>(root: P) -> Result<Vec<PowerSupply>, Error> {
    let path = root.as_ref().join("class/power_supply");
    let mut readdir = tokio::fs::read_dir(path).await?;

    let mut pss = vec![];
    while let Some(entry) = readdir.next_entry().await? {
        let ps = parse_power_supply(entry.path()).await?;
        pss.push(ps);
    }

    Ok(pss)
}

async fn parse_power_supply(path: PathBuf) -> Result<PowerSupply, Error> {
    // parse name from path
    let name = path.file_name().unwrap().to_str().unwrap();
    let mut ps = PowerSupply {
        name: name.to_string(),
        ..Default::default()
    };

    let mut readdir = tokio::fs::read_dir(path).await?;
    while let Some(entry) = readdir.next_entry().await? {
        // TODO: node_exporter will skip none regular entry
        let meta = entry.metadata().await?;
        if !meta.is_file() {
            continue;
        }

        let content = match read_to_string(entry.path()).await {
            Ok(c) => c,
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    continue;
                }

                return Err(err.into());
            }
        };

        let name = entry.file_name();
        let name = name.to_str().unwrap();
        match name {
            "authentic" => ps.authentic = content.parse().ok(),
            "calibrate" => ps.calibrate = content.parse().ok(),
            "capacity" => ps.capacity = content.parse().ok(),
            "capacity_alert_max" => ps.capacity_alert_max = content.parse().ok(),
            "capacity_alert_min" => ps.capacity_alert_min = content.parse().ok(),
            "capacity_level" => ps.capacity_level = content.to_string(),
            "charge_avg" => ps.charge_avg = content.parse().ok(),
            "charge_control_limit" => ps.charge_control_limit = content.parse().ok(),
            "charge_control_limit_max" => ps.charge_control_limit_max = content.parse().ok(),
            "charge_counter" => ps.charge_counter = content.parse().ok(),
            "charge_empty" => ps.charge_empty = content.parse().ok(),
            "charge_empty_design" => ps.charge_empty_design = content.parse().ok(),
            "charge_full" => ps.charge_full = content.parse().ok(),
            "charge_full_design" => ps.charge_full_design = content.parse().ok(),
            "charge_now" => ps.charge_now = content.parse().ok(),
            "charge_term_current" => ps.charge_term_current = content.parse().ok(),
            "charge_type" => ps.charge_type = content.to_string(),
            "constant_charge_current" => ps.constant_charge_current = content.parse().ok(),
            "constant_charge_current_max" => ps.constant_charge_current_max = content.parse().ok(),
            "constant_charge_voltage" => ps.constant_charge_voltage = content.parse().ok(),
            "constant_charge_voltage_max" => ps.constant_charge_voltage_max = content.parse().ok(),
            "current_avg" => ps.current_avg = content.parse().ok(),
            "current_boot" => ps.current_boot = content.parse().ok(),
            "current_max" => ps.current_max = content.parse().ok(),
            "current_now" => ps.current_now = content.parse().ok(),
            "cycle_count" => ps.cycle_count = content.parse().ok(),
            "energy_avg" => ps.energy_avg = content.parse().ok(),
            "energy_empty" => ps.energy_empty = content.parse().ok(),
            "energy_empty_design" => ps.energy_empty_design = content.parse().ok(),
            "energy_full" => ps.energy_full = content.parse().ok(),
            "energy_full_design" => ps.energy_full_design = content.parse().ok(),
            "energy_now" => ps.energy_now = content.parse().ok(),
            "health" => ps.health = content.to_string(),
            "input_current_limit" => ps.input_current_limit = content.parse().ok(),
            "manufacturer" => ps.manufacturer = content.to_string(),
            "model_name" => ps.model_name = content.to_string(),
            "online" => ps.online = content.parse().ok(),
            "power_avg" => ps.power_avg = content.parse().ok(),
            "power_now" => ps.power_now = content.parse().ok(),
            "precharge_current" => ps.precharge_current = content.parse().ok(),
            "present" => ps.present = content.parse().ok(),
            "scope" => ps.scope = content.to_string(),
            "serial_number" => ps.serial_number = content.to_string(),
            "status" => ps.status = content.to_string(),
            "technology" => ps.technology = content.to_string(),
            "temp" => ps.temp = content.parse().ok(),
            "temp_alert_max" => ps.temp_alert_max = content.parse().ok(),
            "temp_alert_min" => ps.temp_alert_min = content.parse().ok(),
            "temp_ambient" => ps.temp_ambient = content.parse().ok(),
            "temp_ambient_max" => ps.temp_ambient_max = content.parse().ok(),
            "temp_ambient_min" => ps.temp_ambient_min = content.parse().ok(),
            "temp_max" => ps.temp_max = content.parse().ok(),
            "temp_min" => ps.temp_min = content.parse().ok(),
            "time_to_empty_avg" => ps.time_to_empty_avg = content.parse().ok(),
            "time_to_empty_now" => ps.time_to_empty_now = content.parse().ok(),
            "time_to_full_avg" => ps.time_to_full_avg = content.parse().ok(),
            "time_to_full_now" => ps.time_to_full_now = content.parse().ok(),
            "type" => ps.typ = content.to_string(),
            "usb_type" => ps.usb_type = content.to_string(),
            "voltage_avg" => ps.voltage_avg = content.parse().ok(),
            "voltage_boot" => ps.voltage_boot = content.parse().ok(),
            "voltage_max" => ps.voltage_max = content.parse().ok(),
            "voltage_max_design" => ps.voltage_max_design = content.parse().ok(),
            "voltage_min" => ps.voltage_min = content.parse().ok(),
            "voltage_min_design" => ps.voltage_min_design = content.parse().ok(),
            "voltage_now" => ps.voltage_now = content.parse().ok(),
            "voltage_ocv" => ps.voltage_ocv = content.parse().ok(),
            _ => continue,
        }
    }

    Ok(ps)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PowerSupplyConfig {
    #[serde(with = "serde_regex")]
    pub ignored: regex::Regex,
}

impl Default for PowerSupplyConfig {
    fn default() -> Self {
        Self {
            ignored: regex::Regex::new("^$").unwrap(),
        }
    }
}

macro_rules! power_supply_metric {
    ($vec: expr, $power_supply: expr, $name: expr, $value: expr) => {
        if let Some(v) = $value {
            // let prefix = "node_power_supply_".to_string()
            $vec.push(Metric::gauge_with_tags(
                "node_power_supply_".to_string() + $name,
                "value of /sys/class/power_supply/<power_supply>/".to_string() + $name,
                v as f64,
                tags! {
                    "power_supply" => $power_supply
                },
            ))
        }
    };
}

macro_rules! power_supply_metric_divide_e6 {
    ($vec: expr, $power_supply: expr, $name: expr, $value: expr) => {
        if let Some(v) = $value {
            $vec.push(Metric::gauge_with_tags(
                "node_power_supply_".to_string() + $name,
                "value of /sys/class/power_supply/<power_supply>/".to_string() + $name,
                v as f64 / 1e6,
                tags! {
                    "power_supply" => $power_supply
                },
            ))
        }
    };
}

macro_rules! power_supply_metric_divide_10 {
    ($vec: expr, $power_supply: expr, $name: expr, $value: expr) => {
        if let Some(v) = $value {
            $vec.push(Metric::gauge_with_tags(
                "node_power_supply_".to_string() + $name,
                "value of /sys/class/power_supply/<power_supply>/".to_string() + $name,
                v as f64 / 10.0,
                tags! {
                    "power_supply" => $power_supply
                },
            ))
        }
    };
}

pub async fn gather(sys_path: &str, conf: &PowerSupplyConfig) -> Result<Vec<Metric>, Error> {
    let psc = power_supply_class(sys_path).await?;
    let mut metrics = vec![];
    for ps in psc {
        if conf.ignored.is_match(&ps.name) {
            continue;
        }

        power_supply_metric!(metrics, &ps.name, "authentic", ps.authentic);
        power_supply_metric!(metrics, &ps.name, "calibrate", ps.calibrate);
        power_supply_metric!(
            metrics,
            &ps.name,
            "capacity_alert_max",
            ps.capacity_alert_max
        );
        power_supply_metric!(
            metrics,
            &ps.name,
            "capacity_alert_min",
            ps.capacity_alert_min
        );
        power_supply_metric!(metrics, &ps.name, "cyclecount", ps.cycle_count);
        power_supply_metric!(metrics, &ps.name, "online", ps.online);
        power_supply_metric!(metrics, &ps.name, "present", ps.present);
        power_supply_metric!(
            metrics,
            &ps.name,
            "time_to_empty_seconds",
            ps.time_to_empty_now
        );
        power_supply_metric!(
            metrics,
            &ps.name,
            "time_to_full_seconds",
            ps.time_to_full_now
        );

        power_supply_metric_divide_e6!(metrics, &ps.name, "current_boot", ps.current_boot);
        power_supply_metric_divide_e6!(metrics, &ps.name, "current_max", ps.current_max);
        power_supply_metric_divide_e6!(metrics, &ps.name, "current_ampere", ps.current_now);
        power_supply_metric_divide_e6!(metrics, &ps.name, "energy_empty", ps.energy_empty);
        power_supply_metric_divide_e6!(
            metrics,
            &ps.name,
            "energy_empty_design",
            ps.energy_empty_design
        );
        power_supply_metric_divide_e6!(metrics, &ps.name, "energy_full", ps.energy_full);
        power_supply_metric_divide_e6!(
            metrics,
            &ps.name,
            "energy_full_design",
            ps.energy_full_design
        );
        power_supply_metric_divide_e6!(metrics, &ps.name, "energy_watthour", ps.energy_now);
        power_supply_metric_divide_e6!(metrics, &ps.name, "voltage_boot", ps.voltage_boot);
        power_supply_metric_divide_e6!(metrics, &ps.name, "voltage_max", ps.voltage_max);
        power_supply_metric_divide_e6!(
            metrics,
            &ps.name,
            "voltage_max_design",
            ps.voltage_max_design
        );
        power_supply_metric_divide_e6!(metrics, &ps.name, "voltage_min", ps.voltage_min);
        power_supply_metric_divide_e6!(
            metrics,
            &ps.name,
            "voltage_min_design",
            ps.voltage_min_design
        );
        power_supply_metric_divide_e6!(metrics, &ps.name, "voltage_volt", ps.voltage_now);
        power_supply_metric_divide_e6!(metrics, &ps.name, "voltage_ocv", ps.voltage_ocv);
        power_supply_metric_divide_e6!(
            metrics,
            &ps.name,
            "charge_control_limit",
            ps.charge_control_limit
        );
        power_supply_metric_divide_e6!(
            metrics,
            &ps.name,
            "charge_control_limit_max",
            ps.charge_control_limit_max
        );
        power_supply_metric_divide_e6!(metrics, &ps.name, "charge_counter", ps.charge_counter);
        power_supply_metric_divide_e6!(metrics, &ps.name, "charge_empty", ps.charge_empty);
        power_supply_metric_divide_e6!(
            metrics,
            &ps.name,
            "charge_empty_design",
            ps.charge_empty_design
        );
        power_supply_metric_divide_e6!(metrics, &ps.name, "charge_full", ps.charge_full);
        power_supply_metric_divide_e6!(
            metrics,
            &ps.name,
            "charge_full_design",
            ps.charge_full_design
        );
        power_supply_metric_divide_e6!(metrics, &ps.name, "charge_ampere", ps.charge_now);
        power_supply_metric_divide_e6!(
            metrics,
            &ps.name,
            "charge_term_current",
            ps.charge_term_current
        );
        power_supply_metric_divide_e6!(
            metrics,
            &ps.name,
            "constant_charge_current",
            ps.constant_charge_current
        );
        power_supply_metric_divide_e6!(
            metrics,
            &ps.name,
            "constant_charge_current_max",
            ps.constant_charge_current_max
        );
        power_supply_metric_divide_e6!(
            metrics,
            &ps.name,
            "constant_charge_voltage",
            ps.constant_charge_voltage
        );
        power_supply_metric_divide_e6!(
            metrics,
            &ps.name,
            "constant_charge_voltage_max",
            ps.constant_charge_voltage_max
        );
        power_supply_metric_divide_e6!(
            metrics,
            &ps.name,
            "precharge_current",
            ps.precharge_current
        );
        power_supply_metric_divide_e6!(
            metrics,
            &ps.name,
            "input_current_limit",
            ps.input_current_limit
        );
        power_supply_metric_divide_e6!(metrics, &ps.name, "power_watt", ps.power_now);

        power_supply_metric_divide_10!(metrics, &ps.name, "temp_celsius", ps.temp);
        power_supply_metric_divide_10!(
            metrics,
            &ps.name,
            "temp_alert_max_celsius",
            ps.temp_alert_max
        );
        power_supply_metric_divide_10!(
            metrics,
            &ps.name,
            "temp_alert_min_celsius",
            ps.temp_alert_min
        );
        power_supply_metric_divide_10!(metrics, &ps.name, "temp_ambient_celsius", ps.temp_ambient);
        power_supply_metric_divide_10!(
            metrics,
            &ps.name,
            "temp_ambient_max_celsius",
            ps.temp_ambient_max
        );
        power_supply_metric_divide_10!(
            metrics,
            &ps.name,
            "temp_ambient_min_celsius",
            ps.temp_ambient_min
        );
        power_supply_metric_divide_10!(metrics, &ps.name, "temp_max_celsius", ps.temp_max);
        power_supply_metric_divide_10!(metrics, &ps.name, "temp_min_celsius", ps.temp_min);

        let mut m = tags!(
            "power_supply" => &ps.name
        );
        if !ps.capacity_level.is_empty() {
            m.insert("capacity_level", ps.capacity_level);
        }

        if !ps.charge_type.is_empty() {
            m.insert("charge_type", ps.charge_type);
        }

        if !ps.health.is_empty() {
            m.insert("health", ps.health);
        }

        if !ps.manufacturer.is_empty() {
            m.insert("manufacturer", ps.manufacturer);
        }

        if !ps.model_name.is_empty() {
            m.insert("model_name", ps.model_name);
        }

        if !ps.serial_number.is_empty() {
            m.insert("serial_number", ps.serial_number);
        }

        if !ps.status.is_empty() {
            m.insert("status", ps.status);
        }

        if !ps.technology.is_empty() {
            m.insert("technology", ps.technology);
        }

        if !ps.typ.is_empty() {
            m.insert("type", ps.typ);
        }

        if !ps.usb_type.is_empty() {
            m.insert("usb_type", ps.usb_type);
        }

        if !ps.scope.is_empty() {
            m.insert("scope", ps.scope);
        }

        metrics.push(Metric::gauge_with_tags(
            "node_power_supply_info",
            "info of /sys/class/power_supply/<power_supply>.",
            1,
            m,
        ))
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_power_supply_class() {
        let root = "tests/fixtures/sys";
        let mut pss = power_supply_class(root).await.unwrap();

        // The readdir_r is not guaranteed to return in any specific order.
        // And the order of Github CI and Centos Stream is different, so it must be sorted
        // See: https://utcc.utoronto.ca/~cks/space/blog/unix/ReaddirOrder
        pss.sort_by(|a, b| a.name.cmp(&b.name));

        assert_eq!(pss.len(), 2);
        assert_eq!(
            pss[0],
            PowerSupply {
                name: "AC".to_string(),
                authentic: None,
                calibrate: None,
                capacity: None,
                capacity_alert_max: None,
                capacity_alert_min: None,
                capacity_level: "".to_string(),
                charge_avg: None,
                charge_control_limit: None,
                charge_control_limit_max: None,
                charge_counter: None,
                charge_empty: None,
                charge_empty_design: None,
                charge_full: None,
                charge_full_design: None,
                charge_now: None,
                charge_term_current: None,
                charge_type: "".to_string(),
                constant_charge_current: None,
                constant_charge_current_max: None,
                constant_charge_voltage: None,
                constant_charge_voltage_max: None,
                current_avg: None,
                current_boot: None,
                current_max: None,
                current_now: None,
                cycle_count: None,
                energy_avg: None,
                energy_empty: None,
                energy_empty_design: None,
                energy_full: None,
                energy_full_design: None,
                energy_now: None,
                health: "".to_string(),
                input_current_limit: None,
                manufacturer: "".to_string(),
                typ: "Mains".to_string(),
                usb_type: "".to_string(),
                voltage_avg: None,
                voltage_boot: None,
                voltage_max: None,
                voltage_max_design: None,
                voltage_min: None,
                voltage_min_design: None,
                voltage_now: None,
                online: Some(0),
                power_avg: None,
                power_now: None,
                precharge_current: None,
                present: None,
                scope: "".to_string(),
                serial_number: "".to_string(),
                status: "".to_string(),
                technology: "".to_string(),
                temp: None,
                temp_alert_max: None,
                temp_alert_min: None,
                temp_ambient: None,
                temp_ambient_max: None,
                temp_ambient_min: None,
                temp_max: None,
                temp_min: None,
                time_to_empty_avg: None,
                time_to_empty_now: None,
                time_to_full_avg: None,
                model_name: "".to_string(),
                time_to_full_now: None,
                voltage_ocv: None,
            }
        );

        assert_eq!(
            pss[1],
            PowerSupply {
                name: "BAT0".to_string(),
                authentic: None,
                calibrate: None,
                capacity: Some(98),
                capacity_alert_max: None,
                capacity_alert_min: None,
                capacity_level: "Normal".to_string(),
                charge_avg: None,
                charge_control_limit: None,
                charge_control_limit_max: None,
                charge_counter: None,
                charge_empty: None,
                charge_empty_design: None,
                charge_full: None,
                charge_full_design: None,
                charge_now: None,
                charge_term_current: None,
                charge_type: "".to_string(),
                constant_charge_current: None,
                constant_charge_current_max: None,
                constant_charge_voltage: None,
                constant_charge_voltage_max: None,
                current_avg: None,
                current_boot: None,
                current_max: None,
                current_now: None,
                cycle_count: Some(0),
                energy_avg: None,
                energy_empty: None,
                energy_empty_design: None,
                energy_full: Some(50060000),
                energy_full_design: Some(47520000),
                energy_now: Some(49450000),
                health: "".to_string(),
                input_current_limit: None,
                manufacturer: "LGC".to_string(),
                model_name: "LNV-45N1".to_string(),
                online: None,
                power_avg: None,
                power_now: Some(4830000),
                precharge_current: None,
                present: Some(1),
                scope: "".to_string(),
                serial_number: "38109".to_string(),
                status: "Discharging".to_string(),
                technology: "Li-ion".to_string(),
                temp: None,
                temp_alert_max: None,
                temp_alert_min: None,
                temp_ambient: None,
                temp_ambient_max: None,
                temp_ambient_min: None,
                temp_max: None,
                temp_min: None,
                time_to_empty_avg: None,
                time_to_empty_now: None,
                time_to_full_avg: None,
                time_to_full_now: None,
                typ: "Battery".to_string(),
                usb_type: "".to_string(),
                voltage_avg: None,
                voltage_boot: None,
                voltage_max: None,
                voltage_max_design: None,
                voltage_min: None,
                voltage_min_design: Some(10800000),
                voltage_now: Some(12229000),
                voltage_ocv: None,
            }
        )
    }
}
