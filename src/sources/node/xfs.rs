//! Exposes XFS runtime statistics
//!
//! Linux (kernel 4.4+)

use std::num::ParseIntError;
use std::path::PathBuf;

use event::{Metric, tags};

use super::Error;

pub async fn gather(sys_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let stats = xfs_sys_stats(sys_path).await?;

    let mut metrics = Vec::with_capacity(stats.len() * 39);
    for stat in stats {
        let tags = tags!("device" => stat.name);

        metrics.extend([
            Metric::sum_with_tags(
                "node_xfs_extent_allocation_extents_allocated_total",
                "Number of extents allocated for a filesystem.",
                stat.extent_allocation.extents_allocated,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_extent_allocation_blocks_allocated_total",
                "Number of blocks allocated for a filesystem.",
                stat.extent_allocation.blocks_allocated,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_extent_allocation_extents_freed_total",
                "Number of extents freed for a filesystem.",
                stat.extent_allocation.extents_freed,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_extent_allocation_blocks_freed_total",
                "Number of blocks freed for a filesystem.",
                stat.extent_allocation.blocks_freed,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_allocation_btree_lookups_total",
                "Number of allocation B-tree lookups for a filesystem.",
                stat.allocation_btree.lookups,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_allocation_btree_compares_total",
                "Number of allocation B-tree compares for a filesystem.",
                stat.allocation_btree.compares,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_allocation_btree_records_inserted_total",
                "Number of allocation B-tree records inserted for a filesystem.",
                stat.allocation_btree.records_inserted,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_allocation_btree_records_deleted_total",
                "Number of allocation B-tree records deleted for a filesystem.",
                stat.allocation_btree.records_deleted,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_block_mapping_reads_total",
                "Number of block map for read operations for a filesystem.",
                stat.block_mapping.reads,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_block_mapping_writes_total",
                "Number of block map for write operations for a filesystem.",
                stat.block_mapping.writes,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_block_mapping_unmaps_total",
                "Number of block unmaps (deletes) for a filesystem.",
                stat.block_mapping.unmaps,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_block_mapping_extent_list_insertions_total",
                "Number of extent list insertions for a filesystem.",
                stat.block_mapping.extent_list_insertions,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_block_mapping_extent_list_deletions_total",
                "Number of extent list deletions for a filesystem.",
                stat.block_mapping.extent_list_deletions,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_block_mapping_extent_list_lookups_total",
                "Number of extent list lookups for a filesystem.",
                stat.block_mapping.extent_list_lookups,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_block_mapping_extent_list_compares_total",
                "Number of extent list compares for a filesystem.",
                stat.block_mapping.extent_list_compares,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_block_map_btree_lookups_total",
                "Number of block map B-tree lookups for a filesystem.",
                stat.block_map_btree.lookups,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_block_map_btree_compares_total",
                "Number of block map B-tree compares for a filesystem.",
                stat.block_map_btree.compares,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_block_map_btree_records_inserted_total",
                "Number of block map B-tree records inserted for a filesystem.",
                stat.block_map_btree.records_inserted,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_block_map_btree_records_deleted_total",
                "Number of block map B-tree records deleted for a filesystem.",
                stat.block_map_btree.records_deleted,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_directory_operation_lookup_total",
                "Number of file name directory lookups which miss the operating systems directory name lookup cache.",
                stat.directory_operation.lookups,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_directory_operation_create_total",
                "Number of times a new directory entry was created for a filesystem.",
                stat.directory_operation.creates,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_directory_operation_remove_total",
                "Number of times an existing directory entry was created for a filesystem.",
                stat.directory_operation.removes,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_directory_operation_getdents_total",
                "Number of times the directory getdents operation was performed for a filesystem.",
                stat.directory_operation.get_dents,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_inode_operation_attempts_total",
                "Number of times the OS looked for an XFS inode in the inode cache.",
                stat.inode_operation.attempts,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_inode_operation_found_total",
                "Number of times the OS looked for and found an XFS inode in the inode cache.",
                stat.inode_operation.found,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_inode_operation_recycled_total",
                "Number of times the OS found an XFS inode in the cache, but could not use it as it was being recycled.",
                stat.inode_operation.recycle,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_inode_operation_missed_total",
                "Number of times the OS looked for an XFS inode in the cache, but did not find it.",
                stat.inode_operation.missed,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_inode_operation_duplicates_total",
                "Number of times the OS tried to add a missing XFS inode to the inode cache, but found it had already been added by another process.",
                stat.inode_operation.duplicate,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_inode_operation_reclaims_total",
                "Number of times the OS reclaimed an XFS inode from the inode cache to free memory for another purpose.",
                stat.inode_operation.reclaims,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_inode_operation_attribute_changes_total",
                "Number of times the OS explicitly changed the attributes of an XFS inode.",
                stat.inode_operation.attribute_change,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_read_calls_total",
                "Number of read(2) system calls made to files in a filesystem.",
                stat.read_write.read,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_write_calls_total",
                "Number of write(2) system calls made to files in a filesystem.",
                stat.read_write.write,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_vnode_active_total",
                "Number of vnodes not on free lists for a filesystem.",
                stat.vnode.active,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_vnode_allocate_total",
                "Number of times vn_alloc called for a filesystem.",
                stat.vnode.allocate,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_vnode_get_total",
                "Number of times vn_get called for a filesystem.",
                stat.vnode.get,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_vnode_hold_total",
                "Number of times vn_hold called for a filesystem.",
                stat.vnode.hold,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_vnode_release_total",
                "Number of times vn_rele called for a filesystem.",
                stat.vnode.release,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_vnode_reclaim_total",
                "Number of times vn_reclaim called for a filesystem.",
                stat.vnode.reclaim,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "node_xfs_vnode_remove_total",
                "Number of times vn_remove called for a filesystem.",
                stat.vnode.remove,
                tags,
            )
        ]);
    }

    Ok(metrics)
}

// ExtentAllocationStats contains statistics regarding XFS extent allocations.
#[derive(Debug, Default)]
struct ExtentAllocationStats {
    extents_allocated: u32,
    blocks_allocated: u32,
    extents_freed: u32,
    blocks_freed: u32,
}

// BtreeStats contains statistics regarding an XFS internal B-tree
#[derive(Debug, Default)]
struct BTreeStats {
    lookups: u32,
    compares: u32,
    records_inserted: u32,
    records_deleted: u32,
}

// BlockMappingStats contains statistics regarding XFS block maps
#[derive(Debug, Default)]
struct BlockMappingStats {
    reads: u32,
    writes: u32,
    unmaps: u32,
    extent_list_insertions: u32,
    extent_list_deletions: u32,
    extent_list_lookups: u32,
    extent_list_compares: u32,
}

// DirectoryOperationStats contains statistics regarding XFS directory entries
#[derive(Debug, Default)]
struct DirectoryOperationStats {
    lookups: u32,
    creates: u32,
    removes: u32,
    get_dents: u32,
}

// TransactionStats contains statistics regarding XFS metadata transactions
#[derive(Debug, Default)]
struct TransactionStats {
    synchronous: u32,
    asynchronous: u32,
    empty: u32,
}

// InodeOperationStats contains statistics regarding XFS inode operations
#[derive(Debug, Default)]
struct InodeOperationStats {
    attempts: u32,
    found: u32,
    recycle: u32,
    missed: u32,
    duplicate: u32,
    reclaims: u32,
    attribute_change: u32,
}

// LogOperationStats contains statistics regarding the XFS log buffer
#[derive(Debug, Default)]
struct LogOperationStats {
    writes: u32,
    blocks: u32,
    no_internal_buffers: u32,
    force: u32,
    force_sleep: u32,
}

// ReadWriteStats contains statistics regarding the number of read
// and write system calls for XFS filesystems.
#[derive(Debug, Default)]
struct ReadWriteStats {
    write: u32,
    read: u32,
}

// VnodeStats contains statistics regarding XFS vnode operations
#[derive(Debug, Default)]
struct VnodeStats {
    active: u32,
    allocate: u32,
    get: u32,
    hold: u32,
    release: u32,
    reclaim: u32,
    remove: u32,
    free: u32,
}

#[derive(Debug, Default)]
struct ExtendedPrecisionStats {
    flush_bytes: u64,
    write_bytes: u64,
    read_bytes: u64,
}

/// Stats contains XFS filesystem runtime statistics, parsed from
/// /proc/fs/xfs/stat
///
/// The name and meanings of each statistics were taken from
/// http://xfs.org/index.php/Runtime_Stats and xfs_stats.h in the
/// Linux kernel source. Most counters are uint32 (same data types
/// used in xfs_stats.h), but some of the "extended precision stats"
/// are uint64s.
#[derive(Debug, Default)]
struct Stats {
    // The name of the filesystem used to source these statistics.
    // If empty, this indicates aggregated statistics for all XFS
    // filesystems on the host
    name: String,

    extent_allocation: ExtentAllocationStats,
    allocation_btree: BTreeStats,
    block_mapping: BlockMappingStats,
    block_map_btree: BTreeStats,
    directory_operation: DirectoryOperationStats,
    transaction: TransactionStats,
    inode_operation: InodeOperationStats,
    log_operation: LogOperationStats,
    read_write: ReadWriteStats,
    vnode: VnodeStats,

    extended_precision: ExtendedPrecisionStats,
    // not all statistics list
}

/// xfs_sys_stats retrieves XFS filesystem runtime statistics for each mounted
/// XFS filesystem. Only available on kernel 4.4+. On older kernels, an empty
/// vector will be returned.
async fn xfs_sys_stats(sys_path: PathBuf) -> Result<Vec<Stats>, Error> {
    let paths = glob::glob(&format!(
        "{}/fs/xfs/*/stats/stats",
        sys_path.to_string_lossy()
    ))
    .map_err(|err| Error::from(format!("glob xfs stats failed, {err}")))?;

    let mut stats = Vec::new();
    for ent in paths {
        match ent {
            Ok(path) => {
                let name = path
                    .iter()
                    .rev()
                    .nth(2)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();

                match parse_stat(path).await {
                    Ok(mut stat) => {
                        stat.name = name;
                        stats.push(stat)
                    }
                    Err(err) => {
                        warn!(
                            message = "parse xfs stat failed",
                            %err
                        );
                    }
                }
            }
            Err(err) => {
                warn!(message = "Iterate glob result failed", %err);
            }
        }
    }

    Ok(stats)
}

async fn parse_stat(path: PathBuf) -> Result<Stats, Error> {
    let data = std::fs::read_to_string(path)?;

    let mut stat = Stats::default();
    for line in data.lines() {
        let parts = line.split_ascii_whitespace().collect::<Vec<_>>();

        // Expact at least a string label and a single integer value, ex:
        // - abt 0
        // - rw 1 2
        if parts.len() < 2 {
            continue;
        }

        let label = parts[0];

        // Extended precision counters are uint64 values
        if label == "xpc" {
            let us = parse_u64s(&parts[1..]).map_err(Error::from)?;

            if us.len() != 3 {
                return Err("incorrect number of values for XFS extended precision stats".into());
            }

            stat.extended_precision.flush_bytes = us[0];
            stat.extended_precision.write_bytes = us[1];
            stat.extended_precision.read_bytes = us[2];

            continue;
        }

        // all other counters are u32 values
        let us = parse_u32s(&parts[1..]).map_err(Error::from)?;

        match label {
            "extent_alloc" => {
                if us.len() != 4 {
                    return Err("incorrect number of values for XFS extent allocation stats".into());
                }

                stat.extent_allocation.extents_allocated = us[0];
                stat.extent_allocation.blocks_allocated = us[1];
                stat.extent_allocation.extents_freed = us[2];
                stat.extent_allocation.blocks_freed = us[3];
            }
            "abt" => {
                if us.len() != 4 {
                    return Err("incorrect number of values for XFS btree stats".into());
                }

                stat.allocation_btree.lookups = us[0];
                stat.allocation_btree.compares = us[1];
                stat.allocation_btree.records_inserted = us[2];
                stat.allocation_btree.records_deleted = us[3];
            }
            "blk_map" => {
                if us.len() != 7 {
                    return Err("invalid number of values for XFS block mapping stats".into());
                }

                stat.block_mapping.reads = us[0];
                stat.block_mapping.writes = us[1];
                stat.block_mapping.unmaps = us[2];
                stat.block_mapping.extent_list_insertions = us[3];
                stat.block_mapping.extent_list_deletions = us[4];
                stat.block_mapping.extent_list_lookups = us[5];
                stat.block_mapping.extent_list_compares = us[6];
            }
            "bmbt" => {
                if us.len() != 4 {
                    return Err("invalid number of values for XFS BlockMapBTree stats".into());
                }

                stat.block_map_btree.lookups = us[0];
                stat.block_map_btree.compares = us[1];
                stat.block_map_btree.records_inserted = us[2];
                stat.block_map_btree.records_deleted = us[3];
            }
            "dir" => {
                if us.len() != 4 {
                    return Err(
                        "incorrect number of values for XFS directory operation stats".into(),
                    );
                }

                stat.directory_operation.lookups = us[0];
                stat.directory_operation.creates = us[1];
                stat.directory_operation.removes = us[2];
                stat.directory_operation.get_dents = us[3];
            }
            "trans" => {}
            "ig" => {
                if us.len() != 7 {
                    return Err("incorrect number of values for XFS inode operation stats".into());
                }

                stat.inode_operation.attempts = us[0];
                stat.inode_operation.found = us[1];
                stat.inode_operation.recycle = us[2];
                stat.inode_operation.missed = us[3];
                stat.inode_operation.duplicate = us[4];
                stat.inode_operation.reclaims = us[5];
                stat.inode_operation.attribute_change = us[6];
            }
            "log" => {}
            "push_ail" => {}
            "xstrat" => {}
            "rw" => {
                if us.len() != 2 {
                    return Err("incorrect number of values for XFS read write stats".into());
                }

                stat.read_write.write = us[0];
                stat.read_write.read = us[1];
            }
            "attr" => {}
            "icluster" => {}
            "vnodes" => {
                // The attribute "Free" appears to not be available on older XFS
                // stats versions. Therefore, 7 or 8 elements may appear in this slice
                let length = us.len();
                if length != 7 && length != 8 {
                    return Err(Error::from(
                        "incorrect number of values for XFS vnode stats",
                    ));
                }

                stat.vnode.active = us[0];
                stat.vnode.allocate = us[1];
                stat.vnode.get = us[2];
                stat.vnode.hold = us[3];
                stat.vnode.release = us[4];
                stat.vnode.reclaim = us[5];
                stat.vnode.remove = us[6];

                // Skip adding free, unless it is present. The zero value will be
                // used in place of an actual count.
                if length == 8 {
                    stat.vnode.free = us[7];
                }
            }
            "buf" => {}
            "xpc" => {}
            "abtb2" => {}
            "abtc2" => {}
            "bmbt2" => {}
            "ibt2" => {}
            _ => {}
        }
    }

    Ok(stat)
}

fn parse_u64s(ss: &[&str]) -> Result<Vec<u64>, ParseIntError> {
    let mut us = Vec::with_capacity(ss.len());

    for s in ss {
        let v = s.parse()?;
        us.push(v);
    }

    Ok(us)
}

fn parse_u32s(ss: &[&str]) -> Result<Vec<u32>, ParseIntError> {
    let mut us = Vec::with_capacity(ss.len());

    for s in ss {
        let v = s.parse()?;
        us.push(v);
    }

    Ok(us)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_xfs_sys_stats() {
        let sys_path = "tests/node/sys".into();
        let stats = xfs_sys_stats(sys_path).await.unwrap();
        assert_eq!(stats.len(), 2);

        assert_eq!(stats[0].name, "sda1");
        assert_eq!(stats[0].extent_allocation.extents_allocated, 1);

        assert_eq!(stats[1].name, "sdb1");
        assert_eq!(stats[1].extent_allocation.extents_allocated, 2);
    }
}
