use std::path::PathBuf;
use crate::btrfs::Stats;
use crate::{Error, read_into, SysFS};

/// InternalStats contains internal bcache statistics
pub struct InternalStats {
    active_journal_entries: u64,
    btree_nodes: u64,
    btree_read_average_duration_nano_seconds: u64,
    cache_read_races: u64,
}

/// PeriodStats contains statistics for a time period (5 min or total)
pub struct PeriodStats {
    bypassed: u64,
    cache_bypass_hits: u64,
    cache_bypass_misses: u64,
    cache_hits: u64,
    cache_miss_collisions: u64,
    cache_misses: u64,
    cache_readaheads: u64
}

/// BcacheStats contains bcache runtime statistics, parsed from /sys/fs/bcache/.
///
/// The names and meanings of each statistic were taken from bcache.txt and
/// files in drivers/md/bcache in the Linux kernel source. Counters are uint64
/// (in-kernel counters are mostly unsigned long).
#[derive(Default)]
pub struct BcacheStats {
    // The name of the bcache used to source these statistics
    name: String,

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

impl SysFS {
    pub async fn bcache(&self) -> Result<Vec<Stats>, Error> {
        let path = self.root.join("fs/bcache");
        let mut dirs = tokio::fs::read_dir(path).await?;
        while let Some(entry) = dirs.next_entry().await? {
            let name = entry.file_name();
            if !name.contains_byte(b'-') {
                continue;
            }

            let name = name.to_string_lossy().to_string();

            // stats


        }

        Ok(vec![])
    }
}

async fn read_stats(root: PathBuf) -> Result<BcacheStats, Error> {
    let mut bs = BcacheStats::default();

    // dir uuid
    bs.average_key_size = read_into(root.join("average_key_size")).await?;
    bs.btree_cache_size = read_into(root.join("btree_cache_size")).await?;
    bs.cache_available_percent = read_into(root.join("cache_available_percent")).await?;
    bs.congested = read_into(root.join("congested")).await?;
    bs.root_usage_percent = read_into(root.join("root_usage_percent")).await?;
    bs.tree_depth = read_into(root.join("tree_depth")).await?;

    // dir internal
    bs.internal.active_journal_entries = read_into(root.join("internal/active_journal_entries")).await?;
    bs.internal.btree_nodes = read_into(root.join("internal/btree_nodes")).await?;
    bs.internal.btree_read_average_duration_nano_seconds = read_into(root.join("internal/btree_read_average_duration_us")).await?;
    bs.internal.cache_read_races = read_into(root.join("internal/cache_read_races")).await?;

    // dir stats_five_minute
    bs.five_min.bypassed = read_into(root.join("stats_five_minute/bypassed")).await?;
    bs.five_min.cache_hits = read_into(root.join("stats_five_minute/cache_hits")).await?;
    bs.five_min.cache_bypass_misses = read_into(root.join("stats_five_minute/cache_bypass_hits")).await?;
    bs.five_min.cache_bypass_hits = read_into(root.join("stats_five_minute/cache_bypass_hits")).await?;
    bs.five_min.cache_miss_collisions = read_into(root.join("stats_five_minute/cache_miss_collisions")).await?;
    bs.five_min.cache_misses = read_into(root.join("stats_five_minute/cache_misses")).await?;
    bs.five_min.cache_readaheads = read_into(root.join("stats_five_minute/cache_readaheads")).await?;

    // dir stats_total
    bs.total.bypassed = read_into(root.join("stats_total/bypassed")).await?;
    bs.total.cache_hits = read_into(root.join("stats_total/cache_hits")).await?;
    bs.total.cache_bypass_misses = read_into(root.join("stats_total/cache_bypass_hits")).await?;
    bs.total.cache_bypass_hits = read_into(root.join("stats_total/cache_bypass_hits")).await?;
    bs.total.cache_miss_collisions = read_into(root.join("stats_total/cache_miss_collisions")).await?;
    bs.total.cache_misses = read_into(root.join("stats_total/cache_misses")).await?;
    bs.total.cache_readaheads = read_into(root.join("stats_total/cache_readaheads")).await?;

    // bdev stats
    // TODO: do we need impl bdev stats

    Ok(bs)
}
