use event::{Metric, tags};

use super::{Connection, Error};

const PERF_CONTEXT_QUERY: &str = "SELECT
  TABLE_SCHEMA,
  TABLE_NAME,
  ifnull(PARTITION_NAME, ''),
  STAT_TYPE,
  VALUE
FROM information_schema.ROCKSDB_PERF_CONTEXT";

const METRIC_INFOS: [(&str, &str, &str); 23] = [
    (
        "USER_KEY_COMPARISON_COUNT",
        "mysql_info_schema_rocksdb_perf_context_user_key_comparison_count",
        "Total number of user key comparisons performed in binary search.",
    ),
    (
        "BLOCK_CACHE_HIT_COUNT",
        "mysql_info_schema_rocksdb_perf_context_block_cache_hit_count",
        "Total number of block read operations from cache.",
    ),
    (
        "BLOCK_READ_COUNT",
        "mysql_info_schema_rocksdb_perf_context_block_read_count",
        "Total number of block read operations from disk.",
    ),
    (
        "BLOCK_READ_BYTE",
        "mysql_info_schema_rocksdb_perf_context_block_read_byte",
        "Total number of bytes read from disk.",
    ),
    (
        "GET_READ_BYTES",
        "mysql_info_schema_rocksdb_perf_context_get_read_bytes",
        "Number of bytes read during Get operations.",
    ),
    (
        "MULTIGET_READ_BYTES",
        "mysql_info_schema_rocksdb_perf_context_multiget_read_bytes",
        "Number of bytes read during MultiGet operations.",
    ),
    (
        "ITER_READ_BYTES",
        "mysql_info_schema_rocksdb_perf_context_iter_read_bytes",
        "Number of bytes read during iterator operations.",
    ),
    (
        "INTERNAL_KEY_SKIPPED_COUNT",
        "mysql_info_schema_rocksdb_perf_context_internal_key_skipped_count",
        "Count of internal keys skipped during operations.",
    ),
    (
        "INTERNAL_DELETE_SKIPPED_COUNT",
        "mysql_info_schema_rocksdb_perf_context_internal_delete_skipped_count",
        "Count of internal delete operations that were skipped.",
    ),
    (
        "INTERNAL_RECENT_SKIPPED_COUNT",
        "mysql_info_schema_rocksdb_perf_context_internal_recent_skipped_count",
        "Count of recently skipped internal operations.",
    ),
    (
        "INTERNAL_MERGE_COUNT",
        "mysql_info_schema_rocksdb_perf_context_internal_merge_count",
        "Total number of internal merge operations.",
    ),
    (
        "GET_FROM_MEMTABLE_COUNT",
        "mysql_info_schema_rocksdb_perf_context_get_from_memtable_count",
        "Number of Get operations served from the memtable.",
    ),
    (
        "SEEK_ON_MEMTABLE_COUNT",
        "mysql_info_schema_rocksdb_perf_context_seek_on_memtable_count",
        "Count of seek operations in the memtable.",
    ),
    (
        "NEXT_ON_MEMTABLE_COUNT",
        "mysql_info_schema_rocksdb_perf_context_next_on_memtable_count",
        "Count of next operations in the memtable.",
    ),
    (
        "PREV_ON_MEMTABLE_COUNT",
        "mysql_info_schema_rocksdb_perf_context_prev_on_memtable_count",
        "Count of previous operations in the memtable.",
    ),
    (
        "SEEK_CHILD_SEEK_COUNT",
        "mysql_info_schema_rocksdb_perf_context_seek_child_seek_count",
        "Count of child seek operations in RocksDB.",
    ),
    (
        "BLOOM_MEMTABLE_HIT_COUNT",
        "mysql_info_schema_rocksdb_perf_context_bloom_memtable_hit_count",
        "Count of successful hits in the bloom filter for memtable searches.",
    ),
    (
        "BLOOM_MEMTABLE_MISS_COUNT",
        "mysql_info_schema_rocksdb_perf_context_bloom_memtable_miss_count",
        "Count of misses in the bloom filter for memtable searches.",
    ),
    (
        "BLOOM_SST_HIT_COUNT",
        "mysql_info_schema_rocksdb_perf_context_bloom_sst_hit_count",
        "Count of successful hits in the bloom filter for SSTable searches.",
    ),
    (
        "BLOOM_SST_MISS_COUNT",
        "mysql_info_schema_rocksdb_perf_context_bloom_sst_miss_count",
        "Count of misses in the bloom filter for SSTable searches.",
    ),
    (
        "KEY_LOCK_WAIT_COUNT",
        "mysql_info_schema_rocksdb_perf_context_key_lock_wait_count",
        "Count of key lock wait events in RocksDB.",
    ),
    (
        "IO_BYTES_WRITTEN",
        "mysql_info_schema_rocksdb_perf_context_io_bytes_written",
        "Total number of bytes written by I/O operations in RocksDB.",
    ),
    (
        "IO_BYTES_READ",
        "mysql_info_schema_rocksdb_perf_context_io_bytes_read",
        "Total number of bytes read by I/O operations in RocksDB.",
    ),
];

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut rows = conn.query(PERF_CONTEXT_QUERY).await?;

    let mut metrics = vec![];
    while let Some(mut row) = rows.next().await? {
        let schema = row.get_str();
        let table = row.get_str();
        let part = row.get_str();
        let stat = row.get_str();
        let value = row.get_str().parse::<f64>()?;

        let Some((_, name, desc)) = METRIC_INFOS.iter().find(|(item, _, _)| *item == stat) else {
            continue;
        };

        metrics.push(Metric::sum_with_tags(
            *name,
            *desc,
            value,
            tags!(
                "schema" => schema,
                "table" => table,
                "part" => part
            ),
        ))
    }

    Ok(metrics)
}
