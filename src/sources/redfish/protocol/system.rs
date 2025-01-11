use serde::Deserialize;

use super::{Link, Status};

/// The collection of `ComputerSystem` resource instances.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct System {
    pub id: String,
    pub name: String,
    /// The type of computer system that this resource represents.
    #[serde(default)]
    pub system_type: String,
    /// The manufacturer or OEM of this system.
    #[serde(default)]
    pub manufacturer: String,
    /// The product name for this system, without the manufacturer name.
    #[serde(default)]
    pub model: String,
    /// The manufacturer SKU for this system.
    #[serde(default, rename = "SKU")]
    pub sku: String,
    /// The serial number for this system.
    #[serde(default)]
    pub serial_number: String,
    /// The part number for this system.
    #[serde(default)]
    pub part_number: String,
    /// The UUID for this system.
    #[serde(default, rename = "UUID")]
    pub uuid: String,
    /// The DNS host name, without any domain information.
    #[serde(default)]
    pub host_name: String,
    /// The user-definable tag that can track this computer system for inventory or other client purposes.
    #[serde(default)]
    pub asset_tag: String,
    /// The status and health of a resource and its children.
    pub status: Option<Status>,

    /// The current power state of the system.
    ///
    /// - Off         -- The resource is powered off. The components within the resource might continue to have AUX power
    /// - On          -- The resource is powered on
    /// - Paused      -- The resource is paused
    /// - PoweringOff -- A temporary state between on and off. The components within the resource can take time to process the power off action.
    /// - PoweringOn  -- A temporary state between off and on. The components within the resource can take time to process the power on action.
    pub power_state: Option<String>,
    #[serde(default)]
    pub bios_version: String,

    /// The collection of `Memory` resource instances.
    #[serde(default)]
    pub memory: Option<Link>,
    #[serde(default)]
    pub network_interfaces: Option<Link>,
    #[serde(default)]
    pub storage: Option<Link>,
    #[serde(default)]
    pub simple_storage: Option<Link>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MemoryLocation {
    // The socket number to which the memory device is connected
    pub socket: usize,
    // The memory controller number to which the memory device is connected.
    pub memory_controller: usize,
    // The channel number to which the memory device is connected.
    pub channel: usize,
    // The slot number to which the memory device is connected.
    pub slot: usize,
}

/// `AlarmTrips` shall contain properties describing the types of alarms that have
/// been raised by the memory. These alarms shall be reset when the system
/// resets. Note that if they are re-discovered they can be reasserted.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AlarmTrips {
    /// `AddressParityError` shall be true if an Address Parity Error was detected
    /// which could not be corrected by retry.
    pub address_parity_error: bool,
    /// `CorrectableECCError` shall be true if the correctable error threshold
    /// crossing alarm trip was detected.
    pub correctable_ecc_error: bool,
    /// `SpareBlock` shall be true if the spare block capacity crossing alarm trip
    /// was detected.
    pub spare_block: bool,
    /// `Temperature` shall be true if a temperature threshold alarm trip was detected.
    pub temperature: bool,
    /// `UncorrectableECCError` shall be true if the uncorrectable error threshold
    /// alarm trip was detected.
    pub uncorrectable_ecc_error: bool,
}

/// `HealthData` shall contain properties which describe the HealthData
/// metrics for the current resource.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct HealthData {
    /// AlarmTrips shall contain properties describe the types of alarms that
    /// have been raised by the memory.
    pub alarm_trips: AlarmTrips,
    /// `DataLossDetected` shall be data loss detection status, with true
    /// indicating data loss detected.
    pub data_loss_detected: bool,
    /// `LastShutdownSuccess` shall be the status of the last shutdown, with
    /// true indicating success
    pub last_shutdown_success: bool,
    /// `PerformanceDegraded` shall be performance degraded mode status, with
    /// true indicating performance degraded
    pub performance_degraded: bool,
    /// `PredictedMediaLifeLeftPercent` shall contain an indicator
    /// of the percentage of life remaining in the media.
    pub predicted_media_life_left_percent: f32,
    /// `RemainingSpareBlockPercentage` shall be the remaining spare blocks in percentage.
    pub remaining_spare_block_percentage: f32,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct MemoryMetrics {
    pub id: String,
    pub name: String,

    pub health_data: HealthData,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Memory {
    pub id: String,
    pub name: String,
    /// Number of ranks available in the memory device
    #[serde(default)]
    pub rank_count: u64,

    /// Memory capacity in mebibytes (MiB)
    #[serde(default, rename = "CapacityMiB")]
    pub capacity_mib: usize,
    #[serde(default)]
    pub data_width_bits: usize,
    #[serde(default)]
    pub bus_width_bits: usize,
    #[serde(default)]
    pub error_correction: String,

    // The type of memory device.
    //
    // - Cache
    // - DRAM
    // - IntelOptane
    // - NVDIMM_F
    // - NVDIMM_N
    // - NVDIMM_P
    #[serde(default)]
    pub memory_type: String,
    #[serde(default)]
    pub memory_device_type: String,
    #[serde(default)]
    pub base_module_type: String,

    // OperatingSpeedMhz shall be the operating speed of Memory in MHz or
    // MT/s (mega-transfers per second) as reported by the memory device.
    #[serde(default)]
    pub operating_speed_mhz: i64,
    pub memory_location: Option<MemoryLocation>,

    pub metrics: Option<Link>,

    pub status: Status,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NetworkInterface {
    pub id: String,
    pub name: String,

    pub status: Status,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Device {
    pub name: String,
    pub status: Status,

    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub capacity_bytes: Option<usize>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SimpleStorage {
    pub id: String,
    pub name: String,
    pub status: Status,
    pub devices: Vec<Device>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StorageController {
    #[serde(default)]
    pub member_id: String,
    pub name: String,
    pub status: Status,

    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub part_number: Option<String>,
    pub speed_gbps: Option<usize>,
    pub firmware_version: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StorageDevice {
    pub id: String,
    pub name: String,

    pub model: Option<String>,
    pub revision: Option<String>,
    pub capacity_bytes: Option<usize>,
    /// An indication of whether this drive currently predicts a failure in the near future.
    pub failure_predicted: Option<usize>,
    pub protocol: Option<String>,
    pub media_type: Option<String>,
    pub manufacturer: Option<String>,
    pub serial_number: Option<String>,
    pub part_number: Option<String>,
    pub encryption_ability: Option<String>,
    pub encryption_status: Option<String>,
    /// The rotation speed of this drive, in revolutions per minute (RPM) units.
    #[serde(rename = "RotationSpeedRPM")]
    pub rotation_speed_rpm: Option<usize>,
    pub block_size_bytes: Option<usize>,
    pub capable_speed_gbs: Option<usize>,
    pub negotiated_speed_gbs: Option<usize>,

    pub status: Status,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct StorageVolume {
    pub id: String,
    pub name: String,
    pub status: Status,

    #[serde(rename = "RAIDType")]
    pub raid_type: Option<String>,
    pub capacity_bytes: Option<usize>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Storage {
    pub id: String,
    pub name: String,
    pub status: Status,

    /// The set of storage controllers that this resource represents.
    #[serde(default)]
    pub storage_controllers: Vec<StorageController>,
    /// The set of drives attached to the storage controllers that this resource represents.
    #[serde(default)]
    pub drives: Vec<Link>,
    #[serde(default)]
    pub volumes: Option<Link>,
}
