use serde::Deserialize;

pub use power::Power;
pub use thermal::Thermal;

use super::{Link, Status};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Chassis {
    pub id: String,
    pub name: String,
    pub chassis_type: Option<String>,
    pub asset_tag: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    #[serde(rename = "SKU")]
    pub sku: Option<String>,
    pub serial_number: Option<String>,
    pub part_number: Option<String>,

    pub power_state: Option<String>,

    // `Thermal` is deprecated, `ThermalSubsystem` is recommend
    pub thermal: Option<Link>,
    // pub thermal_subsystem: Option<Link>,
    pub power: Option<Link>,

    pub status: Option<Status>,
}

mod thermal {
    use serde::Deserialize;

    use super::Status;

    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub struct Fan {
        /// The unique identifier for the member within an array
        pub member_id: String,
        /// Name of the fan
        pub name: String,
        /// The manufacturer of this fan.
        pub manufacturer: Option<String>,
        /// An indication of whether this device can be inserted or removed while the equipment is in operation.
        pub hot_pluggable: Option<bool>,

        /// The status and health of a resource and its children
        pub status: Status,

        // The fan speed
        pub reading: Option<i64>,
        /// The units in which the fan reading and thresholds are measured
        ///
        /// | Value | Description |
        /// | ---- | ------ |
        /// | Percent | The fan reading and thresholds are measured as a percentage. |
        /// | RPM | The fan reading and thresholds are measured in revolutions per minute. |
        pub reading_units: Option<String>,
        /// The value at which the reading is below normal range but not yet fatal.
        pub lower_threshold_critical: Option<i64>,
        /// The value at which the reading is below normal range and fatal
        pub lower_threshold_fatal: Option<i64>,
        /// The value at which the reading is below normal range.
        pub lower_threshold_non_critical: Option<i64>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub struct Temperature {
        pub member_id: String,
        pub name: String,
        pub status: Status,

        // The temperature (°C).
        // None if this is disabled
        pub reading_celsius: Option<i64>,
        // The value at which the reading is above normal range.
        pub upper_threshold_non_critical: Option<i64>,
        // The value at which the reading is above normal range but not yet fatal.
        pub upper_threshold_critical: Option<i64>,
        // The value at which the reading is above normal range and fatal
        pub upper_threshold_fatal: Option<i64>,
        // Minimum value for this sensor
        pub min_reading_range_temp: Option<i64>,
        // Maximum value for this sensor
        pub max_reading_range_temp: Option<i64>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub struct Thermal {
        pub id: String,
        pub name: String,

        #[serde(default)]
        pub fans: Vec<Fan>,
        #[serde(default)]
        pub temperatures: Vec<Temperature>,
    }
}

pub mod thermal_subsystem {
    use serde::Deserialize;

    use super::{Link, Status};

    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub struct SpeedPercent {
        #[serde(default)]
        pub reading: i64,
        #[serde(default, rename = "SpeedRPM")]
        pub speed_rpm: i64,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub struct Fan {
        pub id: String,
        pub name: String,

        pub model: Option<String>,
        pub manufacturer: Option<String>,
        pub part_number: Option<String>,
        pub spare_part_number: Option<String>,
        pub speed_percent: SpeedPercent,

        pub status: Status,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub struct ThermalSubsystem {
        pub id: String,
        pub name: String,

        pub fans: Option<Link>,
        // todo
        // pub thermal_metrics: Option<Link>,
        pub status: Status,
    }
}

mod power {
    use serde::Deserialize;

    use super::Status;

    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub struct Voltage {
        pub member_id: Option<String>,
        pub name: String,
        pub status: Status,

        /// The reading of the voltage sensor.
        pub reading_volts: Option<f64>,
        /// The value at which the reading is above normal range.
        pub upper_threshold_non_critical: Option<f64>,
        /// The value at which the reading is above normal range but not yet fatal.
        pub upper_threshold_critical: Option<f64>,
        /// The value at which the reading is above normal range and fatal.
        pub upper_threshold_fatal: Option<f64>,
        /// The value at which the reading is below normal range.
        pub lower_threshold_non_critical: Option<f64>,
        /// The value at which the reading is below normal range but not yet fatal.
        pub lower_threshold_critical: Option<f64>,
        /// The value at which the reading is below normal range and fatal.
        pub lower_threshold_fatal: Option<f64>,
    }

    /// The power limit status and configuration information for the chassis.
    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub struct PowerLimit {
        /// The power limit, in watt units. If `null`, power capping is disabled.
        pub limit_in_watts: Option<i64>,
        /// The action that is taken if the power cannot be maintained below the `LimitInWatts`.
        ///
        /// | Value | Description |
        /// | ---- | --------- |
        /// | HardPowerOff | Turn the power off immediately when the limit is exceeded. |
        /// | LogEventOnly | Log an event when the limit is exceeded, but take no further action. |
        /// | NoAction | Take no action when the limit is exceeded. |
        /// | Oem | Take an OEM-defined action. |
        pub limit_exception: String,
        /// The time required for the limiting process to reduce power consumption to below the limit.
        pub correction_in_ms: Option<i64>,
    }

    /// The power metrics for a resource.
    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub struct PowerMetrics {
        /// The time interval, or window, over which the power metrics are measured.
        pub interval_in_min: Option<i64>,
        /// The lowest power consumption level, in watt units, over the measurement window that occurred within the last `IntervalInMin` minutes.
        pub min_consumed_watts: Option<i64>,
        /// The highest power consumption level, in watt units, that has occurred over the measurement window within the last `IntervalInMin` minutes.
        pub max_consumed_watts: Option<i64>,
        /// The average power level over the measurement window over the last `IntervalInMin` minutes.
        pub average_consumed_watts: Option<i64>,
    }

    /// The set of power control functions, including power reading and limiting.
    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub struct PowerControl {
        pub name: String,

        /// The actual power that the chassis consumes, in watt units.
        pub power_consumed_watts: Option<i64>,
        /// The potential power, in watt units, that the chassis requests, which might be higher than the current level being consumed because the requested power includes a budget that the chassis wants for future use.
        pub power_requested_watts: Option<i64>,
        /// The amount of reserve power capacity, in watt units, that remains. This value is the PowerCapacityWatts value minus the `PowerAllocatedWatts` value.
        pub power_available_watts: Option<i64>,
        /// The total amount of power that can be allocated to the chassis. This value can be either the power supply capacity or the power budget that an upstream chassis assigns to this chassis.
        pub power_capacity_watts: Option<i64>,
        /// The total amount of power that has been allocated or budgeted to chassis.
        pub power_allocated_watts: Option<i64>,

        pub power_metrics: Option<PowerMetrics>,

        pub power_limit: Option<PowerLimit>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub struct PowerSupply {
        pub member_id: Option<String>,
        pub name: String,
        pub model: Option<String>,
        pub manufacturer: Option<String>,
        pub firmware_version: Option<String>,
        pub serial_number: Option<String>,
        pub part_number: Option<String>,
        pub spare_part_number: Option<String>,

        // NOTE: add redundancy array
        pub status: Status,

        // The maximum capacity of this power supply.
        pub efficiency_percent: Option<f64>,
        pub power_capacity_watts: Option<f64>,
        pub power_input_watts: Option<f64>,
        pub power_output_watts: Option<f64>,

        pub hot_pluggable: Option<bool>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub struct Power {
        pub id: String,
        pub name: String,

        #[serde(default)]
        pub power_supplies: Vec<PowerSupply>,
        #[serde(default)]
        pub power_control: Vec<PowerControl>,
        #[serde(default)]
        pub voltages: Vec<Voltage>,
    }
}

#[cfg(test)]
mod tests {
    use super::power::Power;
    use super::*;

    #[test]
    fn dell_r720_power() {
        let data = std::fs::read("tests/redfish/dell_r720_power.json").unwrap();

        let power = serde_json::from_slice::<Power>(data.as_slice()).unwrap();

        assert_eq!(power.id, "Power");
        assert_eq!(power.name, "Power");

        assert_eq!(power.power_control.len(), 1);
    }
}
