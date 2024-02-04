use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use event::{tags, tags::Key, Metric};

use super::{read_into, read_to_string, Error};

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
    allocation: Allocation,
    devices: BTreeMap<String, Device>,
    features: Vec<String>,
    clone_alignment: u64,
    node_size: u64,
    quota_override: u64,
    sector_size: u64,
}

pub async fn gather(sys_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let stats = stats(sys_path).await?;

    let mut metrics = vec![];
    for stat in stats {
        metrics.extend(stats_to_metrics(stat));
    }

    Ok(metrics)
}

fn stats_to_metrics(stats: Stats) -> Vec<Metric> {
    let mut metrics = vec![
        Metric::gauge_with_tags(
            "node_btrfs_info",
            "Filesystem information",
            1.0,
            tags!(
                "label" => stats.label
            ),
        ),
        Metric::gauge(
            "node_btrfs_global_rsv_size_bytes",
            "Size of global reserve.",
            stats.allocation.global_rsv_size as f64,
        ),
    ];

    // Information about devices
    for (name, device) in stats.devices {
        metrics.push(Metric::gauge_with_tags(
            "node_btrfs_device_size_bytes",
            "Size of a device that is part of the filesystem.",
            device.size as f64,
            tags!(
                Key::from_static("device") => name
            ),
        ));
    }

    // Information about data, metadata and system data.
    if let Some(s) = stats.allocation.data {
        metrics.extend(get_allocation_stats("data", s));
    }
    if let Some(s) = stats.allocation.metadata {
        metrics.extend(get_allocation_stats("metadata", s));
    }
    if let Some(s) = stats.allocation.system {
        metrics.extend(get_allocation_stats("system", s));
    }

    metrics
}

fn get_allocation_stats(typ: &str, stats: AllocationStats) -> Vec<Metric> {
    let mut metrics = vec![Metric::gauge_with_tags(
        "node_btrfs_reserved_bytes",
        "Amount of space reserved for a data type",
        stats.reserved_bytes as f64,
        tags!(
            Key::from_static("block_group_type") => typ
        ),
    )];

    // Add all layout statistics
    for (mode, usage) in stats.layouts {
        metrics.extend_from_slice(&[
            Metric::gauge_with_tags(
                "node_btrfs_used_bytes",
                "Amount of used space by a layout/data type",
                usage.used_bytes as f64,
                tags!(
                    Key::from_static("block_group_type") => typ,
                    Key::from_static("mode") => mode.clone()
                ),
            ),
            Metric::gauge_with_tags(
                "node_btrfs_size_bytes",
                "Amount of space allocated for a layout/data type",
                usage.total_bytes as f64,
                tags!(
                    Key::from_static("block_group_type") => typ,
                    Key::from_static("mode") => mode.clone()
                ),
            ),
            Metric::gauge_with_tags(
                "node_btrfs_allocation_ratio",
                "Data allocation ratio for a layout/data type",
                usage.ratio,
                tags!(
                    Key::from_static("block_group_type") => typ,
                    Key::from_static("mode") => mode
                ),
            ),
        ])
    }

    metrics
}

fn get_layout_metrics(typ: &str, mode: &str, s: LayoutUsage) -> Vec<Metric> {
    vec![
        Metric::gauge_with_tags(
            "node_btrfs_used_bytes",
            "Amount of used space by a layout/data type",
            s.used_bytes as f64,
            tags!(
                "block_group_type" => typ.to_string(),
                "mode" => mode.to_string()
            ),
        ),
        Metric::gauge_with_tags(
            "node_btrfs_size_bytes",
            "Amount of space allocated for a layout/data type",
            s.total_bytes as f64,
            tags!(
                "block_group_type" => typ.to_string(),
                "mode" => mode.to_string()
            ),
        ),
        Metric::gauge_with_tags(
            "node_btrfs_allocation_ratio",
            "Data allocation ratio for a layout/data type",
            s.ratio,
            tags!(
                "block_group_type" => typ.to_string(),
                "mode" => mode.to_string()
            ),
        ),
    ]
}

async fn stats(root: PathBuf) -> Result<Vec<Stats>, Error> {
    let pattern = format!("{}/fs/btrfs/*-*", root.to_string_lossy());
    let paths = glob::glob(&pattern)?;

    let mut stats = vec![];
    for path in paths.flatten() {
        let s = get_stats(path).await?;
        stats.push(s);
    }

    Ok(stats)
}

async fn get_stats(root: PathBuf) -> Result<Stats, Error> {
    let devices = read_device_info(&root).await?;

    let path = root.join("label");
    let label = read_to_string(path)?;

    let path = root.join("metadata_uuid");
    let uuid = read_to_string(path)?;

    let path = root.join("features");
    let features = list_files(path).await?;

    let path = root.join("clone_alignment");
    let clone_alignment = read_into(path)?;

    let path = root.join("nodesize");
    let node_size = read_into(path)?;

    let path = root.join("quota_override");
    let quota_override = read_into(path)?;

    let path = root.join("sectorsize");
    let sector_size = read_into(path)?;

    let path = root.join("allocation/global_rsv_reserved");
    let global_rsv_reserved = read_into(path)?;

    let path = root.join("allocation/global_rsv_size");
    let global_rsv_size = read_into(path)?;

    let path = root.join("allocation/data");
    let data = read_allocation_stats(path, devices.len()).await.ok();

    let path = root.join("allocation/metadata");
    let metadata = read_allocation_stats(path, devices.len()).await.ok();

    let path = root.join("allocation/system");
    let system = read_allocation_stats(path, devices.len()).await.ok();

    Ok(Stats {
        uuid,
        label,
        devices,
        features,
        clone_alignment,
        node_size,
        quota_override,
        sector_size,
        allocation: Allocation {
            global_rsv_reserved,
            global_rsv_size,
            data,
            metadata,
            system,
        },
    })
}

async fn list_files(path: impl AsRef<Path>) -> Result<Vec<String>, Error> {
    let mut dirs = std::fs::read_dir(path)?;
    let mut files = vec![];

    while let Some(Ok(entry)) = dirs.next() {
        let name = entry.file_name().into_string().unwrap();
        files.push(name);
    }

    Ok(files)
}

async fn read_device_info(path: &Path) -> Result<BTreeMap<String, Device>, Error> {
    let path = path.join("devices");
    let mut dirs = std::fs::read_dir(path)?;

    let mut devices = BTreeMap::new();
    while let Some(Ok(entry)) = dirs.next() {
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path().join("size");
        let size: u64 = read_into(path).unwrap_or(0);

        devices.insert(
            name,
            Device {
                size: size * SECTOR_SIZE,
            },
        );
    }

    Ok(devices)
}

async fn read_allocation_stats(root: PathBuf, devices: usize) -> Result<AllocationStats, Error> {
    let may_used_bytes = read_into(root.join("bytes_may_use"))?;
    let pinned_bytes = read_into(root.join("bytes_pinned"))?;
    let read_only_bytes = read_into(root.join("bytes_readonly"))?;
    let reserved_bytes = read_into(root.join("bytes_reserved"))?;
    let used_bytes = read_into(root.join("bytes_used"))?;
    let disk_used_bytes = read_into(root.join("disk_used"))?;
    let disk_total_bytes = read_into(root.join("disk_total"))?;
    let flags = read_into(root.join("flags"))?;
    let total_bytes = read_into(root.join("total_bytes"))?;
    // this file may not exists
    let total_pinned_bytes = read_into(root.join("total_bytes_pinned")).unwrap_or_default();
    let layouts = read_layouts(root, devices).await?;

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

async fn read_layouts(
    root: PathBuf,
    devices: usize,
) -> Result<BTreeMap<String, LayoutUsage>, Error> {
    let mut dirs = std::fs::read_dir(root)?;

    let mut layouts = BTreeMap::new();
    while let Some(Ok(entry)) = dirs.next() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        let layout = read_layout(path, devices).await?;

        layouts.insert(name, layout);
    }

    Ok(layouts)
}

// read_layout reads the Btrfs layout statistics for an allocation layout.
async fn read_layout(root: PathBuf, devices: usize) -> Result<LayoutUsage, Error> {
    let total_bytes = read_into(root.join("total_bytes"))?;
    let used_bytes = read_into(root.join("used_bytes"))?;

    let name = root.file_name().unwrap().to_string_lossy();
    let ratio = calc_ratio(&name, devices);

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
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_stats() {
        let path = "tests/fixtures/sys";
        let stats = stats(path.into()).await.unwrap();

        struct Alloc {
            layout: String,
            size: u64,
            ratio: f64,
        }

        struct Expected {
            uuid: String,
            label: String,
            devices: usize,
            features: usize,
            data: Alloc,
            meta: Alloc,
            system: Alloc,
        }

        let wants = vec![
            Expected {
                uuid: "0abb23a9-579b-43e6-ad30-227ef47fcb9d".to_string(),
                label: "fixture".to_string(),
                devices: 2,
                features: 4,
                data: Alloc {
                    layout: "raid0".to_string(),
                    size: 2147483648,
                    ratio: 1.0,
                },
                meta: Alloc {
                    layout: "raid1".to_string(),
                    size: 1073741824,
                    ratio: 2.0,
                },
                system: Alloc {
                    layout: "raid1".to_string(),
                    size: 8388608,
                    ratio: 2.0,
                },
            },
            Expected {
                uuid: "7f07c59f-6136-449c-ab87-e1cf2328731b".to_string(),
                label: "".to_string(),
                devices: 4,
                features: 5,
                data: Alloc {
                    layout: "raid5".to_string(),
                    size: 644087808,
                    ratio: 4.0 / 3.0,
                },
                meta: Alloc {
                    layout: "raid6".to_string(),
                    size: 429391872,
                    ratio: 4.0 / 2.0,
                },
                system: Alloc {
                    layout: "raid6".to_string(),
                    size: 16777216,
                    ratio: 4.0 / 2.0,
                },
            },
        ];

        assert_eq!(wants.len(), stats.len());
        for i in 0..wants.len() {
            let want = &wants[i];
            let got = &stats[i];

            assert_eq!(got.uuid, want.uuid);
            assert_eq!(got.label, want.label);
            assert_eq!(got.devices.len(), want.devices);
            assert_eq!(got.features.len(), want.features);
            assert_eq!(
                got.allocation.data.as_ref().unwrap().total_bytes,
                want.data.size
            );
            assert_eq!(
                got.allocation.metadata.as_ref().unwrap().total_bytes,
                want.meta.size
            );
            assert_eq!(
                got.allocation.system.as_ref().unwrap().total_bytes,
                want.system.size
            );

            assert_eq!(
                got.allocation
                    .data
                    .as_ref()
                    .unwrap()
                    .layouts
                    .get(&want.data.layout)
                    .unwrap()
                    .ratio,
                want.data.ratio
            );
            assert_eq!(
                got.allocation
                    .metadata
                    .as_ref()
                    .unwrap()
                    .layouts
                    .get(&want.meta.layout)
                    .unwrap()
                    .ratio,
                want.meta.ratio
            );
            assert_eq!(
                got.allocation
                    .system
                    .as_ref()
                    .unwrap()
                    .layouts
                    .get(&want.system.layout)
                    .unwrap()
                    .ratio,
                want.system.ratio
            );
        }
    }

    #[tokio::test]
    async fn test_read_device_info() {
        let path =
            PathBuf::from("tests/fixtures/sys/fs/btrfs/7f07c59f-6136-449c-ab87-e1cf2328731b");
        let _infos = read_device_info(&path).await.unwrap();
    }
}
