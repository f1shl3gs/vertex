//! Exposes XFS runtime statistics
//!
//! Linux (kernel 4.4+)

use std::path::Path;

use event::{Metric, tags};

use super::{Error, Paths};

pub async fn collect(paths: Paths) -> Result<Vec<Metric>, Error> {
    let stats = load_xfs_sys_stats(paths.sys())?;

    let mut metrics = Vec::with_capacity(stats.len() * 39);
    for (device, stat) in stats {
        let tags = tags!("device" => device);

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
                stat.directory_operation.getdents,
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
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Default)]
struct ExtentAllocationStats {
    extents_allocated: u32,
    blocks_allocated: u32,
    extents_freed: u32,
    blocks_freed: u32,
}

// BtreeStats contains statistics regarding an XFS internal B-tree
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Default)]
struct BTreeStats {
    lookups: u32,
    compares: u32,
    records_inserted: u32,
    records_deleted: u32,
}

// BlockMappingStats contains statistics regarding XFS block maps
#[cfg_attr(test, derive(PartialEq))]
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
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Default)]
struct DirectoryOperationStats {
    lookups: u32,
    creates: u32,
    removes: u32,
    getdents: u32,
}

// TransactionStats contains statistics regarding XFS metadata transactions
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Default)]
struct TransactionStats {
    synchronous: u32,
    asynchronous: u32,
    empty: u32,
}

// InodeOperationStats contains statistics regarding XFS inode operations
#[cfg_attr(test, derive(PartialEq))]
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

// // LogOperationStats contains statistics regarding the XFS log buffer
// #[derive(Debug, Default)]
// struct LogOperationStats {
//     writes: u32,
//     blocks: u32,
//     no_internal_buffers: u32,
//     force: u32,
//     force_sleep: u32,
// }

// ReadWriteStats contains statistics regarding the number of read
// and write system calls for XFS filesystems.
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Default)]
struct ReadWriteStats {
    write: u32,
    read: u32,
}

// VnodeStats contains statistics regarding XFS vnode operations
#[cfg_attr(test, derive(PartialEq))]
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

// ExtendedPrecisionStats contains high precision counters used to track the
// total number of bytes read, written, or flushed, during XFS operations.
#[cfg_attr(test, derive(PartialEq))]
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
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, Default)]
struct Stats {
    extent_allocation: ExtentAllocationStats,
    allocation_btree: BTreeStats,
    block_mapping: BlockMappingStats,
    block_map_btree: BTreeStats,
    directory_operation: DirectoryOperationStats,
    transaction: TransactionStats,
    inode_operation: InodeOperationStats,
    // log_operation: LogOperationStats,
    read_write: ReadWriteStats,
    vnode: VnodeStats,

    extended_precision: ExtendedPrecisionStats,
    // not all statistics list
}

/// xfs_sys_stats retrieves XFS filesystem runtime statistics for each mounted
/// XFS filesystem. Only available on kernel 4.4+. On older kernels, an empty
/// vector will be returned.
fn load_xfs_sys_stats(root: &Path) -> Result<Vec<(String, Stats)>, Error> {
    let paths = glob::glob(&format!("{}/fs/xfs/*/stats/stats", root.to_string_lossy()))?;

    let mut stats = Vec::new();
    for path in paths.flatten() {
        match parse_stat(&path) {
            Ok(stat) => {
                let name = path
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();

                stats.push((name, stat))
            }
            Err(err) => {
                warn!(
                    message = "parse xfs stat failed",
                    %err
                );
            }
        }
    }

    Ok(stats)
}

fn parse_stat(path: &Path) -> Result<Stats, Error> {
    let content = std::fs::read_to_string(path)?;

    let mut stat = Stats::default();
    for line in content.lines() {
        let mut parts = line.split_ascii_whitespace();
        let Some(label) = parts.next() else {
            continue;
        };

        match label {
            "extent_alloc" => {
                let values = parts
                    .take(4)
                    .map(|x| x.parse::<u32>())
                    .collect::<Result<Vec<_>, _>>()?;

                if values.len() != 4 {
                    return Err(Error::Malformed("extent_alloc line of stat file"));
                }

                stat.extent_allocation.extents_allocated = values[0];
                stat.extent_allocation.blocks_allocated = values[1];
                stat.extent_allocation.extents_freed = values[2];
                stat.extent_allocation.blocks_freed = values[3];
            }
            "abt" => {
                let values = parts
                    .take(4)
                    .map(|x| x.parse::<u32>())
                    .collect::<Result<Vec<_>, _>>()?;

                if values.len() != 4 {
                    return Err(Error::Malformed("abt line of stat file"));
                }

                stat.allocation_btree.lookups = values[0];
                stat.allocation_btree.compares = values[1];
                stat.allocation_btree.records_inserted = values[2];
                stat.allocation_btree.records_deleted = values[3];
            }
            "blk_map" => {
                let values = parts
                    .take(7)
                    .map(|x| x.parse::<u32>())
                    .collect::<Result<Vec<_>, _>>()?;

                if values.len() != 7 {
                    return Err(Error::Malformed("blk_map line of stat file"));
                }

                stat.block_mapping.reads = values[0];
                stat.block_mapping.writes = values[1];
                stat.block_mapping.unmaps = values[2];
                stat.block_mapping.extent_list_insertions = values[3];
                stat.block_mapping.extent_list_deletions = values[4];
                stat.block_mapping.extent_list_lookups = values[5];
                stat.block_mapping.extent_list_compares = values[6];
            }
            "bmbt" => {
                let values = parts
                    .take(4)
                    .map(|x| x.parse::<u32>())
                    .collect::<Result<Vec<_>, _>>()?;

                if values.len() != 4 {
                    return Err(Error::Malformed("bmbt line of stat file"));
                }

                stat.block_map_btree.lookups = values[0];
                stat.block_map_btree.compares = values[1];
                stat.block_map_btree.records_inserted = values[2];
                stat.block_map_btree.records_deleted = values[3];
            }
            "dir" => {
                let values = parts
                    .take(4)
                    .map(|x| x.parse::<u32>())
                    .collect::<Result<Vec<_>, _>>()?;

                if values.len() != 4 {
                    return Err(Error::Malformed("dir line of stat file"));
                }

                stat.directory_operation.lookups = values[0];
                stat.directory_operation.creates = values[1];
                stat.directory_operation.removes = values[2];
                stat.directory_operation.getdents = values[3];
            }
            "trans" => {
                let values = parts
                    .take(3)
                    .map(|x| x.parse::<u32>())
                    .collect::<Result<Vec<_>, _>>()?;

                if values.len() != 3 {
                    return Err(Error::Malformed("trans line of stat file"));
                }

                stat.transaction.synchronous = values[0];
                stat.transaction.asynchronous = values[1];
                stat.transaction.empty = values[2];
            }
            "ig" => {
                let values = parts
                    .take(7)
                    .map(|x| x.parse::<u32>())
                    .collect::<Result<Vec<_>, _>>()?;

                if values.len() != 7 {
                    return Err(Error::Malformed("trans line of stat file"));
                }

                stat.inode_operation.attempts = values[0];
                stat.inode_operation.found = values[1];
                stat.inode_operation.recycle = values[2];
                stat.inode_operation.missed = values[3];
                stat.inode_operation.duplicate = values[4];
                stat.inode_operation.reclaims = values[5];
                stat.inode_operation.attribute_change = values[6];
            }
            "xpc" => {
                let values = parts
                    .take(3)
                    .map(|x| x.parse::<u64>())
                    .collect::<Result<Vec<_>, _>>()?;

                // take(3) make sure of that the values's max length is 3
                if values.len() != 3 {
                    return Err(Error::Malformed("xpc line of stat file"));
                }

                stat.extended_precision.flush_bytes = values[0];
                stat.extended_precision.write_bytes = values[1];
                stat.extended_precision.read_bytes = values[2];
            }
            "rw" => {
                let values = parts
                    .take(2)
                    .map(|x| x.parse::<u32>())
                    .collect::<Result<Vec<_>, _>>()?;

                if values.len() != 2 {
                    return Err(Error::Malformed("rw line of stat file"));
                }

                stat.read_write.write = values[0];
                stat.read_write.read = values[1];
            }
            "vnodes" => {
                let values = parts
                    .take(8)
                    .map(|x| x.parse::<u32>())
                    .collect::<Result<Vec<_>, _>>()?;

                if values.len() < 7 {
                    return Err(Error::Malformed("vnodes line of stat file"));
                }

                stat.vnode.active = values[0];
                stat.vnode.allocate = values[1];
                stat.vnode.get = values[2];
                stat.vnode.hold = values[3];
                stat.vnode.release = values[4];
                stat.vnode.reclaim = values[5];
                stat.vnode.remove = values[6];

                if values.len() == 8 {
                    stat.vnode.free = values[7];
                }
            }
            _ => {}
        }
    }

    Ok(stat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_stat() {
        let path = Path::new("tests/node/fixtures/proc/fs/xfs/stat");
        let stat = parse_stat(path).unwrap();
        assert_eq!(stat.extent_allocation.extents_allocated, 92447);
    }

    #[test]
    fn sys_stats() {
        let sys_path = Path::new("tests/node/fixtures/sys");
        let array = load_xfs_sys_stats(sys_path).unwrap();
        assert_eq!(array.len(), 2);

        let (name, stats) = &array[0];
        assert_eq!(name, "sda1");
        assert_eq!(stats.extent_allocation.extents_allocated, 1);

        let (name, stats) = &array[1];
        assert_eq!(name, "sdb1");
        assert_eq!(stats.extent_allocation.extents_allocated, 2);
    }

    #[test]
    fn ok() {
        let want = Stats {
            extent_allocation: ExtentAllocationStats {
                extents_allocated: 92447,
                blocks_allocated: 97589,
                extents_freed: 92448,
                blocks_freed: 93751,
            },
            allocation_btree: BTreeStats {
                lookups: 0,
                compares: 0,
                records_inserted: 0,
                records_deleted: 0,
            },
            block_mapping: BlockMappingStats {
                reads: 1767055,
                writes: 188820,
                unmaps: 184891,
                extent_list_insertions: 92447,
                extent_list_deletions: 92448,
                extent_list_lookups: 2140766,
                extent_list_compares: 0,
            },
            block_map_btree: BTreeStats {
                lookups: 0,
                compares: 0,
                records_inserted: 0,
                records_deleted: 0,
            },
            directory_operation: DirectoryOperationStats {
                lookups: 185039,
                creates: 92447,
                removes: 92444,
                getdents: 136422,
            },
            transaction: TransactionStats {
                synchronous: 706,
                asynchronous: 944304,
                empty: 0,
            },
            inode_operation: InodeOperationStats {
                attempts: 185045,
                found: 58807,
                recycle: 0,
                missed: 126238,
                duplicate: 0,
                reclaims: 33637,
                attribute_change: 22,
            },
            // log_operation: LogOperationStats {
            //     writes: 2883,
            //     blocks: 113448,
            //     no_internal_buffers: 9,
            //     force: 17360,
            //     force_sleep: 739,
            // },
            read_write: ReadWriteStats {
                write: 107739,
                read: 94045,
            },
            // attribute_operation: AttributeOperationStats {
            //     Get: 4,
            //     Set: 0,
            //     Remove: 0,
            //     List: 0,
            // },
            // inode_clustering: InodeClusteringStats {
            //     Iflush: 8677,
            //     Flush: 7849,
            //     FlushInode: 135802,
            // },
            vnode: VnodeStats {
                active: 92601,
                allocate: 0,
                get: 0,
                hold: 0,
                release: 92444,
                reclaim: 92444,
                remove: 92444,
                free: 0,
            },
            // buffer: BufferStats {
            //     Get: 2666287,
            //     Create: 7122,
            //     GetLocked: 2659202,
            //     GetLockedWaited: 3599,
            //     BusyLocked: 2,
            //     MissLocked: 7085,
            //     PageRetries: 0,
            //     PageFound: 10297,
            //     GetRead: 7085,
            // },
            extended_precision: ExtendedPrecisionStats {
                flush_bytes: 399724544,
                write_bytes: 92823103,
                read_bytes: 86219234,
            },
            // push_ail: PushAilStats {
            //     TryLogspace: 945014,
            //     SleepLogspace: 0,
            //     Pushes: 134260,
            //     Success: 15483,
            //     PushBuf: 0,
            //     Pinned: 3940,
            //     Locked: 464,
            //     Flushing: 159985,
            //     Restarts: 0,
            //     Flush: 40,
            // },
            // xstrat: XstratStats {
            //     Quick: 92447,
            //     Split: 0,
            // },
            // Debug: DebugStats { Enabled: 0 },
            // QuotaManager: QuotaManagerStats {
            //     Reclaims: 0,
            //     ReclaimMisses: 0,
            //     DquoteDups: 0,
            //     CacheMisses: 0,
            //     CacheHits: 0,
            //     Wants: 0,
            //     ShakeReclaims: 0,
            //     InactReclaims: 0,
            // },
            // BtreeAllocBlocks2: BtreeAllocBlocks2Stats {
            //     Lookup: 184941,
            //     Compare: 1277345,
            //     Insrec: 13257,
            //     Delrec: 13278,
            //     NewRoot: 0,
            //     KillRoot: 0,
            //     Increment: 0,
            //     Decrement: 0,
            //     Lshift: 0,
            //     Rshift: 0,
            //     Split: 0,
            //     Join: 0,
            //     Alloc: 0,
            //     Free: 0,
            //     Moves: 2746147,
            // },
            // BtreeAllocContig2: BtreeAllocContig2Stats {
            //     Lookup: 345295,
            //     Compare: 2416764,
            //     Insrec: 172637,
            //     Delrec: 172658,
            //     NewRoot: 0,
            //     KillRoot: 0,
            //     Increment: 0,
            //     Decrement: 0,
            //     Lshift: 0,
            //     Rshift: 0,
            //     Split: 0,
            //     Join: 0,
            //     Alloc: 0,
            //     Free: 0,
            //     Moves: 21406023,
            // },
            // BtreeBlockMap2: BtreeBlockMap2Stats {
            //     Lookup: 0,
            //     Compare: 0,
            //     Insrec: 0,
            //     Delrec: 0,
            //     NewRoot: 0,
            //     KillRoot: 0,
            //     Increment: 0,
            //     Decrement: 0,
            //     Lshift: 0,
            //     Rshift: 0,
            //     Split: 0,
            //     Join: 0,
            //     Alloc: 0,
            //     Free: 0,
            //     Moves: 0,
            // },
            // BtreeInode2: BtreeInode2Stats {
            //     Lookup: 343004,
            //     Compare: 1358467,
            //     Insrec: 0,
            //     Delrec: 0,
            //     NewRoot: 0,
            //     KillRoot: 0,
            //     Increment: 0,
            //     Decrement: 0,
            //     Lshift: 0,
            //     Rshift: 0,
            //     Split: 0,
            //     Join: 0,
            //     Alloc: 0,
            //     Free: 0,
            //     Moves: 0,
            // },
        };

        let path = Path::new("tests/node/fixtures/proc/fs/xfs/stat");
        let got = parse_stat(path).unwrap();

        assert_eq!(want, got)
    }
}
