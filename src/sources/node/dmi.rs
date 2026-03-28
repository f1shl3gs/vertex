use std::io::ErrorKind;
use std::path::PathBuf;

use event::Metric;
use event::tags::Tags;

use super::{Error, Paths, read_sys_file};

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
    sys_vendor: Option<String>,        // /sys/class/dmi/id/sys_vendor
}

fn load_dmi(root: PathBuf) -> Result<DesktopManagementInterface, Error> {
    let mut dmi = DesktopManagementInterface::default();

    for (filename, dst) in [
        ("bios_date", &mut dmi.bios_date),
        ("bios_release", &mut dmi.bios_release),
        ("bios_vendor", &mut dmi.bios_vendor),
        ("bios_version", &mut dmi.bios_version),
        ("board_asset_tag", &mut dmi.board_asset_tag),
        ("board_name", &mut dmi.board_name),
        ("board_serial", &mut dmi.board_serial),
        ("board_vendor", &mut dmi.board_vendor),
        ("board_version", &mut dmi.board_version),
        ("chassis_asset_tag", &mut dmi.chassis_asset_tag),
        ("chassis_serial", &mut dmi.chassis_serial),
        ("chassis_type", &mut dmi.chassis_type),
        ("chassis_vendor", &mut dmi.chassis_vendor),
        ("chassis_version", &mut dmi.chassis_version),
        ("product_family", &mut dmi.product_family),
        ("product_name", &mut dmi.product_name),
        ("product_serial", &mut dmi.product_serial),
        ("product_sku", &mut dmi.product_sku),
        ("product_uuid", &mut dmi.product_uuid),
        ("product_version", &mut dmi.product_version),
        ("sys_vendor", &mut dmi.sys_vendor),
    ] {
        match read_sys_file(root.join(filename)) {
            Ok(content) => {
                *dst = Some(content);
            }
            Err(err) => {
                if err.kind() == ErrorKind::NotFound || err.kind() == ErrorKind::PermissionDenied {
                    // Only root is allowed to read the serial and product_uuid files
                    continue;
                }

                return Err(err.into());
            }
        }
    }

    Ok(dmi)
}

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let dmi = load_dmi(paths.sys().join("class/dmi/id"))?;

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
    if let Some(value) = dmi.sys_vendor {
        tags.insert("system_vendor", value);
    }

    Ok(vec![Metric::gauge_with_tags(
        "node_dmi_info",
        "A metric with a constant '1' value labeled by bios_date, bios_release, bios_vendor, \
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

    #[tokio::test]
    async fn smoke() {
        let paths = Paths::test();
        let metrics = collect(paths).await.unwrap();
        assert!(!metrics.is_empty());
    }

    #[test]
    fn parse() {
        let dmi = load_dmi("tests/node/fixtures/sys/class/dmi/id".into()).unwrap();

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
        assert_eq!(dmi.sys_vendor.unwrap(), "Dell Inc.");
    }
}
