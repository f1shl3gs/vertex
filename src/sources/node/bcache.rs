use std::io::ErrorKind;
use std::path::PathBuf;

use configurable::Configurable;
use event::{Metric, tags, tags::Key};
use serde::{Deserialize, Serialize};

use super::{Error, read_string};

#[derive(Clone, Configurable, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    /// Expose expensive priority stats
    priority_stats: bool,
}

pub async fn gather(conf: Config, sys_path: PathBuf) -> Result<Vec<Metric>, Error> {
    let stats = bcache_stats(sys_path, conf.priority_stats)?;
    let mut metrics = vec![];

    for stat in stats {
        metrics.extend([
            // metrics under /sys/fs/bcache/<uuid>
            Metric::gauge_with_tags(
                "node_bcache_average_key_size_sectors",
                "Average data per key in the btree (sectors).",
                stat.bcache.average_key_size,
                tags!(
                    "uuid" => stat.name.clone()
                )
            ),
            Metric::gauge_with_tags(
                "node_bcache_btree_cache_size_bytes",
                "Amount of memory currently used by the btree cache.",
                stat.bcache.btree_cache_size,
                tags!(
                    "uuid" => stat.name.clone()
                )
            ),
            Metric::gauge_with_tags(
                "node_bcache_cache_available_percent",
                "Percentage of cache device without dirty data, usable for writeback (may contain clean cached data).",
                stat.bcache.cache_available_percent,
                tags!(
                    "uuid" => stat.name.clone()
                )
            ),
            Metric::gauge_with_tags(
                "node_bcache_congested",
                "Congestion",
                stat.bcache.congested,
                tags!(
                    "uuid" => stat.name.clone()
                )
            ),
            Metric::gauge_with_tags(
                "node_bcache_root_usage_percent",
                "Percentage of the root btree node in use (tree depth increases if too high).",
                stat.bcache.root_usage_percent,
                tags!(
                    "uuid" => stat.name.clone()
                )
            ),
            Metric::gauge_with_tags(
                "node_bcache_tree_depth",
                "Depth of the btree.",
                stat.bcache.tree_depth,
                tags!(
                    "uuid" => stat.name.clone()
                )
            ),
            // metrics under /sys/fs/bcache/<uuid>/internal
            Metric::gauge_with_tags(
                "node_bcache_active_journal_entries",
                "Number of journal entries that are newer than the index.",
                stat.bcache.internal.active_journal_entries,
                tags!(
                    "uuid" => stat.name.clone()
                )
            ),
            Metric::gauge_with_tags(
                "node_bcache_btree_nodes",
                "Total nodes in the btree.",
                stat.bcache.internal.btree_nodes,
                tags!(
                    "uuid" => stat.name.clone()
                )
            ),
            Metric::gauge_with_tags(
                "node_bcache_btree_read_average_duration_seconds",
                "Average btree read duration.",
                stat.bcache.internal.btree_read_average_duration_us as f64 * 1e-9,
                tags!(
                    "uuid" => stat.name.clone()
                )
            ),
            Metric::sum_with_tags(
                "node_bcache_cache_read_races_total",
                "Counts instances where while data was being read from the cache, the bucket was reused and invalidated - i.e. where the pointer was stale after the read completed.",
                stat.bcache.internal.cache_read_races,
                tags!(
                    "uuid" => stat.name.clone()
                )
            )
        ]);

        for bdev in stat.bdevs {
            let tags = tags!(
                Key::from_static("backing_device") => bdev.name,
                Key::from_static("uuid") => stat.name.clone()
            );

            // metrics in /sys/fs/bcache/<uuid>/<bdev>/
            metrics.extend([
                Metric::gauge_with_tags(
                    "node_bcache_dirty_data_bytes",
                    "Amount of dirty data for this backing device in the cache.",
                    bdev.dirty_data,
                    tags.clone(),
                ),
                Metric::gauge_with_tags(
                    "node_bcache_dirty_target_bytes",
                    "Current dirty data target threshold for this backing device in bytes.",
                    bdev.writeback_rate_debug.target,
                    tags.clone(),
                ),
                Metric::gauge_with_tags(
                    "node_bcache_writeback_rate",
                    "Current writeback rate for this backing device in bytes.",
                    bdev.writeback_rate_debug.rate,
                    tags.clone(),
                ),
                Metric::gauge_with_tags(
                    "node_bcache_writeback_rate_proportional_term",
                    "Current result of proportional controller, part of writeback rate",
                    bdev.writeback_rate_debug.proportional,
                    tags.clone(),
                ),
                Metric::gauge_with_tags(
                    "node_bcache_writeback_rate_integral_term",
                    "Current result of integral controller, part of writeback rate",
                    bdev.writeback_rate_debug.integral,
                    tags.clone(),
                ),
                Metric::gauge_with_tags(
                    "node_bcache_writeback_change",
                    "Last writeback rate change step for this backing device.",
                    bdev.writeback_rate_debug.change,
                    tags.clone(),
                ),
            ]);

            // metrics under /sys/fs/bcache/<uuid>/<bdev>/stats_total
            metrics.extend([
                Metric::sum_with_tags(
                    "node_bcache_bypassed_bytes_total",
                    "Amount of IO (both reads and writes) that has bypassed the cache.",
                    bdev.total.bypassed,
                    tags.clone()
                ),
                Metric::sum_with_tags(
                    "node_bcache_cache_hits_total",
                    "Hits counted per individual IO as bcache sees them.",
                    bdev.total.cache_hits,
                    tags.clone()
                ),
                Metric::sum_with_tags(
                    "node_bcache_cache_misses_total",
                    "Misses counted per individual IO as bcache sees them.",
                    bdev.total.cache_misses,
                    tags.clone()
                ),
                Metric::sum_with_tags(
                    "node_bcache_cache_bypass_hits_total",
                    "Hits for IO intended to skip the cache.",
                    bdev.total.cache_bypass_hits,
                    tags.clone()
                ),
                Metric::sum_with_tags(
                    "node_bcache_cache_bypass_misses_total",
                    "Misses for IO intended to skip the cache.",
                    bdev.total.cache_bypass_misses,
                    tags.clone(),
                ),
                Metric::sum_with_tags(
                    "node_bcache_cache_miss_collisions_total",
                    "Instances where data insertion from cache miss raced with write (data already present).",
                    bdev.total.cache_miss_collisions,
                    tags.clone()
                )
            ]);

            if bdev.total.cache_readaheads != 0 {
                metrics.push(Metric::sum_with_tags(
                    "node_bcache_cache_readaheads_total",
                    "Count of times readahead occurred.",
                    bdev.total.cache_readaheads,
                    tags,
                ))
            }
        }

        for cache in stat.caches {
            let tags = tags!(
                Key::from_static("cache_device") => cache.name,
                Key::from_static("uuid") => stat.name.clone(),
            );

            // metrics under /sys/fs/bcache/<uuid>/<cache>
            metrics.extend([
                Metric::gauge_with_tags(
                    "node_bcache_io_errors",
                    "Number of errors that have occurred, decayed by io_error_halflife.",
                    cache.io_errors,
                    tags.clone(),
                ),
                Metric::sum_with_tags(
                    "node_bcache_metadata_written_bytes_total",
                    "Sum of all non data writes (btree writes and all other metadata).",
                    cache.metadata_written,
                    tags.clone(),
                ),
                Metric::sum_with_tags(
                    "node_bcache_written_bytes_total",
                    "Sum of all data that has been written to the cache.",
                    cache.written,
                    tags.clone(),
                ),
            ]);

            // metrics in /sys/fs/bcache/<uuid>/<cache>/priority_stats
            if conf.priority_stats {
                metrics.extend([
                    Metric::gauge_with_tags(
                        "node_bcache_priority_stats_unused_percent",
                        "The percentage of the cache that doesn't contain any data.",
                        cache.priority.unused_percent,
                        tags.clone(),
                    ),
                    Metric::gauge_with_tags(
                        "node_bcache_priority_stats_metadata_percent",
                        "Bcache's metadata overhead.",
                        cache.priority.metadata_percent,
                        tags,
                    ),
                ]);
            }
        }
    }

    Ok(metrics)
}

fn bcache_stats(sys_path: PathBuf, priority_stats: bool) -> Result<Vec<Stat>, Error> {
    let paths = glob::glob(&format!("{}/fs/bcache/*-*", sys_path.to_string_lossy()))?;

    let mut stats = vec![];
    for path in paths.flatten() {
        stats.push(get_stats(path, priority_stats)?)
    }

    Ok(stats)
}

fn get_stats(root: PathBuf, priority_stats: bool) -> Result<Stat, Error> {
    let name = root
        .file_name()
        .expect("filename must exist")
        .to_string_lossy()
        .to_string();

    // bcache stats
    let average_key_size = read_value(root.join("average_key_size"))?;
    let btree_cache_size = read_value(root.join("btree_cache_size"))?;
    let cache_available_percent = read_value(root.join("cache_available_percent"))?;
    let congested = read_value(root.join("congested"))?;
    let root_usage_percent = read_value(root.join("root_usage_percent"))?;
    let tree_depth = read_value(root.join("tree_depth"))?;

    // bcache internal
    let path = root.join("internal");
    let active_journal_entries = read_value(path.join("active_journal_entries"))?;
    let btree_nodes = read_value(path.join("btree_nodes"))?;
    let btree_read_average_duration_us = read_value(path.join("btree_read_average_duration_us"))?;
    let cache_read_races = read_value(path.join("cache_read_races"))?;
    let internal = InternalStats {
        active_journal_entries,
        btree_nodes,
        btree_read_average_duration_us,
        cache_read_races,
    };

    // bcache five_minute
    let five_min = read_period_stats(root.join("stats_five_minute"))?;

    // bcache total
    let total = read_period_stats(root.join("total"))?;

    // bdev stats
    let paths = glob::glob(&format!("{}/bdev[0-9]*", root.to_string_lossy()))?;
    let mut bdevs = vec![];
    for path in paths.flatten() {
        let name = path
            .file_name()
            .expect("must exist")
            .to_string_lossy()
            .to_string();

        let dirty_data = read_value(path.join("dirty_data"))?;
        let five_min = read_period_stats(path.join("stats_five_minute"))?;
        let total = read_period_stats(path.join("stats_total"))?;
        let writeback_rate_debug = read_writeback_rate_debug(path.join("writeback_rate_debug"))?;

        bdevs.push(BdevStats {
            name,
            dirty_data,
            five_min,
            total,
            writeback_rate_debug,
        });
    }

    // cache stats
    let mut caches = vec![];
    let paths = glob::glob(&format!("{}/cache[0-9]*", root.to_string_lossy()))?;
    for path in paths.flatten() {
        let name = path
            .file_name()
            .expect("must exist")
            .to_string_lossy()
            .to_string();

        let io_errors = read_value(path.join("io_errors"))?;
        let metadata_written = read_value(path.join("metadata_written"))?;
        let written = read_value(path.join("written"))?;
        let priority = if priority_stats {
            read_priority_stats(path.join("priority_stats"))?
        } else {
            PriorityStats::default()
        };

        caches.push(CacheStats {
            name,
            io_errors,
            metadata_written,
            written,
            priority,
        })
    }

    Ok(Stat {
        name,
        bcache: BcacheStats {
            average_key_size,
            btree_cache_size,
            cache_available_percent,
            congested,
            root_usage_percent,
            tree_depth,
            internal,
            five_min,
            total,
        },
        bdevs,
        caches,
    })
}

/// InternalStats contains internal bcache statistics.
#[derive(Debug)]
struct InternalStats {
    active_journal_entries: u64,
    btree_nodes: u64,
    btree_read_average_duration_us: u64,
    cache_read_races: u64,
}

/// PeriodStats contains statistics for a time period (5 min or total).
#[derive(Debug)]
struct PeriodStats {
    bypassed: u64,
    cache_bypass_hits: u64,
    cache_bypass_misses: u64,
    cache_hits: u64,
    cache_miss_collisions: u64,
    cache_misses: u64,
    cache_readaheads: u64,
}

fn read_period_stats(path: PathBuf) -> Result<PeriodStats, Error> {
    let bypassed = read_value(path.join("bypassed"))?;
    let cache_bypass_hits = read_value(path.join("cache_bypass_hits"))?;
    let cache_bypass_misses = read_value(path.join("cache_bypass_misses"))?;
    let cache_hits = read_value(path.join("cache_hits"))?;
    let cache_miss_collisions = read_value(path.join("cache_miss_collisions"))?;
    let cache_misses = read_value(path.join("cache_misses"))?;
    let cache_readaheads = read_value(path.join("cache_readaheads"))?;

    Ok(PeriodStats {
        bypassed,
        cache_bypass_hits,
        cache_bypass_misses,
        cache_hits,
        cache_miss_collisions,
        cache_misses,
        cache_readaheads,
    })
}

/// `BcacheStats` contains statistics tied to a bcache ID.
#[derive(Debug)]
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

/// `WritebackRateDebugStats` contains bcache writeback statistics.
#[derive(Debug, Default)]
struct WritebackRateDebugStats {
    rate: u64,
    dirty: u64,
    target: u64,
    proportional: i64,
    integral: i64,
    change: i64,
    next_io: i64,
}

fn read_writeback_rate_debug(path: PathBuf) -> Result<WritebackRateDebugStats, Error> {
    let data = std::fs::read_to_string(path)?;

    let mut stats = WritebackRateDebugStats::default();
    for line in data.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };

        let value = value.trim();
        match key {
            "rate" => {
                let Some(stripped) = value.strip_suffix("/sec") else {
                    continue;
                };

                stats.rate = parse_bytes(stripped).map_err(|_| Error::Malformed("rate"))?;
            }
            "dirty" => {
                stats.dirty = parse_bytes(value).map_err(|_| Error::Malformed("dirty"))?;
            }
            "target" => {
                stats.target = parse_bytes(value).map_err(|_| Error::Malformed("target"))?;
            }
            "proportional" => {
                stats.proportional =
                    parse_bytes(value).map_err(|_| Error::Malformed("proportional"))? as i64;
            }
            "integral" => {
                stats.integral =
                    parse_bytes(value).map_err(|_| Error::Malformed("integral"))? as i64;
            }
            "change" => {
                let Some(stripped) = value.strip_suffix("/sec") else {
                    continue;
                };

                stats.change =
                    parse_bytes(stripped).map_err(|_| Error::Malformed("change"))? as i64;
            }
            "next io" => {
                let value = value.strip_suffix("ms").expect("ms must exist");
                stats.next_io = value.parse()?;
            }
            _ => {}
        }
    }

    Ok(stats)
}

/// `BdevStats` contains statistics for one backing device.
#[derive(Debug)]
struct BdevStats {
    name: String,
    dirty_data: u64,
    five_min: PeriodStats,
    total: PeriodStats,
    writeback_rate_debug: WritebackRateDebugStats,
}

/// `PriorityStats` contains statistics from the priority_stats file.
#[derive(Debug, Default)]
struct PriorityStats {
    unused_percent: u64,
    metadata_percent: u64,
}

// example content
//
// Unused:		99%
// Metadata:	0%
// Average:	10473
// Sectors per Q:	64
// Quantiles:	[0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 20946 20946 20946 20946 20946 20946 20946 20946 20946 20946 20946 20946 20946 20946 20946 20946]
fn read_priority_stats(path: PathBuf) -> Result<PriorityStats, Error> {
    let data = std::fs::read_to_string(path)?;

    let mut stats = PriorityStats::default();
    for line in data.lines() {
        if let Some(value) = line.strip_prefix("Unused:") {
            if let Some(text) = value.trim().strip_suffix('%') {
                stats.unused_percent = text.parse()?;
            }
            continue;
        }

        if let Some(value) = line.strip_prefix("Metadata:") {
            if let Some(text) = value.trim().strip_suffix('%') {
                stats.metadata_percent = text.parse()?;
            }
            continue;
        }
    }

    Ok(stats)
}

/// `CacheStats` contains statistics for one cache device.
#[derive(Debug)]
struct CacheStats {
    name: String,
    io_errors: u64,
    metadata_written: u64,
    written: u64,
    priority: PriorityStats,
}

/// `Stat` contains bcache runtime statistics, parsed from /sys/fs/bcache
///
/// The names and meanings of each statistic were taken from bcache.txt
/// and files in drivers/md/bcache in the Linux kernel source. Counters
/// are u64 (in-kernel counters are mostly unsigned long).
#[derive(Debug)]
struct Stat {
    // The name of the bcache used to source these statistics.
    name: String,
    bcache: BcacheStats,
    bdevs: Vec<BdevStats>,
    caches: Vec<CacheStats>,
}

fn read_value(path: PathBuf) -> Result<u64, Error> {
    match read_string(path) {
        Ok(content) => {
            let value = content.parse()?;
            Ok(value)
        }
        Err(err) => {
            if err.kind() == ErrorKind::NotFound {
                return Ok(0);
            }

            Err(err.into())
        }
    }
}

// parse_bytes converts a human-readable byte slice into an u64
fn parse_bytes(input: &str) -> Result<u64, ()> {
    let len = input.len();
    if len == 0 {
        return Err(());
    }

    let (mut value, unit) = input.split_at(len - 1);
    // Source for conversion rules:
    // linux-kernel/drivers/md/bcache/util.c:bch_hprint()
    let mul = match unit {
        "k" => 1u64 << 10,
        "M" => 1 << 20,
        "G" => 1 << 30,
        "T" => 1 << 40,
        "P" => 1 << 50,
        "E" => 1 << 60,
        "Z" => 64,
        "Y" => 65536,
        _ if unit.parse::<u8>().is_ok() => {
            value = input;
            1
        }
        _ => return Err(()),
    };

    let value = match value.split_once('.') {
        Some((first, second)) => {
            // parses the peculiar format produced by bcache's bch_hprint.
            let mant = first.parse::<f64>().map_err(|_| ())?;
            let frac = second.parse::<f64>().map_err(|_| ())?;

            mant + frac / 10.24
        }
        None => value.parse::<f64>().map_err(|_| ())?,
    };

    Ok((value * mul as f64) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        let path = "tests/node/sys".into();
        let stats = bcache_stats(path, true).unwrap();

        assert_eq!(stats.len(), 1);

        assert_eq!(stats[0].name, "deaddd54-c735-46d5-868e-f331c5fd7c74");
        assert_eq!(stats[0].bdevs.len(), 1);
        assert_eq!(stats[0].caches.len(), 1);
    }

    #[test]
    fn writeback_rate_debug() {
        let path = "tests/node/sys/fs/bcache/deaddd54-c735-46d5-868e-f331c5fd7c74/bdev0/writeback_rate_debug";
        let stats = read_writeback_rate_debug(path.into()).unwrap();
        println!("{:#?}", stats);
    }

    #[test]
    fn priority_stats() {
        let path =
            "tests/node/sys/fs/bcache/deaddd54-c735-46d5-868e-f331c5fd7c74/cache0/priority_stats"
                .into();
        let stats = read_priority_stats(path).unwrap();
        assert_eq!(stats.unused_percent, 99);
        assert_eq!(stats.metadata_percent, 0);
    }

    #[test]
    fn dehumanize() {
        for (input, expect) in [
            ("542k", Ok(555008)),
            ("322M", Ok(337641472)),
            ("1.1k", Ok(1124)),
            ("1.9k", Ok(1924)),
            ("1.10k", Ok(2024)),
            ("", Err(())),
        ] {
            let got = parse_bytes(input);
            assert_eq!(expect, got, "input {:?}", input);
        }
    }
}
