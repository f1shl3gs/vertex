use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use configurable::Configurable;
use event::{Metric, tags};
use framework::config::serde_regex;
use serde::{Deserialize, Serialize};

use super::{Error, Paths, read_sys_file};

/// PowerSupply contains info from files in /sys/class/power_supply for
/// a single power supply
#[derive(Debug, Default, PartialEq)]
struct PowerSupply {
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
    capacity_level: Option<String>,
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
    charge_type: Option<String>,
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
    health: Option<String>,
    // /sys/class/power_supply/<name>/input_current_limit
    input_current_limit: Option<i64>,
    // /sys/class/power_supply/<name>/manufacturer
    manufacturer: Option<String>,
    // /sys/class/power_supply/<name>/model_name
    model_name: Option<String>,
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
    scope: Option<String>,
    // /sys/class/power_supply/<name>/serial_number
    serial_number: Option<String>,
    // /sys/class/power_supply/<name>/status
    status: Option<String>,
    // /sys/class/power_supply/<name>/technology
    technology: Option<String>,
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
    typ: Option<String>,
    // /sys/class/power_supply/<name>/usb_type
    usb_type: Option<String>,
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

fn read_optional(path: PathBuf) -> Result<Option<String>, Error> {
    match read_sys_file(path) {
        Ok(content) => Ok(Some(content)),
        Err(err) => {
            if err.kind() == ErrorKind::NotFound {
                return Ok(None);
            }

            Err(err.into())
        }
    }
}

fn read_optional_i64(path: PathBuf) -> Result<Option<i64>, Error> {
    match read_sys_file(path) {
        Ok(content) => Ok(content.parse().ok()),
        Err(err) => {
            if err.kind() == ErrorKind::NotFound {
                return Ok(None);
            }

            Err(err.into())
        }
    }
}

fn load_power_supply(root: &Path) -> Result<PowerSupply, Error> {
    let capacity_level = read_optional(root.join("capacity_level"))?;
    let charge_type = read_optional(root.join("charge_type"))?;
    let health = read_optional(root.join("health"))?;
    let manufacturer = read_optional(root.join("manufacturer"))?;
    let model_name = read_optional(root.join("model_name"))?;
    let scope = read_optional(root.join("scope"))?;
    let serial_number = read_optional(root.join("serial_number"))?;
    let status = read_optional(root.join("status"))?;
    let technology = read_optional(root.join("technology"))?;
    let typ = read_optional(root.join("type"))?;
    let usb_type = read_optional(root.join("usb_type"))?;

    let authentic = read_optional_i64(root.join("authentic"))?;
    let calibrate = read_optional_i64(root.join("calibrate"))?;
    let capacity = read_optional_i64(root.join("capacity"))?;
    let capacity_alert_max = read_optional_i64(root.join("capacity_alert_max"))?;
    let capacity_alert_min = read_optional_i64(root.join("capacity_alert_min"))?;
    let charge_avg = read_optional_i64(root.join("charge_avg"))?;
    let charge_control_limit = read_optional_i64(root.join("charge_control_limit"))?;
    let charge_control_limit_max = read_optional_i64(root.join("charge_control_limit_max"))?;
    let charge_counter = read_optional_i64(root.join("charge_counter"))?;
    let charge_empty = read_optional_i64(root.join("charge_empty"))?;
    let charge_empty_design = read_optional_i64(root.join("charge_empty_design"))?;
    let charge_full = read_optional_i64(root.join("charge_full"))?;
    let charge_full_design = read_optional_i64(root.join("charge_full_design"))?;
    let charge_now = read_optional_i64(root.join("charge_now"))?;
    let charge_term_current = read_optional_i64(root.join("charge_term_current"))?;
    let constant_charge_current = read_optional_i64(root.join("constant_charge_current"))?;
    let constant_charge_current_max = read_optional_i64(root.join("constant_charge_current_max"))?;
    let constant_charge_voltage = read_optional_i64(root.join("constant_charge_voltage"))?;
    let constant_charge_voltage_max = read_optional_i64(root.join("constant_charge_voltage_max"))?;
    let current_avg = read_optional_i64(root.join("current_avg"))?;
    let current_boot = read_optional_i64(root.join("current_boot"))?;
    let current_max = read_optional_i64(root.join("current_max"))?;
    let current_now = read_optional_i64(root.join("current_now"))?;
    let cycle_count = read_optional_i64(root.join("cycle_count"))?;
    let energy_avg = read_optional_i64(root.join("energy_avg"))?;
    let energy_empty = read_optional_i64(root.join("energy_empty"))?;
    let energy_empty_design = read_optional_i64(root.join("energy_empty_design"))?;
    let energy_full = read_optional_i64(root.join("energy_full"))?;
    let energy_full_design = read_optional_i64(root.join("energy_full_design"))?;
    let energy_now = read_optional_i64(root.join("energy_now"))?;
    let input_current_limit = read_optional_i64(root.join("input_current_limit"))?;
    let online = read_optional_i64(root.join("online"))?;
    let power_avg = read_optional_i64(root.join("power_avg"))?;
    let power_now = read_optional_i64(root.join("power_now"))?;
    let precharge_current = read_optional_i64(root.join("precharge_current"))?;
    let present = read_optional_i64(root.join("present"))?;
    let temp = read_optional_i64(root.join("temp"))?;
    let temp_alert_max = read_optional_i64(root.join("temp_alert_max"))?;
    let temp_alert_min = read_optional_i64(root.join("temp_alert_min"))?;
    let temp_ambient = read_optional_i64(root.join("temp_ambient"))?;
    let temp_ambient_max = read_optional_i64(root.join("temp_ambient_max"))?;
    let temp_ambient_min = read_optional_i64(root.join("temp_ambient_min"))?;
    let temp_max = read_optional_i64(root.join("temp_max"))?;
    let temp_min = read_optional_i64(root.join("temp_min"))?;
    let time_to_empty_avg = read_optional_i64(root.join("time_to_empty_avg"))?;
    let time_to_empty_now = read_optional_i64(root.join("time_to_empty_now"))?;
    let time_to_full_avg = read_optional_i64(root.join("time_to_full_avg"))?;
    let time_to_full_now = read_optional_i64(root.join("time_to_full_now"))?;
    let voltage_avg = read_optional_i64(root.join("voltage_avg"))?;
    let voltage_boot = read_optional_i64(root.join("voltage_boot"))?;
    let voltage_max = read_optional_i64(root.join("voltage_max"))?;
    let voltage_max_design = read_optional_i64(root.join("voltage_max_design"))?;
    let voltage_min = read_optional_i64(root.join("voltage_min"))?;
    let voltage_min_design = read_optional_i64(root.join("voltage_min_design"))?;
    let voltage_now = read_optional_i64(root.join("voltage_now"))?;
    let voltage_ocv = read_optional_i64(root.join("voltage_ocv"))?;

    Ok(PowerSupply {
        authentic,
        calibrate,
        capacity,
        capacity_alert_max,
        capacity_alert_min,
        capacity_level,
        charge_avg,
        charge_control_limit,
        charge_control_limit_max,
        charge_counter,
        charge_empty,
        charge_empty_design,
        charge_full,
        charge_full_design,
        charge_now,
        charge_term_current,
        charge_type,
        constant_charge_current,
        constant_charge_current_max,
        constant_charge_voltage,
        constant_charge_voltage_max,
        current_avg,
        current_boot,
        current_max,
        current_now,
        cycle_count,
        energy_avg,
        energy_empty,
        energy_empty_design,
        energy_full,
        energy_full_design,
        energy_now,
        health,
        input_current_limit,
        manufacturer,
        model_name,
        online,
        power_avg,
        power_now,
        precharge_current,
        present,
        scope,
        serial_number,
        status,
        technology,
        temp,
        temp_alert_max,
        temp_alert_min,
        temp_ambient,
        temp_ambient_max,
        temp_ambient_min,
        temp_max,
        temp_min,
        time_to_empty_avg,
        time_to_empty_now,
        time_to_full_avg,
        time_to_full_now,
        typ,
        usb_type,
        voltage_avg,
        voltage_boot,
        voltage_max,
        voltage_max_design,
        voltage_min,
        voltage_min_design,
        voltage_now,
        voltage_ocv,
    })
}

fn default_ignored() -> regex::Regex {
    regex::Regex::new("^$").unwrap()
}

#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
pub struct Config {
    /// Regex of power supplies to ignore for powersupplyclass collector.
    #[serde(default = "default_ignored", with = "serde_regex")]
    ignored: regex::Regex,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ignored: regex::Regex::new("^$").unwrap(),
        }
    }
}

pub async fn collect(conf: Config, paths: Paths) -> Result<Vec<Metric>, Error> {
    let dirs = std::fs::read_dir(paths.sys().join("class/power_supply"))?;

    let mut metrics = Vec::new();
    for entry in dirs.flatten() {
        let path = entry.path();
        let name = path.file_name().unwrap().to_str().unwrap();
        if conf.ignored.is_match(name) {
            continue;
        }

        let supply = load_power_supply(&path)?;

        for (stat, value) in [
            ("authentic", supply.authentic),
            ("calibrate", supply.calibrate),
            ("capacity", supply.capacity),
            ("capacity_alert_max", supply.capacity_alert_max),
            ("capacity_alert_min", supply.capacity_alert_min),
            ("cyclecount", supply.cycle_count),
            ("online", supply.online),
            ("present", supply.present),
            ("time_to_empty_seconds", supply.time_to_empty_now),
            ("time_to_full_seconds", supply.time_to_full_now),
        ] {
            let Some(value) = value else { continue };

            metrics.push(Metric::gauge_with_tags(
                format!("node_power_supply_{stat}"),
                format!("{stat} value of /sys/class/power_supply/<power_supply>"),
                value,
                tags! {
                    "power_supply" => name
                },
            ))
        }

        for (stat, value) in [
            ("current_boot", supply.current_boot),
            ("current_max", supply.current_max),
            ("current_ampere", supply.current_now),
            ("energy_empty", supply.energy_empty),
            ("energy_empty_design", supply.energy_empty_design),
            ("energy_full", supply.energy_full),
            ("energy_full_design", supply.energy_full_design),
            ("energy_watthour", supply.energy_now),
            ("voltage_boot", supply.voltage_boot),
            ("voltage_max", supply.voltage_max),
            ("voltage_max_design", supply.voltage_max_design),
            ("voltage_min", supply.voltage_min),
            ("voltage_min_design", supply.voltage_min_design),
            ("voltage_volt", supply.voltage_now),
            ("voltage_ocv", supply.voltage_ocv),
            ("charge_control_limit", supply.charge_control_limit),
            ("charge_control_limit_max", supply.charge_control_limit_max),
            ("charge_counter", supply.charge_counter),
            ("charge_empty", supply.charge_empty),
            ("charge_empty_design", supply.charge_empty_design),
            ("charge_full", supply.charge_full),
            ("charge_full_design", supply.charge_full_design),
            ("charge_ampere", supply.charge_now),
            ("charge_term_current", supply.charge_term_current),
            ("constant_charge_current", supply.constant_charge_current),
            (
                "constant_charge_current_max",
                supply.constant_charge_current_max,
            ),
            ("constant_charge_voltage", supply.constant_charge_voltage),
            (
                "constant_charge_voltage_max",
                supply.constant_charge_voltage_max,
            ),
            ("precharge_current", supply.precharge_current),
            ("input_current_limit", supply.input_current_limit),
            ("power_watt", supply.power_now),
        ] {
            let Some(value) = value else { continue };

            metrics.push(Metric::gauge_with_tags(
                format!("node_power_supply_{stat}"),
                format!("{stat} value of /sys/class/power_supply/<power_supply>"),
                value as f64 / 1e6,
                tags! {
                    "power_supply" => name
                },
            ));
        }

        for (stat, value) in [
            ("temp_celsius", supply.temp),
            ("temp_alert_max_celsius", supply.temp_alert_max),
            ("temp_alert_min_celsius", supply.temp_alert_min),
            ("temp_ambient_celsius", supply.temp_ambient),
            ("temp_ambient_max_celsius", supply.temp_ambient_max),
            ("temp_ambient_min_celsius", supply.temp_ambient_min),
            ("temp_max_celsius", supply.temp_max),
            ("temp_min_celsius", supply.temp_min),
        ] {
            let Some(value) = value else { continue };

            metrics.push(Metric::gauge_with_tags(
                format!("node_power_supply_{stat}"),
                format!("{stat} value of /sys/class/power_supply/<power_supply>"),
                value as f64 / 10.0,
                tags! {
                    "power_supply" => name
                },
            ))
        }

        let mut tags = tags!("power_supply" => name);
        if let Some(capacity_level) = supply.capacity_level {
            tags.insert("capacity_level", capacity_level);
        }
        if let Some(charge_type) = supply.charge_type {
            tags.insert("charge_type", charge_type);
        }
        if let Some(health) = supply.health {
            tags.insert("health", health);
        }
        if let Some(manufacturer) = supply.manufacturer {
            tags.insert("manufacturer", manufacturer);
        }
        if let Some(model_name) = supply.model_name {
            tags.insert("model_name", model_name);
        }
        if let Some(serial_number) = supply.serial_number {
            tags.insert("serial_number", serial_number);
        }
        if let Some(status) = supply.status {
            tags.insert("status", status);
        }
        if let Some(technology) = supply.technology {
            tags.insert("technology", technology);
        }
        if let Some(typ) = supply.typ {
            tags.insert("type", typ);
        }
        if let Some(usb_type) = supply.usb_type {
            tags.insert("usb_type", usb_type);
        }
        if let Some(scope) = supply.scope {
            tags.insert("scope", scope);
        }

        metrics.push(Metric::gauge_with_tags(
            "node_power_supply_info",
            "info of /sys/class/power_supply/<power_supply>.",
            1,
            tags,
        ))
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn smoke() {
        let conf = Config::default();
        let paths = Paths::test();
        let metrics = collect(conf, paths).await.unwrap();
        assert_ne!(metrics.len(), 0);
    }

    #[test]
    fn parse() {
        let path = Path::new("tests/node/fixtures/sys/class/power_supply/AC");
        let supply = load_power_supply(path).unwrap();
        assert_eq!(
            supply,
            PowerSupply {
                authentic: None,
                calibrate: None,
                capacity: None,
                capacity_alert_max: None,
                capacity_alert_min: None,
                capacity_level: None,
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
                charge_type: None,
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
                health: None,
                input_current_limit: None,
                manufacturer: None,
                typ: Some("Mains".to_string()),
                usb_type: None,
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
                scope: None,
                serial_number: None,
                status: None,
                technology: None,
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
                model_name: None,
                time_to_full_now: None,
                voltage_ocv: None,
            }
        );

        let path = Path::new("tests/node/fixtures/sys/class/power_supply/BAT0");
        let supply = load_power_supply(path).unwrap();
        assert_eq!(
            supply,
            PowerSupply {
                authentic: None,
                calibrate: None,
                capacity: Some(98),
                capacity_alert_max: None,
                capacity_alert_min: None,
                capacity_level: Some("Normal".to_string()),
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
                charge_type: None,
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
                health: None,
                input_current_limit: None,
                manufacturer: Some("LGC".to_string()),
                model_name: Some("LNV-45N1".to_string()),
                online: None,
                power_avg: None,
                power_now: Some(4830000),
                precharge_current: None,
                present: Some(1),
                scope: None,
                serial_number: Some("38109".to_string()),
                status: Some("Discharging".to_string()),
                technology: Some("Li-ion".to_string()),
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
                typ: Some("Battery".to_string()),
                usb_type: None,
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
