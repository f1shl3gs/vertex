use std::path::{Path, PathBuf};

use configurable::Configurable;
use event::tags::Key;
use event::{Metric, tags};
use framework::config::serde_regex;
use serde::{Deserialize, Serialize};

use super::{Error, read_string};

const POWER_SUPPLY_KEY: Key = Key::from_static("power_supply");

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
async fn power_supply_class(root: &Path) -> Result<Vec<PowerSupply>, Error> {
    let dirs = std::fs::read_dir(root.join("class/power_supply"))?;

    let mut pss = vec![];
    for entry in dirs.flatten() {
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

    let dirs = std::fs::read_dir(path)?;
    for entry in dirs.flatten() {
        // TODO: node_exporter will skip none regular entry
        let meta = entry.metadata()?;
        if !meta.is_file() {
            continue;
        }

        match entry.file_name().to_string_lossy().as_ref() {
            "authentic" => {
                ps.authentic = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "calibrate" => {
                ps.calibrate = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "capacity" => {
                ps.capacity = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "capacity_alert_max" => {
                ps.capacity_alert_max = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "capacity_alert_min" => {
                ps.capacity_alert_min = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "capacity_level" => {
                ps.capacity_level = match read_string(entry.path()) {
                    Ok(content) => content,
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "charge_avg" => {
                ps.charge_avg = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "charge_control_limit" => {
                ps.charge_control_limit = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "charge_control_limit_max" => {
                ps.charge_control_limit_max = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "charge_counter" => {
                ps.charge_counter = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "charge_empty" => {
                ps.charge_empty = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "charge_empty_design" => {
                ps.charge_empty_design = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "charge_full" => {
                ps.charge_full = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "charge_full_design" => {
                ps.charge_full_design = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "charge_now" => {
                ps.charge_now = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "charge_term_current" => {
                ps.charge_term_current = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "charge_type" => {
                ps.charge_type = match read_string(entry.path()) {
                    Ok(content) => content,
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "constant_charge_current" => {
                ps.constant_charge_current = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "constant_charge_current_max" => {
                ps.constant_charge_current_max = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "constant_charge_voltage" => {
                ps.constant_charge_voltage = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "constant_charge_voltage_max" => {
                ps.constant_charge_voltage_max = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "current_avg" => {
                ps.current_avg = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "current_boot" => {
                ps.current_boot = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "current_max" => {
                ps.current_max = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "current_now" => {
                ps.current_now = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "cycle_count" => {
                ps.cycle_count = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "energy_avg" => {
                ps.energy_avg = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "energy_empty" => {
                ps.energy_empty = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "energy_empty_design" => {
                ps.energy_empty_design = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "energy_full" => {
                ps.energy_full = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "energy_full_design" => {
                ps.energy_full_design = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "energy_now" => {
                ps.energy_now = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "health" => {
                ps.health = match read_string(entry.path()) {
                    Ok(content) => content,
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "input_current_limit" => {
                ps.input_current_limit = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "manufacturer" => {
                ps.manufacturer = match read_string(entry.path()) {
                    Ok(content) => content,
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "model_name" => {
                ps.model_name = match read_string(entry.path()) {
                    Ok(content) => content,
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "online" => {
                ps.online = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "power_avg" => {
                ps.power_avg = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "power_now" => {
                ps.power_now = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "precharge_current" => {
                ps.precharge_current = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "present" => {
                ps.present = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "scope" => {
                ps.scope = match read_string(entry.path()) {
                    Ok(content) => content,
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "serial_number" => {
                ps.serial_number = match read_string(entry.path()) {
                    Ok(content) => content,
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "status" => {
                ps.status = match read_string(entry.path()) {
                    Ok(content) => content,
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "technology" => {
                ps.technology = match read_string(entry.path()) {
                    Ok(content) => content,
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "temp" => {
                ps.temp = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "temp_alert_max" => {
                ps.temp_alert_max = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "temp_alert_min" => {
                ps.temp_alert_min = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "temp_ambient" => {
                ps.temp_ambient = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "temp_ambient_max" => {
                ps.temp_ambient_max = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "temp_ambient_min" => {
                ps.temp_ambient_min = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "temp_max" => {
                ps.temp_max = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "temp_min" => {
                ps.temp_min = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "time_to_empty_avg" => {
                ps.time_to_empty_avg = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "time_to_empty_now" => {
                ps.time_to_empty_now = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "time_to_full_avg" => {
                ps.time_to_full_avg = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "time_to_full_now" => {
                ps.time_to_full_now = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "type" => {
                ps.typ = match read_string(entry.path()) {
                    Ok(content) => content,
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "usb_type" => {
                ps.usb_type = match read_string(entry.path()) {
                    Ok(content) => content,
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "voltage_avg" => {
                ps.voltage_avg = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "voltage_boot" => {
                ps.voltage_boot = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "voltage_max" => {
                ps.voltage_max = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "voltage_max_design" => {
                ps.voltage_max_design = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "voltage_min" => {
                ps.voltage_min = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "voltage_min_design" => {
                ps.voltage_min_design = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "voltage_now" => {
                ps.voltage_now = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            "voltage_ocv" => {
                ps.voltage_ocv = match read_string(entry.path()) {
                    Ok(content) => content.parse().ok(),
                    Err(err) => {
                        if err.kind() == std::io::ErrorKind::NotFound {
                            continue;
                        }

                        return Err(err.into());
                    }
                }
            }
            _ => continue,
        }
    }

    Ok(ps)
}

#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(with = "serde_regex")]
    pub ignored: regex::Regex,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ignored: regex::Regex::new("^$").unwrap(),
        }
    }
}

macro_rules! power_supply_metric {
    ($vec: expr, $power_supply: expr, $name: expr, $value: expr) => {
        if let Some(v) = $value {
            $vec.push(Metric::gauge_with_tags(
                concat!("node_power_supply_", $name),
                concat!($name, " value of /sys/class/power_supply/<power_supply>"),
                v,
                tags! {
                    POWER_SUPPLY_KEY => $power_supply.clone()
                },
            ))
        }
    };
}

macro_rules! power_supply_metric_divide_e6 {
    ($vec: expr, $power_supply: expr, $name: tt, $value: expr) => {
        if let Some(v) = $value {
            $vec.push(Metric::gauge_with_tags(
                concat!("node_power_supply_", $name),
                concat!("value of /sys/class/power_supply/<power_supply>/", $name),
                v as f64 / 1e6,
                tags! {
                    POWER_SUPPLY_KEY => $power_supply.clone()
                },
            ))
        }
    };
}

macro_rules! power_supply_metric_divide_10 {
    ($vec: expr, $power_supply: expr, $name: tt, $value: expr) => {
        if let Some(v) = $value {
            $vec.push(Metric::gauge_with_tags(
                concat!("node_power_supply_", $name),
                concat!("value of /sys/class/power_supply/<power_supply>/", $name),
                v as f64 / 10.0,
                tags! {
                    POWER_SUPPLY_KEY => $power_supply.clone()
                },
            ))
        }
    };
}

pub async fn gather(conf: Config, sys_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let psc = power_supply_class(&sys_path).await?;
    let mut metrics = vec![];
    for ps in psc {
        if conf.ignored.is_match(&ps.name) {
            continue;
        }

        power_supply_metric!(metrics, ps.name, "authentic", ps.authentic);
        power_supply_metric!(metrics, ps.name, "calibrate", ps.calibrate);
        power_supply_metric!(metrics, ps.name, "capacity", ps.capacity);
        power_supply_metric!(
            metrics,
            ps.name,
            "capacity_alert_max",
            ps.capacity_alert_max
        );
        power_supply_metric!(
            metrics,
            ps.name,
            "capacity_alert_min",
            ps.capacity_alert_min
        );
        power_supply_metric!(metrics, ps.name, "cyclecount", ps.cycle_count);
        power_supply_metric!(metrics, ps.name, "online", ps.online);
        power_supply_metric!(metrics, ps.name, "present", ps.present);
        power_supply_metric!(
            metrics,
            ps.name,
            "time_to_empty_seconds",
            ps.time_to_empty_now
        );
        power_supply_metric!(
            metrics,
            ps.name,
            "time_to_full_seconds",
            ps.time_to_full_now
        );

        power_supply_metric_divide_e6!(metrics, ps.name, "current_boot", ps.current_boot);
        power_supply_metric_divide_e6!(metrics, ps.name, "current_max", ps.current_max);
        power_supply_metric_divide_e6!(metrics, ps.name, "current_ampere", ps.current_now);
        power_supply_metric_divide_e6!(metrics, ps.name, "energy_empty", ps.energy_empty);
        power_supply_metric_divide_e6!(
            metrics,
            ps.name,
            "energy_empty_design",
            ps.energy_empty_design
        );
        power_supply_metric_divide_e6!(metrics, ps.name, "energy_full", ps.energy_full);
        power_supply_metric_divide_e6!(
            metrics,
            ps.name,
            "energy_full_design",
            ps.energy_full_design
        );
        power_supply_metric_divide_e6!(metrics, ps.name, "energy_watthour", ps.energy_now);
        power_supply_metric_divide_e6!(metrics, ps.name, "voltage_boot", ps.voltage_boot);
        power_supply_metric_divide_e6!(metrics, ps.name, "voltage_max", ps.voltage_max);
        power_supply_metric_divide_e6!(
            metrics,
            ps.name,
            "voltage_max_design",
            ps.voltage_max_design
        );
        power_supply_metric_divide_e6!(metrics, ps.name, "voltage_min", ps.voltage_min);
        power_supply_metric_divide_e6!(
            metrics,
            ps.name,
            "voltage_min_design",
            ps.voltage_min_design
        );
        power_supply_metric_divide_e6!(metrics, ps.name, "voltage_volt", ps.voltage_now);
        power_supply_metric_divide_e6!(metrics, ps.name, "voltage_ocv", ps.voltage_ocv);
        power_supply_metric_divide_e6!(
            metrics,
            ps.name,
            "charge_control_limit",
            ps.charge_control_limit
        );
        power_supply_metric_divide_e6!(
            metrics,
            ps.name,
            "charge_control_limit_max",
            ps.charge_control_limit_max
        );
        power_supply_metric_divide_e6!(metrics, ps.name, "charge_counter", ps.charge_counter);
        power_supply_metric_divide_e6!(metrics, ps.name, "charge_empty", ps.charge_empty);
        power_supply_metric_divide_e6!(
            metrics,
            ps.name,
            "charge_empty_design",
            ps.charge_empty_design
        );
        power_supply_metric_divide_e6!(metrics, ps.name, "charge_full", ps.charge_full);
        power_supply_metric_divide_e6!(
            metrics,
            ps.name,
            "charge_full_design",
            ps.charge_full_design
        );
        power_supply_metric_divide_e6!(metrics, ps.name, "charge_ampere", ps.charge_now);
        power_supply_metric_divide_e6!(
            metrics,
            ps.name,
            "charge_term_current",
            ps.charge_term_current
        );
        power_supply_metric_divide_e6!(
            metrics,
            ps.name,
            "constant_charge_current",
            ps.constant_charge_current
        );
        power_supply_metric_divide_e6!(
            metrics,
            ps.name,
            "constant_charge_current_max",
            ps.constant_charge_current_max
        );
        power_supply_metric_divide_e6!(
            metrics,
            ps.name,
            "constant_charge_voltage",
            ps.constant_charge_voltage
        );
        power_supply_metric_divide_e6!(
            metrics,
            ps.name,
            "constant_charge_voltage_max",
            ps.constant_charge_voltage_max
        );
        power_supply_metric_divide_e6!(metrics, ps.name, "precharge_current", ps.precharge_current);
        power_supply_metric_divide_e6!(
            metrics,
            ps.name,
            "input_current_limit",
            ps.input_current_limit
        );
        power_supply_metric_divide_e6!(metrics, ps.name, "power_watt", ps.power_now);

        power_supply_metric_divide_10!(metrics, ps.name, "temp_celsius", ps.temp);
        power_supply_metric_divide_10!(
            metrics,
            ps.name,
            "temp_alert_max_celsius",
            ps.temp_alert_max
        );
        power_supply_metric_divide_10!(
            metrics,
            ps.name,
            "temp_alert_min_celsius",
            ps.temp_alert_min
        );
        power_supply_metric_divide_10!(metrics, ps.name, "temp_ambient_celsius", ps.temp_ambient);
        power_supply_metric_divide_10!(
            metrics,
            ps.name,
            "temp_ambient_max_celsius",
            ps.temp_ambient_max
        );
        power_supply_metric_divide_10!(
            metrics,
            ps.name,
            "temp_ambient_min_celsius",
            ps.temp_ambient_min
        );
        power_supply_metric_divide_10!(metrics, ps.name, "temp_max_celsius", ps.temp_max);
        power_supply_metric_divide_10!(metrics, ps.name, "temp_min_celsius", ps.temp_min);

        let mut tags = tags!(
            "power_supply" => ps.name
        );
        if !ps.capacity_level.is_empty() {
            tags.insert("capacity_level", ps.capacity_level);
        }

        if !ps.charge_type.is_empty() {
            tags.insert("charge_type", ps.charge_type);
        }

        if !ps.health.is_empty() {
            tags.insert("health", ps.health);
        }

        if !ps.manufacturer.is_empty() {
            tags.insert("manufacturer", ps.manufacturer);
        }

        if !ps.model_name.is_empty() {
            tags.insert("model_name", ps.model_name);
        }

        if !ps.serial_number.is_empty() {
            tags.insert("serial_number", ps.serial_number);
        }

        if !ps.status.is_empty() {
            tags.insert("status", ps.status);
        }

        if !ps.technology.is_empty() {
            tags.insert("technology", ps.technology);
        }

        if !ps.typ.is_empty() {
            tags.insert("type", ps.typ);
        }

        if !ps.usb_type.is_empty() {
            tags.insert("usb_type", ps.usb_type);
        }

        if !ps.scope.is_empty() {
            tags.insert("scope", ps.scope);
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
    async fn test_power_supply_class() {
        let root = PathBuf::from("tests/node/sys");
        let mut pss = power_supply_class(&root).await.unwrap();

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
