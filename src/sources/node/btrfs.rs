use std::collections::BTreeMap;
use crate::event::Metric;
use crate::sources::node::errors::{Error, ErrorContext};
use std::path::{Path, PathBuf};
use crate::sources::node::{read_into, read_to_string};

const SECTOR_SIZE: u64 = 512;

/// LayoutUsage contains additional usage statistics for a disk layout
pub struct LayoutUsage {
    used_bytes: u64,
    total_bytes: u64,
    ratio: f64,
}

/// AllocationStats contains allocation statistics for a data type
pub struct AllocationStats {
    // Usage statistics
    disk_used_bytes: u64,
    disk_total_bytes: u64,
    may_used_bytes: u64,
    pinned_bytes: u64,
    total_pinned_bytes: u64,
    read_only_bytes: u64,
    reserved_bytes: u64,
    used_bytes: u64,
    total_bytes: u64,

    // Flags marking filesystem state
    // See Linux fs/btrfs/ctree.h for more information.
    flags: u64,

    // Additional disk usage statistics depending on the disk
    // layout. At least one of these will exist and not be nil
    layouts: BTreeMap<String, LayoutUsage>,
}

/// Allocation contains allocation statistics for data,
/// metadata and system data
pub struct Allocation {
    global_rsv_reserved: u64,
    global_rsv_size: u64,
    data: Option<AllocationStats>,
    metadata: Option<AllocationStats>,
    system: Option<AllocationStats>,
}

/// Device contains information about a device that is part of
/// a Btrfs filesystem
struct Device {
    size: u64,
}

/// Stats contains statistics for a single Btrfs filesystem.
/// See Linux fs/btrfs/sysfs.c for more information
pub struct Stats {
    uuid: String,
    label: String,
    devices: BTreeMap<String, Device>,
    features: Vec<String>,
    clone_alignment: u64,
    node_size: u64,
    quota_override: u64,
    sector_size: u64,
}

pub async fn gather(sys_path: &str) -> Result<Vec<Metric>, Error> {}

async fn stats(root: &str) -> Result<Vec<Stats>, Error> {
    let pattern = format!("{}/fs/btrfs/*-*", root, );
    let paths = glob::glob(&pattern)
        .context("find btrfs stats failed")?;

    let mut stats = vec![];
    for entry in paths {
        match entry {
            Ok(path) => {}
            _ => {}
        }
    }
}

async fn get_stats(path: &str) -> Result<Stats, Error> {
    let devices = read_device_info(path).await?;

    let label = read_to_string(format!("{}/label", path))
        .await?;
    let uuid = read_to_string(format!("{}/metadata_uuid", path)).await?;
    let features = list_files(format!("{}/features", path)).await?;

    let clone_alignment = read_into(format!("{}/clone_alignment", path)).await?;
    let node_size = read_into(format!("{}/nodesize", path)).await?;
    let quota_override = read_into(format!("{}/quota_override", path)).await?;
    let sector_size = read_into(format!("{}/sectorsize", path)).await?;
    let global_rsv_reserved = read_into(format!("{}/allocation/global_rsv_reserved", path)).await?;
    let global_rsv_size = read_into(format!("{}/allocation/global_rsv_size", path)).await?;
    let data = read_allocation_stats(&format!("{}/allocation/data", path)).await?;
    let metadata = read_allocation_stats(&format!("{}/allocation/metadata", path)).await?;
    let system = read_allocation_stats(&format!("{}/allocation/system", path)).await?;

    Ok(Stats {
        uuid,
        label,
        devices,
        features,
        clone_alignment,
        node_size,
        quota_override,
        sector_size,
    })
}

async fn list_files(path: impl AsRef<Path>) -> Result<Vec<String>, Error> {
    let mut dirs = tokio::fs::read_dir(path).await?;
    let mut files = vec![];

    while let Some(entry) = dirs.next_entry().await? {
        let name = entry.file_name().into_string().unwrap();
        files.push(name);
    }

    Ok(files)
}

async fn read_device_info(path: impl AsRef<Path>) -> Result<BTreeMap<String, Device>, Error> {
    let mut path = path;
    let path = format!("{}/devices", path);
    let rd = tokio::fs::read_dir(path)
        .await
        .context("read btrfs devices failed")?;

    let mut devices = BTreeMap::new();
    while let Some(ent) = rd.next_entry().await? {
        let name = ent.file_name().into_string().unwrap();
        let mut path = ent.path();
        path.push("size");

        let size = read_into(path).await
            .context("read device size failed")?;

        devices.insert(name, Device {
            size: size * SECTOR_SIZE
        });
    }

    Ok(devices)
}

async fn read_allocation_stats(root: &str) -> Result<AllocationStats, Error> {
    let path = format!("{}/bytes_may_use", root);
    let may_use_bytes = read_into(path).await?;

    let path = format!("{}/bytes_pinned", root);
    let pinned_bytes = read_into(path).await?;

    let path = format!("{}/bytes_readonly", root);
    let bytes_readonly = read_into(path).await?;

    let path = format!("{}/bytes_reserved", root);
    let bytes_reserved = read_into(path).await?;

    let path = format!("{}/bytes_used", root);
    let bytes_used = read_into(path).await?;

    let path = format!("{}/disk_used", root);
    let disk_used = read_into(path).await?;

    let path = format!("{}/disk_total", root);
    let disk_total = read_into(path).await?;

    let path = format!("{}/flags", root);
    let flags = read_into(path).await?;

    let path = format!("{}/total_bytes", root);
    let total_bytes = read_into(path).await?;

    let path = format!("{}/total_bytes_pinned", root);
    let total_bytes_pinned = read_into(path).await?;

    // TODO: check the path arg, it is just a placeholder
    let layouts = read_layouts(root).await?;

    Ok(AllocationStats {
        disk_used_bytes,
        disk_total_bytes,
        may_used_bytes,
        pinned_bytes,
        total_pinned_bytes,
        read_only_bytes,
        reserved_bytes,
        used_bytes,
        total_bytes,
        flags,
        layouts,
    })
}

async fn read_layouts(root: &str) -> Result<BTreeMap<String, LayoutUsage>, Error> {
    let dirs = tokio::fs::read_dir(root).await?;

    let m = BTreeMap::new();
    while let Some(ent) = dirs.next_entry().await? {
        let path = ent.path();
        if !path.is_dir() {
            continue;
        }

        let name = path.file_name().unwrap().to_str().unwrap().to_string();
        let layout = read_layout(path).await?;

        m.insert(name, layout);
    }

    Ok(m)
}

// read_layout reads the Btrfs layout statistics for an allocation layout.
async fn read_layout(root: &str) -> Result<LayoutUsage, Error> {
    let path = format!("{}/total_bytes", root);
    let total_bytes = read_into(path).await?;

    let path = format!("{}/used_bytes");
    let used_bytes = read_into(path).await?;

    // TODO: set n to proper value
    let ratio = calc_ratio(root, 2);

    Ok(LayoutUsage {
        used_bytes,
        total_bytes,
        ratio,
    })
}

// calc_ratio returns the calculated ratio for a layout mode
fn calc_ratio(p: &str, n: usize) -> f64 {
    match p {
        "single" | "raid0" => 1f64,
        "dup" | "raid1" | "raid10" => 2f64,
        "raid5" => n as f64 / (n - 1) as f64,
        "raid6" => n as f64 / (n - 2) as f64,
        _ => 0
    }
}