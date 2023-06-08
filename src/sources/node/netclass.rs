use event::{tags, Metric};
use framework::config::serde_regex;
use serde::{Deserialize, Serialize};

use super::{read_to_string, Error, ErrorContext};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NetClassConfig {
    // Regexp of net devices to ignore for netclass collector
    #[serde(default = "default_ignores")]
    #[serde(with = "serde_regex")]
    pub ignores: regex::Regex,
}

impl Default for NetClassConfig {
    fn default() -> Self {
        Self {
            ignores: default_ignores(),
        }
    }
}

fn default_ignores() -> regex::Regex {
    regex::Regex::new("^$").unwrap()
}

pub async fn gather(conf: &NetClassConfig, sys_path: &str) -> Result<Vec<Metric>, Error> {
    let devices = net_class_devices(sys_path)
        .await
        .context("read net class devices failed")?;

    let mut metrics = Vec::new();
    for device in devices {
        let device = &device;
        if conf.ignores.is_match(device) {
            continue;
        }

        let path = format!("{}/class/net/{}", sys_path, device);
        let nci = match NetClassInterface::from(&path).await {
            Ok(nci) => nci,
            _ => continue,
        };

        let mut up = 0.0;
        if nci.operstate == "up" {
            up = 1.0;
        }

        metrics.push(Metric::gauge_with_tags(
            "node_network_up",
            "Value is 1 if operstat is 'up', o otherwise",
            up,
            tags!(
                "device" => device,
            ),
        ));

        metrics.push(Metric::gauge_with_tags(
            "node_network_info",
            "Non-numeric data from /sys/class/net/<iface>, value is always 1",
            1f64,
            tags!(
                "device" => device,
                "address" => nci.address,
                "broadcast" => nci.broadcast,
                "duplex" => nci.duplex,
                "operstate" => nci.operstate,
                "ifalias" => nci.ifalias,
            ),
        ));

        if let Some(v) = nci.addr_assign_type {
            metrics.push(Metric::gauge_with_tags(
                "node_network_address_assign_type",
                "address_assign_type value of /sys/class/net/<iface>",
                v as f64,
                tags!(
                    "device" => device,
                ),
            ));
        }

        if let Some(v) = nci.carrier {
            metrics.push(Metric::gauge_with_tags(
                "node_network_carrier",
                "carrier value of /sys/class/net/<iface>",
                v as f64,
                tags!(
                    "device" => device,
                ),
            ));
        }

        if let Some(v) = nci.carrier_changes {
            metrics.push(Metric::sum_with_tags(
                "node_network_carrier_changes_total",
                "carrier_changes value of /sys/class/net/<iface>",
                v as f64,
                tags!(
                    "device" => device,
                ),
            ));
        }

        if let Some(v) = nci.carrier_up_count {
            metrics.push(Metric::sum_with_tags(
                "node_network_carrier_up_changes_total",
                "carrier_up_count value of /sys/class/net/<iface>",
                v as f64,
                tags!(
                    "device" => device,
                ),
            ));
        }

        if let Some(v) = nci.carrier_down_count {
            metrics.push(Metric::sum_with_tags(
                "node_network_carrier_down_changes_total",
                "carrier_down_count value of /sys/class/net/<iface>",
                v as f64,
                tags!(
                    "device" => device,
                ),
            ));
        }

        if let Some(v) = nci.dev_id {
            metrics.push(Metric::gauge_with_tags(
                "node_network_device_id",
                "dev_id value of /sys/class/net/<iface>",
                v as f64,
                tags!(
                    "device" => device,
                ),
            ));
        }

        if let Some(v) = nci.dormant {
            metrics.push(Metric::gauge_with_tags(
                "node_network_dormant",
                "dormant value of /sys/class/net/<iface>",
                v as f64,
                tags!(
                    "device" => device,
                ),
            ));
        }

        if let Some(v) = nci.flags {
            metrics.push(Metric::gauge_with_tags(
                "node_network_flags",
                "flags value of /sys/class/net/<iface>",
                v as f64,
                tags!(
                    "device" => device,
                ),
            ));
        }

        if let Some(v) = nci.ifindex {
            metrics.push(Metric::gauge_with_tags(
                "node_network_iface_id",
                "ifindex value of /sys/class/net/<iface>",
                v as f64,
                tags!(
                    "device" => device,
                ),
            ));
        }

        if let Some(v) = nci.iflink {
            metrics.push(Metric::gauge_with_tags(
                "node_network_iface_link",
                "iflink value of /sys/class/net/<iface>",
                v as f64,
                tags!(
                    "device" => device,
                ),
            ));
        }

        if let Some(v) = nci.link_mode {
            metrics.push(Metric::gauge_with_tags(
                "node_network_iface_link_mode",
                "link_mode value of /sys/class/net/<iface>",
                v as f64,
                tags!(
                    "device" => device,
                ),
            ));
        }

        if let Some(v) = nci.mtu {
            metrics.push(Metric::gauge_with_tags(
                "node_network_mtu_bytes",
                "mtu value of /sys/class/net/<iface>",
                v as f64,
                tags!(
                    "device" => device,
                ),
            ));
        }

        if let Some(v) = nci.name_assign_type {
            metrics.push(Metric::gauge_with_tags(
                "node_network_name_assign_type",
                "name_assign_type value of /sys/class/net/<iface>",
                v as f64,
                tags!(
                    "device" => device,
                ),
            ));
        }

        if let Some(v) = nci.netdev_group {
            metrics.push(Metric::gauge_with_tags(
                "node_network_net_dev_group",
                "netdev_group value of /sys/class/net/<iface>",
                v as f64,
                tags!(
                    "device" => device,
                ),
            ));
        }

        if let Some(v) = nci.speed {
            // Some devices return -1 if the speed is unknown
            let speed_bytes = v as f64 * 1000.0 * 1000.0 / 8.0;
            metrics.push(Metric::gauge_with_tags(
                "node_network_speed_bytes",
                "speed value of /sys/class/net/<iface>",
                speed_bytes,
                tags!(
                        "device" => device,
                ),
            ));
        }

        if let Some(v) = nci.tx_queue_len {
            metrics.push(Metric::gauge_with_tags(
                "node_network_transmit_queue_length",
                "tx_queue_len value of /sys/class/net/<iface>",
                v as f64,
                tags!(
                    "device" => device,
                ),
            ))
        }

        if let Some(v) = nci.typ {
            metrics.push(Metric::gauge_with_tags(
                "node_network_protocol_type",
                "type value of /sys/class/net/<iface>",
                v as f64,
                tags!(
                    "device" => device,
                ),
            ));
        }
    }

    Ok(metrics)
}

async fn net_class_devices(sys_path: &str) -> Result<Vec<String>, Error> {
    let path = format!("{}/class/net", sys_path);
    let mut dirs = std::fs::read_dir(path)?;
    let mut devices = Vec::new();

    while let Some(Ok(ent)) = dirs.next() {
        devices.push(ent.file_name().into_string().unwrap());
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
    pub async fn from(device_path: &str) -> Result<NetClassInterface, Error> {
        let mut dirs = std::fs::read_dir(device_path)?;
        let mut nci = NetClassInterface::default();

        while let Some(Ok(entry)) = dirs.next() {
            let file = entry.file_name();
            let file = file.to_str().unwrap();

            let value = match read_to_string(entry.path()).await {
                Ok(v) => v,
                _ => continue,
            };

            match file {
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

    #[tokio::test]
    async fn test_netcalss_interface() {
        let path = "tests/fixtures/sys/class/net/eth0";
        let nci = NetClassInterface::from(path).await.unwrap();

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
