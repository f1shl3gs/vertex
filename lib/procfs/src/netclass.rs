use crate::{read_to_string, Error, SysFS};

#[derive(Default, Debug)]
pub struct NetClassInterface {
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
    typ: Option<i64>,
}

impl SysFS {
    pub async fn netclass(&self) -> Result<Vec<String>, Error> {
        let path = self.root.join("class/net");
        let mut dirs = tokio::fs::read_dir(path).await?;
        let mut devices = Vec::new();

        while let Some(ent) = dirs.next_entry().await? {
            let metadata = ent.metadata().await?;
            if !metadata.is_dir() {
                continue;
            }

            devices.push(ent.file_name().into_string().unwrap());
        }

        Ok(devices)
    }

    pub async fn netclass_interface(&self, name: &str) -> Result<NetClassInterface, Error> {
        let path = format!("{}/class/net/{}", self.root.to_string_lossy(), name);
        let mut dirs = tokio::fs::read_dir(path).await?;
        let mut nci = NetClassInterface::default();

        while let Some(entry) = dirs.next_entry().await? {
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
        let sysfs = SysFS::test_sysfs();
        let devs = sysfs.netclass().await.unwrap();
        assert_eq!(devs.len(), 4);
        assert_eq!(devs[2], "eth0".to_string());

        let nci = sysfs.netclass_interface("eth0").await.unwrap();
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
