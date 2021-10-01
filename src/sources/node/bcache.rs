use serde::{Deserialize, Serialize};
use crate::event::Metric;
use crate::sources::node::errors::Error;

#[derive(Default, Deserialize, Serialize)]
pub struct BCacheConfig {
    priority_stats: bool,
}

pub async fn gather(sys_path: &str, conf: &BCacheConfig) -> Result<Vec<Metric>, Error> {
    todo!()
}

/// Stats contains bcache runtime statistics, parsed from /sys/fs/bcache/.
///
/// The names and meanings of each statistic were taken from bcache.txt and
/// files in drivers/md/bcache in the Linux kernel source. Counters are
/// u64 (in-kernel counters are mostly unsigned long)
struct Stats {
    // The name of the bcache used to source these statistics
    name: String,
    bcache: BcacheStats,
    bdevs: Vec<BDevStats>,
    caches: Vec<CacheStats>,
}

/// BcacheStats contains statistics tied to a bcache ID
struct BcacheStats {
    average_key_size: u64,
    btree_cache_size: u64,
    cache_available_percent: u64,
    congested: u64,
    root_usage_percent: u64,
    tree_depth: u64,
    internal: InternalStats,
    five_min: PeriodStats,
    total: PeriodStats,
}

/// BDevStats contains statistics for one backing device
struct BDevStats {
    name: String,
    dirty_data: u64,
    five_min: PeriodStats,
    total: PeriodStats,
    writeback_rate_debug: WritebackRateDebugStats,
}

/// CacheStats contains statistics for one cache device
struct CacheStats {
    name: String,
    io_errors: u64,
    metadata_written: u64,
    written: u64,
    priority: PriorityStats,
}

/// PriorityStats contains statistics from the priority_stats file
struct PriorityStats {
    unused_percent: u64,
    metadata_percent: u64,
}

/// InternalStats contains internal bcache statistics.
struct InternalStats {
    active_journal_entries: u64,
    btree_nodes: u64,
    btree_read_average_duration_nano_seconds: u64,
    cache_read_races: u64,
}

/// PeriodStats contains statistics for a time period (5m or total)
struct PeriodStats {
    bypassed: u64,
    cache_bypass_hits: u64,
    cache_bypass_misses: u64,
    cache_hits: u64,
    cache_miss_collisions: u64,
    cache_misses: u64,
    cache_readaheads: u64,
}

struct WritebackRateDebugStats {
    rate: u64,
    dirty: u64,
    target: u64,
    proportional: i64,
    integral: i64,
    change: i64,
    next_io: i64,
}