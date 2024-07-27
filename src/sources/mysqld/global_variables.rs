use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::sync::LazyLock;

use event::{tags, Metric};
use sqlx::MySqlPool;

use super::{valid_name, MysqlError};

static GLOBAL_VARIABLES_DESC: LazyLock<BTreeMap<&str, &str>> = LazyLock::new(|| {
    let mut map = BTreeMap::new();
    // https://github.com/facebook/mysql-5.6/wiki/New-MySQL-RocksDB-Server-Variables
    map.insert("rocksdb_access_hint_on_compaction_start", "File access pattern once a compaction is started, applied to all input files of a compaction.");
    map.insert(
        "rocksdb_advise_random_on_open",
        "Hint of random access to the filesystem when a data file is opened.",
    );
    map.insert(
        "rocksdb_allow_concurrent_memtable_write",
        "Allow multi-writers to update memtables in parallel.",
    );
    map.insert(
        "rocksdb_allow_mmap_reads",
        "Allow the OS to mmap a data file for reads.",
    );
    map.insert(
        "rocksdb_allow_mmap_writes",
        "Allow the OS to mmap a data file for writes.",
    );
    map.insert("rocksdb_block_cache_size", "Size of the LRU block cache in RocksDB. This memory is reserved for the block cache, which is in addition to any filesystem caching that may occur.");
    map.insert(
        "rocksdb_block_restart_interval",
        "Number of keys for each set of delta encoded data.",
    );
    map.insert("rocksdb_block_size_deviation", "If the percentage of free space in the current data block (size specified in rocksdb-block-size) is less than this amount, close the block (and write record to new block).");
    map.insert(
        "rocksdb_block_size",
        "Size of the data block for reading sst files.",
    );
    map.insert("rocksdb_bulk_load_size", "Sets the number of keys to accumulate before committing them to the storage engine during bulk loading.");
    map.insert("rocksdb_bulk_load", "When set, MyRocks will ignore checking keys for uniqueness or acquiring locks during transactions. This option should only be used when the application is certain there are no row conflicts, such as when setting up a new MyRocks instance from an existing MySQL dump.");
    map.insert(
        "rocksdb_bytes_per_sync",
        "Enables the OS to sync out file writes as data files are created.",
    );
    map.insert("rocksdb_cache_index_and_filter_blocks", "Requests RocksDB to use the block cache for caching the index and bloomfilter data blocks from each data file. If this is not set, RocksDB will allocate additional memory to maintain these data blocks.");
    map.insert(
        "rocksdb_checksums_pct",
        "Sets the percentage of rows to calculate and set MyRocks checksums.",
    );
    map.insert(
        "rocksdb_collect_sst_properties",
        "Enables collecting statistics of each data file for improving optimizer behavior.",
    );
    map.insert("rocksdb_commit_in_the_middle", "Commit rows implicitly every rocksdb-bulk-load-size, during bulk load/insert/update/deletes.");
    map.insert("rocksdb_compaction_readahead_size", "When non-zero, bigger reads are performed during compaction. Useful if running RocksDB on spinning disks, compaction will do sequential instead of random reads.");
    map.insert(
        "rocksdb_compaction_sequential_deletes_count_sd",
        "If enabled, factor in single deletes as part of rocksdb-compaction-sequential-deletes.",
    );
    map.insert("rocksdb_compaction_sequential_deletes_file_size", "Threshold to trigger compaction if the number of sequential keys that are all delete markers exceed this value. While this compaction helps reduce request latency by removing delete markers, it can increase write rates of RocksDB.");
    map.insert("rocksdb_compaction_sequential_deletes_window", "Threshold to trigger compaction if, within a sliding window of keys, there exists this parameter's number of delete marker.");
    map.insert("rocksdb_compaction_sequential_deletes", "Enables triggering of compaction when the number of delete markers in a data file exceeds a certain threshold. Depending on workload patterns, RocksDB can potentially maintain large numbers of delete markers and increase latency of all queries.");
    map.insert(
        "rocksdb_create_if_missing",
        "Allows creating the RocksDB database if it does not exist.",
    );
    map.insert(
        "rocksdb_create_missing_column_families",
        "Allows creating new column families if they did not exist.",
    );
    map.insert("rocksdb_db_write_buffer_size", "Size of the memtable used to store writes within RocksDB. This is the size per column family. Once this size is reached, a flush of the memtable to persistent media occurs.");
    map.insert(
        "rocksdb_deadlock_detect",
        "Enables deadlock detection in RocksDB.",
    );
    map.insert(
        "rocksdb_debug_optimizer_no_zero_cardinality",
        "Test only to prevent MyRocks from calculating cardinality.",
    );
    map.insert("rocksdb_delayed_write_rate", "When RocksDB hits the soft limits/thresholds for writes, such as soft_pending_compaction_bytes_limit being hit, or level0_slowdown_writes_trigger being hit, RocksDB will slow the write rate down to the value of this parameter as bytes/second.");
    map.insert("rocksdb_delete_obsolete_files_period_micros", "The periodicity of when obsolete files get deleted, but does not affect files removed through compaction.");
    map.insert("rocksdb_enable_bulk_load_api", "Enables using the SSTFileWriter feature in RocksDB, which bypasses the memtable, but this requires keys to be inserted into the table in either ascending or descending order. If disabled, bulk loading uses the normal write path via the memtable and does not keys to be inserted in any order.");
    map.insert(
        "rocksdb_enable_thread_tracking",
        "Set to allow RocksDB to track the status of threads accessing the database.",
    );
    map.insert("rocksdb_enable_write_thread_adaptive_yield", "Set to allow RocksDB write batch group leader to wait up to the max time allowed before blocking on a mutex, allowing an increase in throughput for concurrent workloads.");
    map.insert(
        "rocksdb_error_if_exists",
        "If set, reports an error if an existing database already exists.",
    );
    map.insert("rocksdb_flush_log_at_trx_commit", "Sync'ing on transaction commit similar to innodb-flush-log-at-trx-commit: 0 - never sync, 1 - always sync, 2 - sync based on a timer controlled via rocksdb-background-sync");
    map.insert("rocksdb_flush_memtable_on_analyze", "When analyze table is run, determines of the memtable should be flushed so that data in the memtable is also used for calculating stats.");
    map.insert("rocksdb_force_compute_memtable_stats", "When enabled, also include data in the memtables for index statistics calculations used by the query optimizer. Greater accuracy, but requires more cpu.");
    map.insert(
        "rocksdb_force_flush_memtable_now",
        "Triggers MyRocks to flush the memtables out to the data files.",
    );
    map.insert("rocksdb_force_index_records_in_range", "When force index is used, a non-zero value here will be used as the number of rows to be returned to the query optimizer when trying to determine the estimated number of rows.");
    map.insert("rocksdb_hash_index_allow_collision", "Enables RocksDB to allow hashes to collide (uses less memory). Otherwise, the full prefix is stored to prevent hash collisions.");
    map.insert(
        "rocksdb_keep_log_file_num",
        "Sets the maximum number of info LOG files to keep around.",
    );
    map.insert("rocksdb_lock_scanned_rows", "If enabled, rows that are scanned during UPDATE remain locked even if they have not been updated.");
    map.insert("rocksdb_lock_wait_timeout", "Sets the number of seconds MyRocks will wait to acquire a row lock before aborting the request.");
    map.insert(
        "rocksdb_log_file_time_to_roll",
        "Sets the number of seconds a info LOG file captures before rolling to a new LOG file.",
    );
    map.insert("rocksdb_manifest_preallocation_size", "Sets the number of bytes to preallocate for the MANIFEST file in RocksDB and reduce possible random I/O on XFS. MANIFEST files are used to store information about column families, levels, active files, etc.");
    map.insert(
        "rocksdb_max_open_files",
        "Sets a limit on the maximum number of file handles opened by RocksDB.",
    );
    map.insert(
        "rocksdb_max_row_locks",
        "Sets a limit on the maximum number of row locks held by a transaction before failing it.",
    );
    map.insert("rocksdb_max_subcompactions", "For each compaction job, the maximum threads that will work on it simultaneously (i.e. subcompactions). A value of 1 means no subcompactions.");
    map.insert("rocksdb_max_total_wal_size", "Sets a limit on the maximum size of WAL files kept around. Once this limit is hit, RocksDB will force the flushing of memtables to reduce the size of WAL files.");
    map.insert("rocksdb_merge_buf_size", "Size (in bytes) of the merge buffers used to accumulate data during secondary key creation. During secondary key creation the data, we avoid updating the new indexes through the memtable and L0 by writing new entries directly to the lowest level in the database. This requires the values to be sorted so we use a merge/sort algorithm. This setting controls how large the merge buffers are. The default is 64Mb.");
    map.insert("rocksdb_merge_combine_read_size", "Size (in bytes) of the merge combine buffer used in the merge/sort algorithm as described in rocksdb-merge-buf-size.");
    map.insert("rocksdb_new_table_reader_for_compaction_inputs", "Indicates whether RocksDB should create a new file descriptor and table reader for each compaction input. Doing so may use more memory but may allow pre-fetch options to be specified for compaction input files without impacting table readers used for user queries.");
    map.insert(
        "rocksdb_no_block_cache",
        "Disables using the block cache for a column family.",
    );
    map.insert(
        "rocksdb_paranoid_checks",
        "Forces RocksDB to re-read a data file that was just created to verify correctness.",
    );
    map.insert(
        "rocksdb_pause_background_work",
        "Test only to start and stop all background compactions within RocksDB.",
    );
    map.insert(
        "rocksdb_perf_context_level",
        "Sets the level of information to capture via the perf context plugins.",
    );
    map.insert(
        "rocksdb_persistent_cache_size_mb",
        "The size (in Mb) to allocate to the RocksDB persistent cache if desired.",
    );
    map.insert("rocksdb_pin_l0_filter_and_index_blocks_in_cache", "If rocksdb-cache-index-and-filter-blocks is true then this controls whether RocksDB 'pins' the filter and index blocks in the cache.");
    map.insert("rocksdb_print_snapshot_conflict_queries", "If this is true, MyRocks will log queries that generate snapshot conflicts into the .err log.");
    map.insert("rocksdb_rate_limiter_bytes_per_sec", "Controls the rate at which RocksDB is allowed to write to media via memtable flushes and compaction.");
    map.insert(
        "rocksdb_records_in_range",
        "Test only to override the value returned by records-in-range.",
    );
    map.insert(
        "rocksdb_seconds_between_stat_computes",
        "Sets the number of seconds between recomputation of table statistics for the optimizer.",
    );
    map.insert(
        "rocksdb_signal_drop_index_thread",
        "Test only to signal the MyRocks drop index thread.",
    );
    map.insert(
        "rocksdb_skip_bloom_filter_on_read",
        "Indicates whether the bloom filters should be skipped on reads.",
    );
    map.insert(
        "rocksdb_skip_fill_cache",
        "Requests MyRocks to skip caching data on read requests.",
    );
    map.insert(
        "rocksdb_stats_dump_period_sec",
        "Sets the number of seconds to perform a RocksDB stats dump to the info LOG files.",
    );
    map.insert(
        "rocksdb_store_row_debug_checksums",
        "Include checksums when writing index/table records.",
    );
    map.insert(
        "rocksdb_strict_collation_check",
        "Enables MyRocks to check and verify table indexes have the proper collation settings.",
    );
    map.insert(
        "rocksdb_table_cache_numshardbits",
        "Sets the number of table caches within RocksDB.",
    );
    map.insert("rocksdb_use_adaptive_mutex", "Enables adaptive mutexes in RocksDB which spins in user space before resorting to the kernel.");
    map.insert("rocksdb_use_direct_reads", "Enable direct IO when opening a file for read/write. This means that data will not be cached or buffered.");
    map.insert(
        "rocksdb_use_fsync",
        "Requires RocksDB to use fsync instead of fdatasync when requesting a sync of a data file.",
    );
    map.insert(
        "rocksdb_validate_tables",
        "Requires MyRocks to verify all of MySQL's .frm files match tables stored in RocksDB.",
    );
    map.insert(
        "rocksdb_verify_row_debug_checksums",
        "Verify checksums when reading index/table records.",
    );
    map.insert(
        "rocksdb_wal_bytes_per_sync",
        "Controls the rate at which RocksDB writes out WAL file data.",
    );
    map.insert(
        "rocksdb_wal_recovery_mode",
        "Sets RocksDB's level of tolerance when recovering the WAL files after a system crash.",
    );
    map.insert("rocksdb_wal_size_limit_mb", "Maximum size the RocksDB WAL is allow to grow to. When this size is exceeded rocksdb attempts to flush sufficient memtables to allow for the deletion of the oldest log.");
    map.insert(
        "rocksdb_wal_ttl_seconds",
        "No WAL file older than this value should exist.",
    );
    map.insert("rocksdb_whole_key_filtering", "Enables the bloomfilter to use the whole key for filtering instead of just the prefix. In order for this to be efficient, lookups should use the whole key for matching.");
    map.insert(
        "rocksdb_write_disable_wal",
        "Disables logging data to the WAL files. Useful for bulk loading.",
    );
    map.insert(
        "rocksdb_write_ignore_missing_column_families",
        "If 1, then writes to column families that do not exist is ignored by RocksDB.",
    );
    map
});

const GLOBAL_VARIABLES_QUERY: &str = r#"SHOW GLOBAL VARIABLES"#;

#[derive(Debug, sqlx::FromRow)]
struct GlobalVariable {
    #[sqlx(rename = "Variable_name")]
    name: String,
    #[sqlx(rename = "Value")]
    value: String,
}

pub async fn gather(pool: &MySqlPool) -> Result<Vec<Metric>, MysqlError> {
    let variables = sqlx::query_as::<_, GlobalVariable>(GLOBAL_VARIABLES_QUERY)
        .fetch_all(pool)
        .await
        .map_err(|err| MysqlError::Query {
            query: GLOBAL_VARIABLES_QUERY,
            err,
        })?;

    let mut metrics = vec![];
    let mut text_items = BTreeMap::new();
    text_items.insert("innodb_version".to_string(), "".to_string());
    text_items.insert("version".to_string(), "".to_string());
    text_items.insert("version_comment".to_string(), "".to_string());
    text_items.insert("wsrep_cluster_name".to_string(), "".to_string());
    text_items.insert("wsrep_provider_options".to_string(), "".to_string());

    for var in variables {
        let key = valid_name(&var.name);
        let fv = match var.value.parse::<f64>() {
            Ok(v) => v,
            _ => {
                if let Entry::Occupied(mut entry) = text_items.entry(key) {
                    entry.insert(var.value);
                }

                continue;
            }
        };

        let desc = GLOBAL_VARIABLES_DESC
            .get(key.as_str())
            .unwrap_or(&"Generic gauge metric from SHOW GLOBAL VARIABLES");

        metrics.push(Metric::gauge(
            "mysql_global_variables_".to_owned() + &key,
            *desc,
            fv,
        ));
    }

    // mysql_version_info metric
    metrics.push(Metric::gauge_with_tags(
        "mysql_version_info",
        "MySQL version and distribution",
        1,
        tags!(
            "innodb_version" => text_items.get("innodb_version").unwrap(),
            "version" => text_items.get("version").unwrap(),
            "version_comment" => text_items.get("version_comment").unwrap(),
        ),
    ));

    // mysql_galera_variables_info metric
    match text_items.get("wsrep_cluster_name") {
        Some(value) if !value.is_empty() => metrics.push(Metric::gauge_with_tags(
            "mysql_galera_variables_info",
            "PXC/Galera variables information",
            1,
            tags!(
                "wsrep_cluster_name" => text_items.get("wsrep_cluster_name").unwrap(),
            ),
        )),
        _ => {}
    }

    // mysql_galera_gcache_size_bytes metric
    match text_items.get("wsrep_provider_options") {
        Some(value) if !value.is_empty() => {
            metrics.push(Metric::gauge(
                "mysql_galera_gcache_size_bytes",
                "PXC/Galera gcache size",
                parse_wsrep_provider_options(value),
            ));
        }
        _ => {}
    }

    Ok(metrics)
}

// parse wsrep_provider_options to get gcache.size in bytes
fn parse_wsrep_provider_options(s: &str) -> f64 {
    match s.find("gcache.size = ") {
        Some(index) => s
            .chars()
            .skip(index + 14)
            .take_while(|c| c.is_numeric() || *c == 'M' || *c == 'G')
            .fold(0.0, |acc, c| match c {
                'M' => acc * 1024.0 * 1024.0,
                'G' => acc * 1024.0 * 1024.0 * 1024.0,
                _ => acc * 10.0 + (c as u8 - b'0') as f64,
            }),
        None => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wsrep_provider_options() {
        for (input, want) in [
            (
                "",
                0.0
            ),
            (
                "base_dir = /var/lib/mysql/; base_host = 10.91.142.82; base_port = 4567; cert.log_conflicts = no; debug = no; evs.auto_evict = 0; evs.causal_keepalive_period = PT1S; evs.debug_log_mask = 0x1; evs.delay_margin = PT1S; evs.delayed_keep_period = PT30S; evs.inactive_check_period = PT0.5S; evs.inactive_timeout = PT15S; evs.info_log_mask = 0; evs.install_timeout = PT7.5S; evs.join_retrans_period = PT1S; evs.keepalive_period = PT1S; evs.max_install_timeouts = 3; evs.send_window = 4; evs.stats_report_period = PT1M; evs.suspect_timeout = PT5S; evs.use_aggregate = true; evs.user_send_window = 2; evs.version = 0; evs.view_forget_timeout = P1D; gcache.dir = /var/lib/mysql/; gcache.keep_pages_count = 0; gcache.keep_pages_size = 0; gcache.mem_size = 0; gcache.name = /var/lib/mysql//galera.cache; gcache.page_size = 128M; gcache.size = 128M; gcomm.thread_prio = ; gcs.fc_debug = 0; gcs.fc_factor = 1.0; gcs.fc_limit = 16; gcs.fc_master_slave = no; gcs.max_packet_size = 64500; gcs.max_throttle = 0.25; gcs.recv_q_hard_limit = 9223372036854775807; gcs.recv_q_soft_limit = 0.25; gcs.sync_donor = no; gmcast.listen_addr = tcp://0.0.0.0:4567; gmcast.mcast_addr = ; gmcast.mcast_ttl = 1; gmcast.peer_timeout = PT3S; gmcast.segment = 0; gmcast.time_wait = PT5S; gmcast.version = 0; ist.recv_addr = 10.91.142.82; pc.announce_timeout = PT3S; pc.checksum = false; pc.ignore_quorum = false; pc.ignore_sb = false; pc.linger = PT20S; pc.npvo = false; pc.recovery = true; pc.version = 0; pc.wait_prim = true; pc.wait_prim_timeout = P30S; pc.weight = 1; protonet.backend = asio; protonet.version = 0; repl.causal_read_timeout = PT30S; repl.commit_order = 3; repl.key_format = FLAT8; repl.max_ws_size = 2147483647; repl.proto_max = 7; socket.checksum = 2; socket.recv_buf_size = 212992;",
                128.0 * 1024.0 * 1024.0
            ),
            (
                "base_dir = /var/lib/mysql/; base_host = 10.91.194.244; base_port = 4567; cert.log_conflicts = no; debug = no; evs.auto_evict = 0; evs.causal_keepalive_period = PT1S; evs.debug_log_mask = 0x1; evs.delay_margin = PT1S; evs.delayed_keep_period = PT30S; evs.inactive_check_period = PT0.5S; evs.inactive_timeout = PT15S; evs.info_log_mask = 0; evs.install_timeout = PT7.5S; evs.join_retrans_period = PT1S; evs.keepalive_period = PT1S; evs.max_install_timeouts = 3; evs.send_window = 4; evs.stats_report_period = PT1M; evs.suspect_timeout = PT5S; evs.use_aggregate = true; evs.user_send_window = 2; evs.version = 0; evs.view_forget_timeout = P1D; gcache.dir = /var/lib/mysql/; gcache.keep_pages_count = 0; gcache.keep_pages_size = 0; gcache.mem_size = 0; gcache.name = /var/lib/mysql//galera.cache; gcache.page_size = 128M; gcache.size = 2G; gcomm.thread_prio = ; gcs.fc_debug = 0; gcs.fc_factor = 1.0; gcs.fc_limit = 16; gcs.fc_master_slave = no; gcs.max_packet_size = 64500; gcs.max_throttle = 0.25; gcs.recv_q_hard_limit = 9223372036854775807; gcs.recv_q_soft_limit = 0.25; gcs.sync_donor = no; gmcast.listen_addr = tcp://0.0.0.0:4567; gmcast.mcast_addr = ; gmcast.mcast_ttl = 1; gmcast.peer_timeout = PT3S; gmcast.segment = 0; gmcast.time_wait = PT5S; gmcast.version = 0; ist.recv_addr = 10.91.194.244; pc.announce_timeout = PT3S; pc.checksum = false; pc.ignore_quorum = false; pc.ignore_sb = false; pc.linger = PT20S; pc.npvo = false; pc.recovery = true; pc.version = 0; pc.wait_prim = true; pc.wait_prim_timeout = P30S; pc.weight = 1; protonet.backend = asio; protonet.version = 0; repl.causal_read_timeout = PT30S; repl.commit_order = 3; repl.key_format = FLAT8; repl.max_ws_size = 2147483647; repl.proto_max = 7; socket.checksum = 2; socket.recv_buf_size = 212992;",
                2.0 * 1024.0 * 1024.0 * 1024.0
            ),
            (
                "gcache.page_size = 128M; gcache.size = 131072; gcomm.thread_prio = ;",
                131072.0
            )
        ] {
            assert_eq!(parse_wsrep_provider_options(input), want);
        }
    }
}
