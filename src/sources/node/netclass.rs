use serde::{Deserialize, Serialize};
use crate::{
    config::{deserialize_regex, serialize_regex}
};
use crate::event::Metric;
use crate::sources::node::errors::Error;
use tokio::fs;
use std::os::unix::fs::MetadataExt;

#[derive(Debug, Deserialize, Serialize)]
pub struct NetClassConfig {
    // Regexp of net devices to ignore for netclass collector
    #[serde(default = "default_ignores")]
    #[serde(deserialize_with = "deserialize_regex", serialize_with = "serialize_with")]
    pub ignores: regex::Regex,
}

impl Default for NetClassConfig {
    fn default() -> Self {
        Self {
            ignores: regex::Regex::new("^$").unwrap()
        }
    }
}

pub async fn gather(conf: &NetClassConfig, sys_path: &str) -> Result<Vec<Metric>, ()> {
    todo!()
}

struct NetClassInterface {
    name: String,

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
    tpe: Option<i64>,
}

impl NetClassInterface {
    pub async fn from(device_path: &str) -> Result<NetClassInterface, Error> {
        let mut dirs = fs::read_dir(device_path).await.map_err(Error::from)?;

        while let Some(entry) = dirs.next_entry().await.map_err(Error::from)? {
            let meta = match fs::metadata(entry.path()).await {
                Ok(m) => m,
                Err(_) => continue
            };

            let mode = meta.mode();
            if mode &
        }

        Ok(())
    }
}