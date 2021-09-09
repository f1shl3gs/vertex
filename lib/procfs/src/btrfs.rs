use crate::{ProcFS, SysFS};

use std::collections::BTreeMap;


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

impl ProcFS {
    pub async fn btrfs() -> Result<Vec<Stats>, Error> {}
}