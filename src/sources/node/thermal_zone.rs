/// Exposes thermal zone & cooling device statistics from /sys/class/thermal
pub fn thermal_zone() {}


/// ClassThermalStats contains info from files in /sys/class/thermal_zone<zone>
/// for a single <zone>
///
/// https://www.kernel.org/doc/Documentation/thermal/sysfs-api.txt
struct ClassThermalZoneStats {
    // The name of the zone from the directory structure
    name: String,

    // The type of thermal zone
    typ: String,

    // Temperature in millidegree Celsius
    temp: i64,

    // One of the various thermal governors used for a particular zone
    policy: String,

    // Optional: One of the predefined values in [enabled, disabled]
    mode: Option<bool>,

    // Optional: millidegrees Celsius. (0 for disabled, > 1000 for enabled+value)
    passive: Option<u64>,
}