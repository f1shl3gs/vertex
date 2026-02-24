use std::path::PathBuf;

use configurable::Configurable;
use event::{Metric, tags};
use framework::config::serde_regex;
use serde::{Deserialize, Serialize};

use super::{Error, read_string};

#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
pub struct Config {
    // Regexp of net devices to ignore for netclass collector
    #[serde(default = "default_ignores")]
    #[serde(with = "serde_regex")]
    pub ignores: regex::Regex,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ignores: default_ignores(),
        }
    }
}

fn default_ignores() -> regex::Regex {
    regex::Regex::new("^$").unwrap()
}

pub async fn gather(conf: Config, sys_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let devices = net_class_devices(sys_path.clone())?;

    let mut metrics = Vec::new();
    for device in devices {
        if conf.ignores.is_match(&device) {
            continue;
        }

        let path = sys_path.join("class/net").join(&device);
        let nci = match NetClassInterface::parse(path) {
            Ok(nci) => nci,
            _ => continue,
        };

        let tags = tags!("device" => &device);
        metrics.extend([
            Metric::gauge_with_tags(
                "node_network_up",
                "Value is 1 if operstate is 'up', 0 otherwise",
                nci.operstate == "up",
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "node_network_info",
                "Non-numeric data from /sys/class/net/<iface>, value is always 1",
                1f64,
                tags!(
                    "address" => nci.address,
                    "adminstate" => admin_state(nci.flags),
                    "broadcast" => nci.broadcast,
                    "device" => device,
                    "duplex" => nci.duplex,
                    "ifalias" => nci.ifalias,
                    "operstate" => nci.operstate,
                ),
            ),
        ]);

        if let Some(v) = nci.addr_assign_type {
            metrics.push(Metric::gauge_with_tags(
                "node_network_address_assign_type",
                "Network device property: address_assign_type",
                v,
                tags.clone(),
            ));
        }

        if let Some(v) = nci.carrier {
            metrics.push(Metric::gauge_with_tags(
                "node_network_carrier",
                "Network device property: carrier",
                v,
                tags.clone(),
            ));
        }

        if let Some(v) = nci.carrier_changes {
            metrics.push(Metric::sum_with_tags(
                "node_network_carrier_changes_total",
                "Network device property: carrier_changes_total",
                v,
                tags.clone(),
            ));
        }

        if let Some(v) = nci.carrier_up_count {
            metrics.push(Metric::sum_with_tags(
                "node_network_carrier_up_changes_total",
                "Network device property: carrier_up_changes_total",
                v,
                tags.clone(),
            ));
        }

        if let Some(v) = nci.carrier_down_count {
            metrics.push(Metric::sum_with_tags(
                "node_network_carrier_down_changes_total",
                "Network device property: carrier_down_changes_total",
                v,
                tags.clone(),
            ));
        }

        if let Some(v) = nci.dev_id {
            metrics.push(Metric::gauge_with_tags(
                "node_network_device_id",
                "Network device property: device_id",
                v,
                tags.clone(),
            ));
        }

        if let Some(v) = nci.dormant {
            metrics.push(Metric::gauge_with_tags(
                "node_network_dormant",
                "Network device property: dormant",
                v,
                tags.clone(),
            ));
        }

        if let Some(v) = nci.flags {
            metrics.push(Metric::gauge_with_tags(
                "node_network_flags",
                "Network device property: flags",
                v,
                tags.clone(),
            ));
        }

        if let Some(v) = nci.ifindex {
            metrics.push(Metric::gauge_with_tags(
                "node_network_iface_id",
                "Network device property: iface_id",
                v,
                tags.clone(),
            ));
        }

        if let Some(v) = nci.iflink {
            metrics.push(Metric::gauge_with_tags(
                "node_network_iface_link",
                "Network device property: iface_link",
                v,
                tags.clone(),
            ));
        }

        if let Some(v) = nci.link_mode {
            metrics.push(Metric::gauge_with_tags(
                "node_network_iface_link_mode",
                "Network device property: iface_link_mode",
                v,
                tags.clone(),
            ));
        }

        if let Some(v) = nci.mtu {
            metrics.push(Metric::gauge_with_tags(
                "node_network_mtu_bytes",
                "Network device property: mtu_bytes",
                v,
                tags.clone(),
            ));
        }

        if let Some(v) = nci.name_assign_type {
            metrics.push(Metric::gauge_with_tags(
                "node_network_name_assign_type",
                "Network device property: name_assign_type",
                v,
                tags.clone(),
            ));
        }

        if let Some(v) = nci.netdev_group {
            metrics.push(Metric::gauge_with_tags(
                "node_network_net_dev_group",
                "netdev_group value of /sys/class/net/<iface>",
                v,
                tags.clone(),
            ));
        }

        if let Some(v) = nci.speed {
            // Some devices return -1 if the speed is unknown
            let speed_bytes = v as f64 * 1000.0 * 1000.0 / 8.0;
            metrics.push(Metric::gauge_with_tags(
                "node_network_speed_bytes",
                "Network device property: speed_bytes",
                speed_bytes,
                tags.clone(),
            ));
        }

        if let Some(v) = nci.tx_queue_len {
            metrics.push(Metric::gauge_with_tags(
                "node_network_transmit_queue_length",
                "Network device property: transmit_queue_length",
                v,
                tags.clone(),
            ))
        }

        if let Some(v) = nci.typ {
            metrics.push(Metric::gauge_with_tags(
                "node_network_protocol_type",
                "Network device property: protocol_type",
                v,
                tags,
            ));
        }
    }

    Ok(metrics)
}

#[inline]
fn admin_state(flags: Option<i64>) -> &'static str {
    match flags {
        Some(flags) => {
            if flags & 1 == 1 {
                "up"
            } else {
                "down"
            }
        }
        None => "unknown",
    }
}

fn net_class_devices(sys_path: PathBuf) -> Result<Vec<String>, Error> {
    let dirs = std::fs::read_dir(sys_path.join("class/net"))?;

    let mut devices = Vec::new();
    for entry in dirs.flatten() {
        devices.push(entry.file_name().into_string().unwrap());
    }

    Ok(devices)
}

#[derive(Default, Debug)]
struct NetClassInterface {
    _name: String,

    // /sys/class/net/<iface>/addr_assign_type
    addr_assign_type: Option<i64>,

    // /sys/class/net/<iface>/addr_len
    addr_len: Option<i64>,

    // /sys/class/net/<iface>/address
    address: String,

    // /sys/class/net/<iface>/broadcast
    broadcast: String,

    // /sys/class/net/<iface>/carrier
    carrier: Option<i64>,

    // /sys/class/net/<iface>/carrier_changes
    carrier_changes: Option<i64>,

    // /sys/class/net/<iface>/carrier_up_count
    carrier_up_count: Option<i64>,

    // /sys/class/net/<iface>/carrier_down_count
    carrier_down_count: Option<i64>,

    // /sys/class/net/<iface>/dev_id
    dev_id: Option<i64>,

    // /sys/class/net/<iface>/dormant
    dormant: Option<i64>,

    // /sys/class/net/<iface>/duplex
    duplex: String,

    // /sys/class/net/<iface>/flags
    flags: Option<i64>,

    // /sys/class/net/<iface>/ifalias
    ifalias: String,

    // /sys/class/net/<iface>/ifindex
    ifindex: Option<i64>,

    // /sys/class/net/<iface>/iflink
    iflink: Option<i64>,

    // /sys/class/net/<iface>/link_mode
    link_mode: Option<i64>,

    // /sys/class/net/<iface>/mtu
    mtu: Option<i64>,

    // /sys/class/net/<iface>/name_assign_type
    name_assign_type: Option<i64>,

    // /sys/class/net/<iface>/netdev_group
    netdev_group: Option<i64>,

    // /sys/class/net/<iface>/operstate
    operstate: String,

    // /sys/class/net/<iface>/phys_port_id
    phys_port_id: String,

    // /sys/class/net/<iface>/phys_port_name
    phys_port_name: String,

    // /sys/class/net/<iface>/phys_switch_id
    phys_switch_id: String,

    // /sys/class/net/<iface>/speed
    speed: Option<i64>,

    // /sys/class/net/<iface>/tx_queue_len
    tx_queue_len: Option<i64>,

    // /sys/class/net/<iface>/type
    typ: Option<i64>,
}

impl NetClassInterface {
    fn parse(path: PathBuf) -> Result<NetClassInterface, Error> {
        let dirs = std::fs::read_dir(path)?;

        let mut nci = NetClassInterface::default();
        for entry in dirs.flatten() {
            let file = entry.file_name();
            let value = match read_string(entry.path()) {
                Ok(v) => v,
                _ => continue,
            };

            match file.to_string_lossy().as_ref() {
                "addr_assign_type" => nci.addr_assign_type = value.parse().ok(),
                "addr_len" => nci.addr_len = value.parse().ok(),
                "address" => nci.address = value,
                "broadcast" => nci.broadcast = value,
                "carrier" => nci.carrier = value.parse().ok(),
                "carrier_changes" => nci.carrier_changes = value.parse().ok(),
                "carrier_up_count" => nci.carrier_up_count = value.parse().ok(),
                "carrier_down_count" => nci.carrier_down_count = value.parse().ok(),
                "dev_id" => {
                    nci.dev_id = i64::from_str_radix(value.strip_prefix("0x").unwrap(), 16).ok()
                }
                "dormant" => nci.dormant = value.parse().ok(),
                "duplex" => nci.duplex = value,
                "flags" => {
                    nci.flags = i64::from_str_radix(value.strip_prefix("0x").unwrap(), 16).ok()
                }
                "ifalias" => nci.ifalias = value,
                "ifindex" => nci.ifindex = value.parse().ok(),
                "iflink" => nci.iflink = value.parse().ok(),
                "link_mode" => nci.link_mode = value.parse().ok(),
                "mtu" => nci.mtu = value.parse().ok(),
                "name_assign_type" => nci.name_assign_type = value.parse().ok(),
                "netdev_group" => nci.netdev_group = value.parse().ok(),
                "operstate" => nci.operstate = value,
                "phys_port_id" => nci.phys_port_id = value,
                "phys_port_name" => nci.phys_port_name = value,
                "phys_switch_id" => nci.phys_switch_id = value,
                "speed" => nci.speed = value.parse().ok(),
                "tx_queue_len" => nci.tx_queue_len = value.parse().ok(),
                "type" => nci.typ = value.parse().ok(),
                _ => {}
            }
        }

        Ok(nci)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let path = "tests/node/sys/class/net/eth0";
        let nci = NetClassInterface::parse(path.into()).unwrap();

        assert_eq!(nci.addr_assign_type, Some(3));
        assert_eq!(nci.address, "01:01:01:01:01:01");
        assert_eq!(nci.addr_len, Some(6));
        assert_eq!(nci.broadcast, "ff:ff:ff:ff:ff:ff");
        assert_eq!(nci.carrier, Some(1));
        assert_eq!(nci.carrier_changes, Some(2));
        assert_eq!(nci.carrier_down_count, Some(1));
        assert_eq!(nci.carrier_up_count, Some(1));
        assert_eq!(nci.dev_id, Some(0x20));
        assert_eq!(nci.dormant, Some(1));
        assert_eq!(nci.duplex, "full");
        assert_eq!(nci.flags, Some(0x1303));
        assert_eq!(nci.ifalias, "");
        assert_eq!(nci.ifindex, Some(2));
        assert_eq!(nci.iflink, Some(2));
        assert_eq!(nci.link_mode, Some(1));
        assert_eq!(nci.mtu, Some(1500));
        assert_eq!(nci.name_assign_type, Some(2));
        assert_eq!(nci.netdev_group, Some(0));
        assert_eq!(nci.operstate, "up");
        assert_eq!(nci.phys_port_id, "");
        assert_eq!(nci.phys_port_name, "");
        assert_eq!(nci.phys_switch_id, "");
        assert_eq!(nci.speed, Some(1000));
        assert_eq!(nci.tx_queue_len, Some(1000));
        assert_eq!(nci.typ, Some(1));
    }
}
