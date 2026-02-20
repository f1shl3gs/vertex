use std::io::ErrorKind;
use std::path::PathBuf;

use event::Metric;
use event::tags::Tags;

use super::{Error, read_string};

/// `DesktopManagementInterface` contains info from files in /sys/class/dmi/id
#[derive(Default)]
struct DesktopManagementInterface {
    bios_date: Option<String>,         // /sys/class/dmi/id/bios_date
    bios_release: Option<String>,      // /sys/class/dmi/id/bios_release
    bios_vendor: Option<String>,       // /sys/class/dmi/id/bios_vendor
    bios_version: Option<String>,      // /sys/class/dmi/id/bios_version
    board_asset_tag: Option<String>,   // /sys/class/dmi/id/board_asset_tag
    board_name: Option<String>,        // /sys/class/dmi/id/board_name
    board_serial: Option<String>,      // /sys/class/dmi/id/board_serial
    board_vendor: Option<String>,      // /sys/class/dmi/id/board_vendor
    board_version: Option<String>,     // /sys/class/dmi/id/board_version
    chassis_asset_tag: Option<String>, // /sys/class/dmi/id/chassis_asset_tag
    chassis_serial: Option<String>,    // /sys/class/dmi/id/chassis_serial
    chassis_type: Option<String>,      // /sys/class/dmi/id/chassis_type
    chassis_vendor: Option<String>,    // /sys/class/dmi/id/chassis_vendor
    chassis_version: Option<String>,   // /sys/class/dmi/id/chassis_version
    product_family: Option<String>,    // /sys/class/dmi/id/product_family
    product_name: Option<String>,      // /sys/class/dmi/id/product_name
    product_serial: Option<String>,    // /sys/class/dmi/id/product_serial
    product_sku: Option<String>,       // /sys/class/dmi/id/product_sku
    product_uuid: Option<String>,      // /sys/class/dmi/id/product_uuid
    product_version: Option<String>,   // /sys/class/dmi/id/product_version
    system_vendor: Option<String>,     // /sys/class/dmi/id/sys_vendor
}

impl DesktopManagementInterface {
    fn parse(root: PathBuf) -> Result<Self, Error> {
        let dirs = std::fs::read_dir(root)?;

        let mut dmi = DesktopManagementInterface::default();
        for entry in dirs.flatten() {
            if !entry.metadata()?.is_file() {
                continue;
            }

            match entry.file_name().to_string_lossy().as_ref() {
                "bios_date" => {
                    dmi.bios_date = {
                        match read_string(entry.path()) {
                            Ok(value) => Some(value),
                            Err(err) => {
                                if err.kind() == ErrorKind::PermissionDenied {
                                    continue;
                                }

                                return Err(err.into());
                            }
                        }
                    }
                }
                "bios_release" => {
                    dmi.bios_release = {
                        match read_string(entry.path()) {
                            Ok(value) => Some(value),
                            Err(err) => {
                                if err.kind() == ErrorKind::PermissionDenied {
                                    continue;
                                }

                                return Err(err.into());
                            }
                        }
                    }
                }
                "bios_vendor" => {
                    dmi.bios_vendor = {
                        match read_string(entry.path()) {
                            Ok(value) => Some(value),
                            Err(err) => {
                                if err.kind() == ErrorKind::PermissionDenied {
                                    continue;
                                }

                                return Err(err.into());
                            }
                        }
                    }
                }
                "bios_version" => {
                    dmi.bios_version = {
                        match read_string(entry.path()) {
                            Ok(value) => Some(value),
                            Err(err) => {
                                if err.kind() == ErrorKind::PermissionDenied {
                                    continue;
                                }

                                return Err(err.into());
                            }
                        }
                    }
                }
                "board_asset_tag" => {
                    dmi.board_asset_tag = {
                        match read_string(entry.path()) {
                            Ok(value) => Some(value),
                            Err(err) => {
                                if err.kind() == ErrorKind::PermissionDenied {
                                    continue;
                                }

                                return Err(err.into());
                            }
                        }
                    }
                }
                "board_name" => {
                    dmi.board_name = {
                        match read_string(entry.path()) {
                            Ok(value) => Some(value),
                            Err(err) => {
                                if err.kind() == ErrorKind::PermissionDenied {
                                    continue;
                                }

                                return Err(err.into());
                            }
                        }
                    }
                }
                "board_serial" => {
                    dmi.board_serial = {
                        match read_string(entry.path()) {
                            Ok(value) => Some(value),
                            Err(err) => {
                                if err.kind() == ErrorKind::PermissionDenied {
                                    continue;
                                }

                                return Err(err.into());
                            }
                        }
                    }
                }
                "board_vendor" => {
                    dmi.board_vendor = {
                        match read_string(entry.path()) {
                            Ok(value) => Some(value),
                            Err(err) => {
                                if err.kind() == ErrorKind::PermissionDenied {
                                    continue;
                                }

                                return Err(err.into());
                            }
                        }
                    }
                }
                "board_version" => {
                    dmi.board_version = {
                        match read_string(entry.path()) {
                            Ok(value) => Some(value),
                            Err(err) => {
                                if err.kind() == ErrorKind::PermissionDenied {
                                    continue;
                                }

                                return Err(err.into());
                            }
                        }
                    }
                }
                "chassis_asset_tag" => {
                    dmi.chassis_asset_tag = {
                        match read_string(entry.path()) {
                            Ok(value) => Some(value),
                            Err(err) => {
                                if err.kind() == ErrorKind::PermissionDenied {
                                    continue;
                                }

                                return Err(err.into());
                            }
                        }
                    }
                }
                "chassis_serial" => {
                    dmi.chassis_serial = {
                        match read_string(entry.path()) {
                            Ok(value) => Some(value),
                            Err(err) => {
                                if err.kind() == ErrorKind::PermissionDenied {
                                    continue;
                                }

                                return Err(err.into());
                            }
                        }
                    }
                }
                "chassis_type" => {
                    dmi.chassis_type = {
                        match read_string(entry.path()) {
                            Ok(value) => Some(value),
                            Err(err) => {
                                if err.kind() == ErrorKind::PermissionDenied {
                                    continue;
                                }

                                return Err(err.into());
                            }
                        }
                    }
                }
                "chassis_vendor" => {
                    dmi.chassis_vendor = {
                        match read_string(entry.path()) {
                            Ok(value) => Some(value),
                            Err(err) => {
                                if err.kind() == ErrorKind::PermissionDenied {
                                    continue;
                                }

                                return Err(err.into());
                            }
                        }
                    }
                }
                "chassis_version" => {
                    dmi.chassis_version = {
                        match read_string(entry.path()) {
                            Ok(value) => Some(value),
                            Err(err) => {
                                if err.kind() == ErrorKind::PermissionDenied {
                                    continue;
                                }

                                return Err(err.into());
                            }
                        }
                    }
                }
                "product_family" => {
                    dmi.product_family = {
                        match read_string(entry.path()) {
                            Ok(value) => Some(value),
                            Err(err) => {
                                if err.kind() == ErrorKind::PermissionDenied {
                                    continue;
                                }

                                return Err(err.into());
                            }
                        }
                    }
                }
                "product_name" => {
                    dmi.product_name = {
                        match read_string(entry.path()) {
                            Ok(value) => Some(value),
                            Err(err) => {
                                if err.kind() == ErrorKind::PermissionDenied {
                                    continue;
                                }

                                return Err(err.into());
                            }
                        }
                    }
                }
                "product_serial" => {
                    dmi.product_serial = {
                        match read_string(entry.path()) {
                            Ok(value) => Some(value),
                            Err(err) => {
                                if err.kind() == ErrorKind::PermissionDenied {
                                    continue;
                                }

                                return Err(err.into());
                            }
                        }
                    }
                }
                "product_sku" => {
                    dmi.product_sku = {
                        match read_string(entry.path()) {
                            Ok(value) => Some(value),
                            Err(err) => {
                                if err.kind() == ErrorKind::PermissionDenied {
                                    continue;
                                }

                                return Err(err.into());
                            }
                        }
                    }
                }
                "product_uuid" => {
                    dmi.product_uuid = {
                        match read_string(entry.path()) {
                            Ok(value) => Some(value),
                            Err(err) => {
                                if err.kind() == ErrorKind::PermissionDenied {
                                    continue;
                                }

                                return Err(err.into());
                            }
                        }
                    }
                }
                "product_version" => {
                    dmi.product_version = {
                        match read_string(entry.path()) {
                            Ok(value) => Some(value),
                            Err(err) => {
                                if err.kind() == ErrorKind::PermissionDenied {
                                    continue;
                                }

                                return Err(err.into());
                            }
                        }
                    }
                }
                "sys_vendor" => {
                    dmi.system_vendor = {
                        match read_string(entry.path()) {
                            Ok(value) => Some(value),
                            Err(err) => {
                                if err.kind() == ErrorKind::PermissionDenied {
                                    continue;
                                }

                                return Err(err.into());
                            }
                        }
                    }
                }
                _ => continue,
            }
        }

        Ok(dmi)
    }
}

pub async fn gather(sys_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let path = sys_path.join("class/dmi/id");
    let dmi = DesktopManagementInterface::parse(path)?;

    let mut tags = Tags::with_capacity(20);
    if let Some(value) = dmi.bios_date {
        tags.insert("bios_date", value);
    }
    if let Some(value) = dmi.bios_release {
        tags.insert("bios_release", value);
    }
    if let Some(value) = dmi.bios_vendor {
        tags.insert("bios_vendor", value);
    }
    if let Some(value) = dmi.bios_version {
        tags.insert("bios_version", value);
    }
    if let Some(value) = dmi.board_asset_tag {
        tags.insert("board_asset_tag", value);
    }
    if let Some(value) = dmi.board_name {
        tags.insert("board_name", value);
    }
    if let Some(value) = dmi.board_serial {
        tags.insert("board_serial", value);
    }
    if let Some(value) = dmi.board_vendor {
        tags.insert("board_vendor", value);
    }
    if let Some(value) = dmi.board_version {
        tags.insert("board_version", value);
    }
    if let Some(value) = dmi.chassis_asset_tag {
        tags.insert("chassis_asset_tag", value);
    }
    if let Some(value) = dmi.chassis_serial {
        tags.insert("chassis_serial", value);
    }
    if let Some(value) = dmi.chassis_vendor {
        tags.insert("chassis_vendor", value);
    }
    if let Some(value) = dmi.chassis_version {
        tags.insert("chassis_version", value);
    }
    if let Some(value) = dmi.product_family {
        tags.insert("product_family", value);
    }
    if let Some(value) = dmi.product_name {
        tags.insert("product_name", value);
    }
    if let Some(value) = dmi.product_serial {
        tags.insert("product_serial", value);
    }
    if let Some(value) = dmi.product_sku {
        tags.insert("product_sku", value);
    }
    if let Some(value) = dmi.product_uuid {
        tags.insert("product_uuid", value);
    }
    if let Some(value) = dmi.product_version {
        tags.insert("product_version", value);
    }
    if let Some(value) = dmi.system_vendor {
        tags.insert("system_vendor", value);
    }

    Ok(vec![Metric::gauge_with_tags(
        "node_dmi_info",
        "A metric with a constant '1' value labeled by bios_date, bios_release, bios_vendor,\
         bios_version, board_asset_tag, board_name, board_serial, board_vendor, board_version, \
         chassis_asset_tag, chassis_serial, chassis_vendor, chassis_version, product_family, \
         product_name, product_serial, product_sku, product_uuid, product_version, system_vendor \
         if provided by DMI.",
        1,
        tags,
    )])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let dmi = DesktopManagementInterface::parse("tests/node/sys/class/dmi/id".into()).unwrap();

        assert_eq!(dmi.bios_date.unwrap(), "04/12/2021");
        assert_eq!(dmi.bios_release.unwrap(), "2.2");
        assert_eq!(dmi.bios_vendor.unwrap(), "Dell Inc.");
        assert_eq!(dmi.bios_version.unwrap(), "2.2.4");
        assert_eq!(dmi.board_name.unwrap(), "07PXPY");
        assert_eq!(dmi.board_serial.unwrap(), ".7N62AI2.GRTCL6944100GP.");
        assert_eq!(dmi.board_vendor.unwrap(), "Dell Inc.");
        assert_eq!(dmi.board_version.unwrap(), "A01");
        assert_eq!(dmi.chassis_serial.unwrap(), "7N62AI2");
        assert_eq!(dmi.chassis_type.unwrap(), "23");
        assert_eq!(dmi.chassis_vendor.unwrap(), "Dell Inc.");
        assert_eq!(dmi.product_family.unwrap(), "PowerEdge");
        assert_eq!(dmi.product_name.unwrap(), "PowerEdge R6515");
        assert_eq!(dmi.product_serial.unwrap(), "7N62AI2");
        assert_eq!(
            dmi.product_sku.unwrap(),
            "SKU=NotProvided;ModelName=PowerEdge R6515"
        );
        assert_eq!(
            dmi.product_uuid.unwrap(),
            "83340ca8-cb49-4474-8c29-d2088ca84dd9"
        );
        assert_eq!(dmi.system_vendor.unwrap(), "Dell Inc.");
    }
}
