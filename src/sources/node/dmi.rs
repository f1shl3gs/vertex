use std::fs::read_dir;
use std::io::ErrorKind;
use std::path::PathBuf;

use event::{tags, Metric};

use super::{read_to_string, Error};

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
        let mut dirs = read_dir(root)?;
        let mut dmi = DesktopManagementInterface::default();

        while let Some(Ok(entry)) = dirs.next() {
            if !entry.metadata()?.is_file() {
                continue;
            }

            let name = entry.file_name();
            if name == "modalias" || name == "uevent" {
                continue;
            }

            let value = match read_to_string(entry.path()) {
                Ok(value) => value,
                Err(err) => {
                    if err.kind() == ErrorKind::PermissionDenied {
                        continue;
                    }

                    return Err(err.into());
                }
            };

            match name.to_string_lossy().as_ref() {
                "bios_date" => dmi.bios_date = Some(value),
                "bios_release" => dmi.bios_release = Some(value),
                "bios_vendor" => dmi.bios_vendor = Some(value),
                "bios_version" => dmi.bios_version = Some(value),
                "board_asset_tag" => dmi.board_asset_tag = Some(value),
                "board_name" => dmi.board_name = Some(value),
                "board_serial" => dmi.board_serial = Some(value),
                "board_vendor" => dmi.board_vendor = Some(value),
                "board_version" => dmi.board_version = Some(value),
                "chassis_asset_tag" => dmi.chassis_asset_tag = Some(value),
                "chassis_serial" => dmi.chassis_serial = Some(value),
                "chassis_type" => dmi.chassis_type = Some(value),
                "chassis_vendor" => dmi.chassis_vendor = Some(value),
                "chassis_version" => dmi.chassis_version = Some(value),
                "product_family" => dmi.product_family = Some(value),
                "product_name" => dmi.product_name = Some(value),
                "product_serial" => dmi.product_serial = Some(value),
                "product_sku" => dmi.product_sku = Some(value),
                "product_uuid" => dmi.product_uuid = Some(value),
                "product_version" => dmi.product_version = Some(value),
                "sys_vendor" => dmi.system_vendor = Some(value),
                _ => continue,
            }
        }

        Ok(dmi)
    }
}

pub async fn gather(sys_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let path = sys_path.join("class/dmi/id");
    let dmi = DesktopManagementInterface::parse(path)?;

    Ok(vec![Metric::gauge_with_tags(
        "node_dmi_info",
        "A metric with a constant '1' value labeled by bios_date, bios_release, bios_vendor,\
         bios_version, board_asset_tag, board_name, board_serial, board_vendor, board_version, \
         chassis_asset_tag, chassis_serial, chassis_vendor, chassis_version, product_family, \
         product_name, product_serial, product_sku, product_uuid, product_version, system_vendor \
         if provided by DMI.",
        1,
        tags!(
            "bios_date" => dmi.bios_date.unwrap_or_default(),
            "bios_release" => dmi.bios_release.unwrap_or_default(),
            "bios_vendor" => dmi.bios_vendor.unwrap_or_default(),
            "bios_version" => dmi.bios_version.unwrap_or_default(),
            "board_asset_tag" => dmi.board_asset_tag.unwrap_or_default(),
            "board_name" => dmi.board_name.unwrap_or_default(),
            "board_serial" => dmi.board_serial.unwrap_or_default(),
            "board_vendor" => dmi.board_vendor.unwrap_or_default(),
            "board_version" => dmi.board_version.unwrap_or_default(),
            "chassis_asset_tag" => dmi.chassis_asset_tag.unwrap_or_default(),
            "chassis_serial" => dmi.chassis_serial.unwrap_or_default(),
            "chassis_type" => dmi.chassis_type.unwrap_or_default(),
            "chassis_vendor" => dmi.chassis_vendor.unwrap_or_default(),
            "chassis_version" => dmi.chassis_version.unwrap_or_default(),
            "product_family" => dmi.product_family.unwrap_or_default(),
            "product_name" => dmi.product_name.unwrap_or_default(),
            "product_serial" => dmi.product_serial.unwrap_or_default(),
            "product_sku" => dmi.product_sku.unwrap_or_default(),
            "product_uuid" => dmi.product_uuid.unwrap_or_default(),
            "product_version" => dmi.product_version.unwrap_or_default(),
            "sys_vendor" => dmi.system_vendor.unwrap_or_default(),
        ),
    )])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let dmi =
            DesktopManagementInterface::parse("tests/fixtures/sys/class/dmi/id".into()).unwrap();

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
