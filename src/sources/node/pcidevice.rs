#![deny(dead_code)]

use std::io::ErrorKind;
use std::path::PathBuf;

use event::{Metric, tags};

use super::Error;

/// The location of the device attached
/// "0000:00:00.0" represents Segment:Bus:Device.Function
#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Default)]
struct Location {
    segment: i32,
    bus: i32,
    device: i32,
    function: i32,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Default)]
enum PowerState {
    #[default]
    Unknown,
    Error,
    D0,
    D1,
    D2,
    D3Hot,
    D3Cold,
}

impl PowerState {
    fn as_str(&self) -> &'static str {
        match self {
            PowerState::Unknown => "unknown",
            PowerState::Error => "error",
            PowerState::D0 => "D0",
            PowerState::D1 => "D1",
            PowerState::D2 => "D2",
            PowerState::D3Hot => "D3hot",
            PowerState::D3Cold => "D3cold",
        }
    }
}

/// Information from files in /sys/bus/pci/devices for a single PCI device
#[cfg_attr(test, derive(Debug, PartialEq))]
#[derive(Default)]
struct Device {
    location: Location,
    parent_location: Option<Location>,

    class: u32,            // /sys/bus/pci/devices/<Location>/class
    vendor: u32,           // /sys/bus/pci/devices/<Location>/vendor
    device: u32,           // /sys/bus/pci/devices/<Location>/device
    subsystem_vendor: u32, // /sys/bus/pci/devices/<Location>/subsystem_vendor
    subsystem_device: u32, // /sys/bus/pci/devices/<Location>/subsystem_device
    revision: u32,         // /sys/bus/pci/devices/<Location>/revision

    numa_node: Option<i32>, // /sys/bus/pci/devices/<Location>/numa_node

    max_link_speed: Option<f64>, // /sys/bus/pci/devices/<Location>/max_link_speed
    max_link_width: Option<f64>, // /sys/bus/pci/devices/<Location>/max_link_width
    current_link_speed: Option<f64>, // /sys/bus/pci/devices/<Location>/current_link_speed
    current_link_width: Option<f64>, // /sys/bus/pci/devices/<Location>/current_link_width

    sriov_drivers_autoprobe: Option<u32>, // /sys/bus/pci/devices/<Location>/sriov_drivers_autoprobe
    sriov_numvfs: Option<u32>,            // /sys/bus/pci/devices/<Location>/sriov_numvfs
    sriov_offset: Option<u32>,            // /sys/bus/pci/devices/<Location>/sriov_offset
    sriov_stride: Option<u32>,            // /sys/bus/pci/devices/<Location>/sriov_stride
    sriov_totalvfs: Option<u32>,          // /sys/bus/pci/devices/<Location>/sriov_totalvfs
    sriov_vf_device: Option<u32>,         // /sys/bus/pci/devices/<Location>/sriov_vf_device
    sriov_vf_total_msix: Option<u64>,     // /sys/bus/pci/devices/<Location>/sriov_vf_total_msix

    d3cold_allowed: Option<u32>, // /sys/bus/pci/devices/<Location>/d3cold_allowed
    power_state: Option<PowerState>, // /sys/bus/pci/devices/<Location>/power_state
}

pub async fn collect(sys_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let devices = load_devices(sys_path.join("bus/pci/devices"))?;

    let mut metrics = Vec::with_capacity(devices.len() * 12);
    for device in devices {
        let mut info_tags = tags!(
            "segment" => format!("{:04x}", device.location.segment),
            "bus" => format!("{:02x}", device.location.bus),
            "device" => format!("{:02x}", device.location.device),
            "function" => format!("{:x}", device.location.function),
            // extra tags
            "class_id" => format!("0x{:06x}", device.class),
            "vendor_id" => format!("0x{:04x}", device.vendor),
            "device_id" => format!("0x{:04x}", device.device),
            "subsystem_vendor_id" => format!("0x{:04x}", device.subsystem_vendor),
            "subsystem_device_id" => format!("0x{:04x}", device.subsystem_device),
            "revision" => format!("0x{:02x}", device.revision),
        );
        match device.parent_location {
            Some(location) => {
                info_tags.insert("parent_segment", format!("{:04x}", location.segment));
                info_tags.insert("parent_bus", format!("{:02x}", location.bus));
                info_tags.insert("parent_device", format!("{:02x}", location.device));
                info_tags.insert("parent_function", format!("{:x}", location.function));
            }
            None => {
                info_tags.insert("parent_segment", "*");
                info_tags.insert("parent_bus", "*");
                info_tags.insert("parent_device", "*");
                info_tags.insert("parent_function", "*");
            }
        }

        metrics.push(Metric::gauge_with_tags(
            "node_pcidevice_info",
            "Non-numeric data from /sys/bus/pci/devices/<location>, value is always 1.",
            1,
            info_tags,
        ));

        let tags = tags!(
            "segment" => format!("{:04x}", device.location.segment),
            "bus" => format!("{:02x}", device.location.bus),
            "device" => format!("{:02x}", device.location.device),
            "function" => format!("{:x}", device.location.function),
        );
        metrics.extend([
            Metric::gauge_with_tags(
                "node_pcidevice_max_link_transfers_per_second",
                "Value of maximum link's transfers per second (T/s)",
                device.max_link_speed.map(|v| v * 1e9).unwrap_or(-1.0),
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_pcidevice_max_link_width",
                "Value of maximum link's width (number of lanes)",
                device.max_link_width.unwrap_or(-1.0),
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_pcidevice_current_link_transfers_per_second",
                "Value of current link's transfers per second (T/s)",
                device.current_link_speed.map(|v| v * 1e9).unwrap_or(-1.0),
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_pcidevice_current_link_width",
                "Value of current link's width (number of lanes)",
                device.current_link_width.unwrap_or(-1.0),
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_pcidevice_d3cold_allowed",
                "Whether the PCIe device supports D3cold power state (0/1).",
                device.d3cold_allowed.unwrap_or(0),
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_pcidevice_sriov_drivers_autoprobe",
                "Whether SR-IOV drivers autoprobe is enabled for the device (0/1).",
                device.sriov_drivers_autoprobe.unwrap_or(0),
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_pcidevice_sriov_numvfs",
                "Number of Virtual Functions (VFs) currently enabled for SR-IOV.",
                device.sriov_numvfs.unwrap_or(0),
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_pcidevice_sriov_totalvfs",
                "Total number of Virtual Functions (VFs) supported by the device.",
                device.sriov_totalvfs.unwrap_or(0),
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_pcidevice_sriov_vf_total_msix",
                "Total number of MSI-X vectors for Virtual Functions.",
                device.sriov_vf_total_msix.unwrap_or(0),
                tags.clone(),
            ),
        ]);

        // emit power state metrics with state labels only if power state is available
        if let Some(power_state) = device.power_state {
            for state in ["D0", "D1", "D2", "D3hot", "D3cold", "unknown", "error"] {
                metrics.push(Metric::gauge_with_tags(
                    "node_pcidevice_power_state",
                    "PCIe device power state, one of: D0, D1, D2, D3hot, D3cold, unknown or error.",
                    power_state.as_str() == state,
                    tags!(
                        "segment" => format!("{:04x}", device.location.segment),
                        "bus" => format!("{:02x}", device.location.bus),
                        "device" => format!("{:02x}", device.location.device),
                        "function" => format!("{:x}", device.location.function),
                        "state" => state
                    ),
                ))
            }
        }

        // only emit numa_node metric if the value is available (not -1)
        if let Some(value) = device.numa_node
            && value != -1
        {
            metrics.push(Metric::gauge_with_tags(
                "node_pcidevice_numa_node",
                "NUMA node number for the PCI device. -1 indicates unknown or not available.",
                value,
                tags,
            ));
        }
    }

    Ok(metrics)
}

fn load_devices(root: PathBuf) -> Result<Vec<Device>, Error> {
    let dirs = std::fs::read_dir(root)?;

    let mut devices = Vec::new();
    for entry in dirs.flatten() {
        let Ok(real) = entry.path().read_link() else {
            continue;
        };

        let Some(location) = parse_location(real.file_name().unwrap().to_string_lossy().as_ref())
        else {
            continue;
        };

        let parent = real
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy();
        let parent_location = if parent.starts_with("pci") {
            None
        } else {
            parse_location(parent.as_ref())
        };

        let mut device = Device {
            location,
            parent_location,
            ..Device::default()
        };

        // these files must exist in a device directory
        for filename in [
            "class",
            "vendor",
            "device",
            "subsystem_vendor",
            "subsystem_device",
            "revision",
        ] {
            let content = std::fs::read_to_string(entry.path().join(filename))?;
            let content = content.trim_end().trim_start_matches("0x");
            let value = u32::from_str_radix(content, 16)?;

            match filename {
                "class" => device.class = value,
                "vendor" => device.vendor = value,
                "device" => device.device = value,
                "subsystem_vendor" => device.subsystem_vendor = value,
                "subsystem_device" => device.subsystem_device = value,
                "revision" => device.revision = value,
                _ => unreachable!(),
            }
        }

        // optional files
        for filename in [
            "max_link_speed",
            "max_link_width",
            "current_link_speed",
            "current_link_width",
            "numa_node",
        ] {
            let content = match std::fs::read_to_string(entry.path().join(filename)) {
                Ok(content) => content,
                Err(err) => {
                    if err.kind() == ErrorKind::NotFound {
                        continue;
                    }

                    return Err(Error::from(err));
                }
            };

            match filename {
                "max_link_speed" => {
                    device.max_link_speed = content
                        .split_once(' ')
                        .and_then(|(value, _)| value.parse::<f64>().ok());
                }
                "current_link_speed" => {
                    device.current_link_speed = content
                        .split_once(' ')
                        .and_then(|(value, _)| value.parse::<f64>().ok());
                }
                "max_link_width" => {
                    device.max_link_width = content.trim().parse().ok();
                }
                "current_link_width" => {
                    device.current_link_width = content.trim().parse().ok();
                }
                "numa_node" => {
                    device.numa_node = content.trim().parse().ok();
                }
                _ => unreachable!(),
            }
        }

        // parse SR-IOV files (these are optional and may not exist for all devices
        for filename in [
            "sriov_drivers_autoprobe",
            "sriov_numvfs",
            "sriov_offset",
            "sriov_stride",
            "sriov_totalvfs",
            "sriov_vf_device",
            "sriov_vf_total_msix",
        ] {
            let content = match std::fs::read_to_string(entry.path().join(filename)) {
                Ok(content) => content,
                Err(err) => {
                    if err.kind() == ErrorKind::NotFound {
                        continue;
                    }

                    return Err(Error::from(err));
                }
            };

            let content = content.trim();
            if content.is_empty() {
                continue;
            }

            match filename {
                "sriov_drivers_autoprobe" => {
                    device.sriov_drivers_autoprobe = content.parse::<u32>().ok();
                }
                "sriov_numvfs" => {
                    device.sriov_numvfs = content.parse().ok();
                }
                "sriov_offset" => {
                    device.sriov_offset = content.parse().ok();
                }
                "sriov_stride" => {
                    device.sriov_stride = content.parse().ok();
                }
                "sriov_totalvfs" => {
                    device.sriov_totalvfs = content.parse().ok();
                }
                "sriov_vf_device" => {
                    device.sriov_vf_device = content.parse().ok();
                }
                "sriov_vf_total_msix" => {
                    device.sriov_vf_total_msix = content.parse().ok();
                }
                _ => unreachable!(),
            }
        }

        // parse power management files (there are optional and may not exist for all devices)
        for filename in ["d3cold_allowed", "power_state"] {
            let content = match std::fs::read_to_string(entry.path().join(filename)) {
                Ok(content) => content,
                Err(err) => {
                    if err.kind() == ErrorKind::NotFound {
                        continue;
                    }

                    return Err(Error::from(err));
                }
            };

            let content = content.trim();
            if content.is_empty() {
                continue;
            }

            match filename {
                "d3cold_allowed" => {
                    device.d3cold_allowed = content.parse::<u32>().ok();
                }
                "power_state" => {
                    // power_state is a string (one of: "unknown", "error", "D0", "D1", "D2", "D3hot", "D3cold")
                    device.power_state = Some(match content {
                        "D0" => PowerState::D0,
                        "D1" => PowerState::D1,
                        "D2" => PowerState::D2,
                        "D3hot" => PowerState::D3Hot,
                        "D3cold" => PowerState::D3Cold,
                        "error" => PowerState::Error,
                        _ => PowerState::Unknown,
                    });
                }
                _ => unreachable!(),
            }
        }

        devices.push(device);
    }

    Ok(devices)
}

// "0000:00:00.0"
fn parse_location(input: &str) -> Option<Location> {
    let mut parts = input.split([':', '.']);

    let segment = i32::from_str_radix(parts.next()?, 16).ok()?;
    let bus = i32::from_str_radix(parts.next()?, 16).ok()?;
    let device = i32::from_str_radix(parts.next()?, 16).ok()?;
    let function = i32::from_str_radix(parts.next()?, 16).ok()?;

    Some(Location {
        segment,
        bus,
        device,
        function,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn location() {
        let location = parse_location("0001:9b:0c.0").unwrap();

        assert_eq!(
            location,
            Location {
                segment: 1,
                bus: 0x9b,
                device: 0xc,
                function: 0,
            }
        )
    }

    #[test]
    fn devices() {
        let devices = load_devices("tests/node/sys/bus/pci/devices".into()).unwrap();

        let want = vec![
            Device {
                location: Location {
                    segment: 0,
                    bus: 0,
                    device: 2,
                    function: 1,
                },
                parent_location: None,

                class: 0x060400,
                vendor: 0x1022,
                device: 0x1634,
                subsystem_vendor: 0x17aa,
                subsystem_device: 0x5095,
                revision: 0x00,
                numa_node: Some(-1),

                max_link_speed: Some(8.0),
                max_link_width: Some(8.0),
                current_link_speed: Some(8.0),
                current_link_width: Some(4.0),

                sriov_drivers_autoprobe: Some(0),
                sriov_numvfs: Some(0),
                sriov_offset: None,
                sriov_stride: None,
                sriov_totalvfs: Some(0),
                sriov_vf_device: None,
                sriov_vf_total_msix: Some(0),

                d3cold_allowed: Some(1),
                power_state: Some(PowerState::D0),
            },
            Device {
                location: Location {
                    segment: 0,
                    bus: 1,
                    device: 0,
                    function: 0,
                },
                parent_location: Some(Location {
                    segment: 0,
                    bus: 0,
                    device: 2,
                    function: 1,
                }),

                class: 0x010802,
                vendor: 0xc0a9,
                device: 0x540a,
                subsystem_vendor: 0xc0a9,
                subsystem_device: 0x5021,
                revision: 0x01,
                numa_node: Some(-1),

                max_link_speed: Some(16.0),
                max_link_width: Some(4.0),
                current_link_speed: Some(8.0),
                current_link_width: Some(4.0),

                sriov_drivers_autoprobe: Some(1),
                sriov_numvfs: Some(4),
                sriov_offset: None,
                sriov_stride: None,
                sriov_totalvfs: Some(8),
                sriov_vf_device: None,
                sriov_vf_total_msix: Some(16),

                d3cold_allowed: Some(1),
                power_state: Some(PowerState::D0),
            },
            Device {
                location: Location {
                    segment: 0,
                    bus: 0x45,
                    device: 0,
                    function: 0,
                },
                parent_location: Some(Location {
                    segment: 0,
                    bus: 0x40,
                    device: 1,
                    function: 3,
                }),

                class: 0x020000,
                vendor: 0x8086,
                device: 0x1521,
                subsystem_vendor: 0x8086,
                subsystem_device: 0x00a3,
                revision: 0x01,
                numa_node: Some(0),

                max_link_speed: Some(5.0),
                max_link_width: Some(4.0),
                current_link_speed: Some(5.0),
                current_link_width: Some(4.0),

                // SR-IOV fields
                sriov_drivers_autoprobe: Some(1),
                sriov_numvfs: Some(0),
                sriov_offset: Some(128),
                sriov_stride: Some(4),
                sriov_totalvfs: Some(7),
                sriov_vf_device: Some(1520),
                sriov_vf_total_msix: Some(0),

                // Power management fields
                d3cold_allowed: Some(1),
                power_state: Some(PowerState::D0),
            },
        ];

        assert_eq!(want, devices);
    }
}
