use crate::{read_to_string, Error, SysFS};

/// DMIClass contains info from files in /sys/class/dmi/id.
#[derive(Default)]
pub struct DmiClass {
    bios_date: String,         // /sys/class/dmi/id/bios_date
    bios_release: String,      // /sys/class/dmi/id/bios_release
    bios_vendor: String,       // /sys/class/dmi/id/bios_vendor
    bios_version: String,      // /sys/class/dmi/id/bios_version
    board_asset_tag: String,   // /sys/class/dmi/id/board_asset_tag
    board_name: String,        // /sys/class/dmi/id/board_name
    board_serial: String,      // /sys/class/dmi/id/board_serial
    board_vendor: String,      // /sys/class/dmi/id/board_vendor
    board_version: String,     // /sys/class/dmi/id/board_version
    chassis_asset_tag: String, // /sys/class/dmi/id/chassis_asset_tag
    chassis_serial: String,    // /sys/class/dmi/id/chassis_serial
    chassis_type: String,      // /sys/class/dmi/id/chassis_type
    chassis_vendor: String,    // /sys/class/dmi/id/chassis_vendor
    chassis_version: String,   // /sys/class/dmi/id/chassis_version
    product_family: String,    // /sys/class/dmi/id/product_family
    product_name: String,      // /sys/class/dmi/id/product_name
    product_serial: String,    // /sys/class/dmi/id/product_serial
    product_sku: String,       // /sys/class/dmi/id/product_sku
    product_uuid: String,      // /sys/class/dmi/id/product_uuid
    product_version: String,   // /sys/class/dmi/id/product_version
    sys_vendor: String,        // /sys/class/dmi/id/sys_vendor
}

impl SysFS {
    pub async fn dmi(&self) -> Result<DmiClass, Error> {
        let path = self.root.join("class/dmi/id");
        let mut dirs = tokio::fs::read_dir(path).await?;
        let mut d = DmiClass::default();

        while let Some(file) = dirs.next_entry().await? {
            let metadata = file.metadata().await?;
            if !metadata.is_file() {
                continue;
            }

            let filename = file.file_name();
            let value = match read_to_string(file.path()).await {
                Ok(value) => value,
                Err(err) => {
                    if err.kind() == std::io::ErrorKind::PermissionDenied {
                        continue;
                    }

                    return Err(err.into());
                }
            };

            match filename.to_str().unwrap() {
                "bios_date" => d.bios_date = value,
                "bios_release" => d.bios_release = value,
                "bios_vendor" => d.bios_vendor = value,
                "bios_version" => d.bios_version = value,
                "board_asset_tag" => d.board_asset_tag = value,
                "board_name" => d.board_name = value,
                "board_serial" => d.board_serial = value,
                "board_vendor" => d.board_vendor = value,
                "board_version" => d.board_version = value,
                "chassis_asset_tag" => d.chassis_asset_tag = value,
                "chassis_serial" => d.chassis_serial = value,
                "chassis_type" => d.chassis_type = value,
                "chassis_vendor" => d.chassis_vendor = value,
                "chassis_version" => d.chassis_version = value,
                "product_family" => d.product_family = value,
                "product_name" => d.product_name = value,
                "product_serial" => d.product_serial = value,
                "product_sku" => d.product_sku = value,
                "product_uuid" => d.product_uuid = value,
                "product_version" => d.product_version = value,
                "sys_vendor" => d.sys_vendor = value,
                _ => continue
            }
        }

        Ok(d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dmi() {
        let sysfs = SysFS::test_sysfs();
        let d = sysfs.dmi().await.unwrap();

        assert_eq!(d.bios_date, "04/12/2021".to_string());
        assert_eq!(d.bios_release, "2.2".to_string());
        assert_eq!(d.bios_vendor, "Dell Inc.".to_string());
        assert_eq!(d.bios_version, "2.2.4".to_string());
        assert_eq!(d.board_asset_tag, "".to_string());
        assert_eq!(d.board_name, "07PXPY".to_string());
        assert_eq!(d.board_serial, ".7N62AI2.GRTCL6944100GP.".to_string());
        assert_eq!(d.board_vendor, "Dell Inc.".to_string());
        assert_eq!(d.board_version, "A01".to_string());
        assert_eq!(d.chassis_asset_tag, "".to_string());
        assert_eq!(d.chassis_serial, "7N62AI2".to_string());
        assert_eq!(d.chassis_type, "23".to_string());
        assert_eq!(d.chassis_vendor, "Dell Inc.".to_string());
        assert_eq!(d.chassis_version, "".to_string());
        assert_eq!(d.product_family, "PowerEdge".to_string());
        assert_eq!(d.product_name, "PowerEdge R6515".to_string());
        assert_eq!(d.product_serial, "7N62AI2".to_string());
        assert_eq!(d.product_sku, "SKU=NotProvided;ModelName=PowerEdge R6515".to_string());
        assert_eq!(d.product_uuid, "83340ca8-cb49-4474-8c29-d2088ca84dd9".to_string());
        assert_eq!(d.product_version, "System Version".to_string());
        assert_eq!(d.sys_vendor, "Dell Inc.".to_string());
    }
}