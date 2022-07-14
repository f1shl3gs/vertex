use std::collections::BTreeMap;

use event::attributes::Attributes;
use event::{tags, Metric};
use serde::Deserialize;

use super::Elasticsearch;

/// `IndicesDocs` defines node stats docs information structure for indices.
#[derive(Deserialize)]
struct IndicesDocs {
    count: i64,
    deleted: i64,
}

/// `IndicesStore` defines node stats store information structure for indices.
#[derive(Deserialize)]
struct IndicesStore {
    size_in_bytes: i64,
    throttle_time_in_millis: i64,
}

/// `IndicesIndexing` defines node stats indexing information structure for indices
#[derive(Deserialize)]
struct IndicesIndexing {
    index_total: i64,
    index_time_in_millis: i64,
    index_current: i64,
    delete_total: i64,
    delete_time_in_millis: i64,
    delete_current: i64,
    is_throttled: bool,
    throttle_time_in_millis: i64,
}

/// `IndicesMerges` defines node stats merges information structure for indices.
#[derive(Deserialize)]
struct IndicesMerges {
    current: i64,
    current_docs: i64,
    current_size_in_bytes: i64,
    total: i64,
    total_docs: i64,
    total_size_in_bytes: i64,
    total_time_in_millis: i64,
    total_throttled_time_in_millis: i64,
}

/// `IndicesGet` defines node stats get information structure for indices
#[derive(Deserialize)]
struct IndicesGet {
    total: i64,
    time_in_millis: i64,
    exists_total: i64,
    exists_time_in_millis: i64,
    missing_total: i64,
    missing_time_in_millis: i64,
    current: i64,
}

/// `IndicesSearch` defines node stats search information structure for indices
#[derive(Deserialize)]
struct IndicesSearch {
    open_contexts: i64,
    query_total: i64,
    query_time_in_millis: i64,
    query_current: i64,
    fetch_total: i64,
    fetch_time_in_millis: i64,
    fetch_current: i64,
    suggest_total: i64,
    suggest_time_in_millis: i64,
    scroll_total: i64,
    scroll_time_in_millis: i64,
}

/// `IndicesCache` defines node stats cache information structure for indices
#[derive(Deserialize)]
struct IndicesCache {
    evictions: i64,
    memory_size_in_bytes: i64,
    cache_count: i64,
    cache_size: i64,
    hit_count: i64,
    miss_count: i64,
    total_count: i64,
}

/// `IndicesFlush` defines node stats flush information structure for indices.
#[derive(Deserialize)]
struct IndicesFlush {
    total: i64,
    total_time_in_millis: i64,
}

/// `IndicesWarmer` defines node stats warmer information structure for indices
#[derive(Deserialize)]
struct IndicesWarmer {
    total: i64,
    total_time_in_millis: i64,
}

/// `IndicesSegments` defines node stats segments information structure for indices
#[derive(Deserialize)]
struct IndicesSegments {
    count: i64,
    memory_in_bytes: i64,
    terms_memory_in_bytes: i64,
    index_writer_memory_in_bytes: i64,
    norms_memory_in_bytes: i64,
    stored_fields_memory_in_bytes: i64,
    fixed_bit_set_memory_in_bytes: i64,
    doc_values_memory_in_bytes: i64,
    term_vectors_memory_in_bytes: i64,
    points_memory_in_bytes: i64,
    version_map_memory_in_bytes: i64,
}

/// `IndicesRefresh` defines node stats refresh information structure for indices.
#[derive(Deserialize)]
struct IndicesRefresh {
    total: i64,
    total_time_in_millis: i64,
}

/// `IndicesTranslog` defines node stats translog information structure for indices.
#[derive(Deserialize)]
struct IndicesTranslog {
    operations: i64,
    size_in_bytes: i64,
}

/// `IndicesCompletion` defines node stats completion information structure for indices.
#[derive(Deserialize)]
struct IndicesCompletion {
    size_in_bytes: i64,
}

/// Indices is a representation of a indices stats (size, document count, indexing,
/// deletion times, search times, field cache size, merges and flushes).
#[derive(Deserialize)]
struct Indices {
    docs: IndicesDocs,
    store: IndicesStore,
    indexing: IndicesIndexing,
    merges: IndicesMerges,
    get: IndicesGet,
    search: IndicesSearch,
    #[serde(rename = "fielddata")]
    field_data: IndicesCache,
    filter_cache: IndicesCache,
    query_cache: IndicesCache,
    request_cache: IndicesCache,
    flush: IndicesFlush,
    warmer: IndicesWarmer,
    segments: IndicesSegments,
    refresh: IndicesRefresh,
    translog: IndicesTranslog,
    completion: IndicesCompletion,
}

#[derive(Deserialize)]
struct OsCpuLoad {
    #[serde(rename = "1m")]
    load1: f64,
    #[serde(rename = "5m")]
    load5: f64,
    #[serde(rename = "15m")]
    load15: f64,
}

/// `OsCpu` defines node stats operating system CPU usage structure
#[derive(Deserialize)]
struct OsCpu {
    load_average: OsCpuLoad,
    percent: i64,
}

/// `OsMem` defines node stats operating system memory usage structure
#[derive(Deserialize)]
struct OsMem {
    free_in_bytes: i64,
    used_in_bytes: i64,
    actual_free_in_bytes: i64,
    actual_used_in_bytes: i64,
}

/// `OsSwap` defines node stats operating system swap usage structure
#[derive(Deserialize)]
struct OsSwap {
    used_in_bytes: i64,
    free_in_bytes: i64,
}

/// `Os` is a representation of an operating system stats, load average, mem and swap
#[derive(Deserialize)]
struct Os {
    timestamp: i64,
    uptime_in_millis: i64,
    // LoadAvg was an array of per-cpu values pre-2.0, and is a string in 2.0
    // Leaving this here in case we want to implement parsing logic later.
    //
    cpu: OsCpu,
    mem: OsMem,
    swap: OsSwap,
}

/// `Tcp` defines node stats TCP information structure.
#[derive(Deserialize)]
struct Tcp {
    active_opens: i64,
    passive_opens: i64,
    curr_estab: i64,
    in_segs: i64,
    out_segs: i64,
    retrans_segs: i64,
    estab_resets: i64,
    attempt_fails: i64,
    in_errs: i64,
    out_rsts: i64,
}

/// `Network` defines node stats network information structure
#[derive(Deserialize)]
struct Network {
    tcp: Tcp,
}

/// `FsData` defines node stats filesystem data structure
#[derive(Deserialize)]
struct FsData {
    path: String,
    mount: String,
    dev: String,
    total_in_bytes: i64,
    free_in_bytes: i64,
    available_in_bytes: i64,
}

/// `FsIoStatsDevice` is a representation of a node stat filesystem device
#[derive(Deserialize)]
struct FsIoStatsDevice {
    device_name: String,
    operations: i64,
    read_operations: i64,
    write_operations: i64,
    read_kilobytes: i64,
    write_kilobytes: i64,
}

/// `FsIoStats`
#[derive(Deserialize)]
struct FsIoStats {
    devices: Vec<FsIoStatsDevice>,
}

struct Fs {
    timestamp: i64,
    data: Vec<FsData>,
    io_stats: FsIoStats,
}

/// `JvmGcCollector` defines node stats JVM garbage collector collection information strucutre.
#[derive(Deserialize)]
struct JvmGcCollector {
    collection_count: i64,
    collection_time_in_millis: i64,
}

/// `JvmGc` defines node stats JVM garbage collector information structure.
#[derive(Deserialize)]
struct JvmGc {
    collectors: BTreeMap<String, JvmGcCollector>,
}

/// `JvmMemPool` defines node status JVM memory pool information
#[derive(Deserialize, Default)]
struct JvmMemPool {
    used_in_bytes: i64,
    max_in_bytes: i64,
    peak_used_in_bytes: i64,
    peak_max_in_bytes: i64,
}

/// `JvmMem` defines node stats JVM memory information
#[derive(Deserialize)]
struct JvmMem {
    heap_committed_in_bytes: i64,
    heap_used_in_bytes: i64,
    heap_max_in_bytes: i64,
    non_heap_committed_in_bytes: i64,
    non_heap_used_in_bytes: i64,
    pools: BTreeMap<String, JvmMemPool>,
}

/// `JvmBufferPool` defines node stats JVM buffer pool information
#[derive(Deserialize, Default)]
struct JvmBufferPool {
    count: i64,
    total_capacity_in_bytes: i64,
    used_in_bytes: i64,
}

/// `Jvm` is a representation of a JVM stats, memory pool information, garbage collection,
/// buffer pools, number of loaded/unloaded classes.
#[derive(Deserialize)]
struct Jvm {
    buffer_pools: BTreeMap<String, JvmBufferPool>,
    gc: JvmGc,
    mem: JvmMem,
    uptime_in_millis: i64,
}

#[derive(Deserialize)]
struct ProcessCpu {
    percent: i64,
    total_in_millis: i64,
}

#[derive(Deserialize)]
struct ProcessMem {
    resident_in_bytes: i64,
    share_in_bytes: i64,
    total_virtual_in_bytes: i64,
}

/// `Process` is a representation of a process statistics, memory consumption,
/// cpu usage and open file descriptors
#[derive(Deserialize)]
struct Process {
    timestamp: i64,
    open_file_descriptors: i64,
    max_file_descriptors: i64,
    cpu: ProcessCpu,
    mem: ProcessMem,
}

#[derive(Deserialize)]
struct Transport {
    server_open: i64,
    rx_count: i64,
    rx_size_in_bytes: i64,
    tx_count: i64,
    tx_size_in_bytes: i64,
}

#[derive(Deserialize)]
struct Breaker {
    estimated_size_in_bytes: i64,
    limit_size_in_bytes: i64,
    overhead: i64,
    tripped: i64,
}

#[derive(Deserialize)]
struct ThreadPool {
    threads: i64,
    queue: i64,
    active: i64,
    rejected: i64,
    largest: i64,
    completed: i64,
}

/// `NodeStats` defines node stats information structure for nodes
#[derive(Deserialize)]
struct NodeStats {
    name: String,
    host: String,
    timestamp: i64,
    transport_address: String,
    hostname: String,
    #[serde(default)]
    roles: Vec<String>,
    attributes: BTreeMap<String, String>,
    indices: Indices,
    os: Os,
    network: Network,
    fs: Fs,
    thread_pool: BTreeMap<String, ThreadPool>,
    jvm: Jvm,
    breakers: BTreeMap<String, Breaker>,
    // http: BtreeMap<String, Http>,
    transport: Transport,
    process: Process,
}

/// `NodeStatsResp` is a representation of an Elasticsearch Node Stats.
#[derive(Deserialize)]
struct NodeStatsResp {
    cluster_name: String,
    nodes: BTreeMap<String, NodeStats>,
}

impl Elasticsearch {
    async fn collect_node_stats(&self, node: &str) -> Result<Vec<Metric>, crate::Error> {
        let mut metrics = vec![];
        let url = format!("/_node/{}/stats", node);
        let stats = self.fetch::<NodeStatsResp>(url.as_str()).await?;

        for (_name, node) in stats.nodes {
            for role in ["master", "data", "client", "ingest"] {
                if node.roles.iter().any(|s| s == role) {
                    metrics.push(Metric::gauge_with_tags(
                        "elasticsearch_nodes_roles",
                        "Node roles",
                        1.0,
                        tags!(
                            "cluster" => stats.cluster_name.clone(),
                            "host" => node.host.clone(),
                            "name" => node.name.clone()
                        ),
                    ))
                }
            }

            let es_master_node = node.roles.iter().any(|s| s == "master").to_string();
            let es_data_node = node.roles.iter().any(|s| s == "data").to_string();
            let es_ingest_node = node.roles.iter().any(|s| s == "ingest").to_string();
            let es_client_node = node.roles.iter().any(|s| s == "client").to_string();

            let tags = tags!(
                "cluster" => stats.cluster_name.clone(),
                "host" => node.host,
                "name" => node.name,
                "es_master_node" => es_master_node,
                "es_data_node" => es_data_node,
                "es_ingest_node" => es_ingest_node,
                "es_client_node" => es_client_node,
            );
            // OS stats
            metrics.extend(os_metrics(tags.clone(), node.os));

            metrics.extend(node_metrics(tags.clone(), &node));

            // GC stats
            for (collector, stats) in node.jvm.gc.collectors {
                metrics.extend(gc_metrics(tags.with("gc", collector), &stats));
            }

            // Breaker stats
            for (breaker, stats) in node.breakers {
                metrics.extend(breaker_metrics(
                    tags.with("breaker", breaker.clone()),
                    stats,
                ));
            }

            // Thread pool stats
            for (name, pool) in node.thread_pool {
                metrics.extend(thread_pool_metrics(tags.with("pool", name.clone()), pool));
            }

            // Filesystem data stats
            for fs_stats in node.fs.data {
                metrics.extend(filesystem_data_metrics(tags.clone(), fs_stats));
            }

            // Filesystem IO device stats
            for io_stats in node.fs.io_stats.devices {
                metrics.extend(filesystem_io_metrics(tags.clone(), io_stats))
            }
        }

        Ok(metrics)
    }
}

fn os_metrics(tags: Attributes, stats: Os) -> Vec<Metric> {
    vec![
        Metric::gauge_with_tags(
            "elasticsearch_os_load1",
            "Shortterm load average",
            stats.cpu.load_average.load1,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_os_load5",
            "Midterm load average",
            stats.cpu.load_average.load5,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_os_load15",
            "Longterm load average",
            stats.cpu.load_average.load15,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_os_cpu_percent",
            "Percent CPU used by OS",
            stats.cpu.percent,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_os_mem_free_bytes",
            "Amount of free physical memory in bytes",
            stats.mem.free_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_os_mem_used_bytes",
            "Amount of used physical memory in bytes",
            stats.mem.used_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_os_mem_actual_free_bytes",
            "Amount of free physical memory in bytes",
            stats.mem.actual_free_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_os_mem_actual_used_bytes",
            "Amount of used physical memory in bytes",
            stats.mem.actual_used_in_bytes,
            tags,
        ),
    ]
}

fn node_metrics(tags: Attributes, node: &NodeStats) -> Vec<Metric> {
    vec![
        Metric::gauge_with_tags(
            "elasticsearch_indices_fielddata_memory_size_bytes",
            "Field data cache memory usage in bytes",
            node.indices.field_data.memory_size_in_bytes,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_fielddata_evictions",
            "Evictions from field data",
            node.indices.field_data.evictions,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_completion_size_in_bytes",
            "Completion in bytes",
            node.indices.completion.size_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_filter_cache_memory_size_bytes",
            "Filter cache memory usage in bytes",
            node.indices.filter_cache.memory_size_in_bytes,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_filter_cache_evictions",
            "Evictions from filter cache",
            node.indices.filter_cache.evictions,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_query_cache_memory_size_bytes",
            "Query cache memory usage in bytes",
            node.indices.query_cache.memory_size_in_bytes,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_query_cache_evictions",
            "Evictions from query cache",
            node.indices.query_cache.evictions,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_query_cache_total",
            "Query cache total count",
            node.indices.query_cache.total_count,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_query_cache_cache_size",
            "Query cache cache size",
            node.indices.query_cache.cache_size,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_query_cache_cache_total",
            "Query cache cache count",
            node.indices.query_cache.cache_count,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_query_cache_count",
            "Query cache count",
            node.indices.query_cache.hit_count,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_query_miss_count",
            "Query miss count",
            node.indices.query_cache.miss_count,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_request_cache_memory_size_bytes",
            "Request cache memory usage in bytes",
            node.indices.request_cache.memory_size_in_bytes,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_request_cache_evictions",
            "Evictions from request cache",
            node.indices.request_cache.evictions,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_request_cache_count",
            "Request cache count",
            node.indices.request_cache.hit_count,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_request_miss_count",
            "Request miss count",
            node.indices.request_cache.miss_count,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_translog_operations",
            "Total translog operations",
            node.indices.translog.operations,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_translog_size_in_bytes",
            "Total translog size in bytes",
            node.indices.translog.size_in_bytes,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_get_time_seconds",
            "Total get time in seconds",
            node.indices.get.time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_get_total",
            "Total get",
            node.indices.get.total,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_get_missing_time_seconds",
            "Total time of get missing in seconds",
            node.indices.get.missing_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_get_missing_total",
            "Total get missing",
            node.indices.get.missing_total,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_get_exists_time_seconds",
            "Total time get exists in seconds",
            node.indices.get.exists_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_get_exists_total",
            "Total get exists operation",
            node.indices.get.exists_total,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_refresh_time_seconds_total",
            "Total time spent refreshing in seconds",
            node.indices.refresh.total_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_refresh_total",
            "Total refreshes",
            node.indices.refresh.total,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_search_query_time_seconds",
            "Total search query time in seconds",
            node.indices.search.query_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_search_query_total",
            "Total number of queries",
            node.indices.search.query_total,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_search_fetch_time_seconds",
            "Total search fetch time in seconds",
            node.indices.search.fetch_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_search_fetch_total",
            "Total number of fetches",
            node.indices.search.fetch_total,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_search_suggest_total",
            "Total number of suggests",
            node.indices.search.suggest_total,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_search_suggest_time_seconds",
            "Total suggest time in seconds",
            node.indices.search.suggest_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_search_scroll_total",
            "Total number of scrolls",
            node.indices.search.scroll_total,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_search_scroll_time_seconds",
            "Total scroll time in seconds",
            node.indices.search.scroll_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_docs",
            "Count of documents on this node",
            node.indices.docs.count,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_docs_deleted",
            "Count of deleted documents on this node",
            node.indices.docs.deleted,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_store_size_bytes",
            "Current size of stored index data in bytes",
            node.indices.store.size_in_bytes,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_store_throttle_time_seconds_total",
            "Throttle time for index store in seconds",
            node.indices.store.throttle_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_segments_memory_bytes",
            "Current memory size of segments in bytes",
            node.indices.segments.memory_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_segments_count",
            "Count of index segments on this node",
            node.indices.segments.count,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_segments_terms_memory_in_bytes",
            "Count of terms in memory for this node",
            node.indices.segments.terms_memory_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_segments_index_writer_memory_in_bytes",
            "Count of memory for index writer on this node",
            node.indices.segments.index_writer_memory_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_segments_norms_memory_in_bytes",
            "Count of memory used by norms",
            node.indices.segments.norms_memory_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_segments_stored_fields_memory_in_bytes",
            "Count of stored fields memory",
            node.indices.segments.stored_fields_memory_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_segments_doc_values_memory_in_bytes",
            "Count of doc values memory",
            node.indices.segments.doc_values_memory_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_segments_fixed_bit_set_memory_in_bytes",
            "count of fixed bit set",
            node.indices.segments.fixed_bit_set_memory_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_segments_term_vectors_memory_in_bytes",
            "term vectors memory usage in bytes",
            node.indices.segments.term_vectors_memory_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_segments_points_memory_in_bytes",
            "Point values memory usage in bytes",
            node.indices.segments.points_memory_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_segments_version_map_memory_in_bytes",
            "Version map memory usage in bytes",
            node.indices.segments.version_map_memory_in_bytes,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_flush_total",
            "Total flushes",
            node.indices.flush.total,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_flush_time_seconds",
            "Cumulative flush time in seconds",
            node.indices.flush.total_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_warmer_total",
            "total warmer count",
            node.indices.warmer.total,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_warmer_time_seconds_total",
            "Total warmer time in seconds",
            node.indices.warmer.total_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_indexing_index_time_seconds_total",
            "Cumulative index time in seconds",
            node.indices.indexing.index_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_indexing_index_total",
            "Total index calls",
            node.indices.indexing.index_total,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_indexing_delete_time_seconds_total",
            "Total time indexing delete in seconds",
            node.indices.indexing.delete_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_indexing_delete_total",
            "Total indexing deletes",
            node.indices.indexing.delete_total,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_indexing_is_throttled",
            "Indexing throttling",
            node.indices.indexing.is_throttled,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_indexing_throttle_time_seconds_total",
            "Cumulative indexing throttling time",
            node.indices.indexing.throttle_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_merges_total",
            "Total merges",
            node.indices.merges.total,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_merges_current",
            "Current merge",
            node.indices.merges.current,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_merges_current_size_in_bytes",
            "Size of a current merges in bytes",
            node.indices.merges.current_size_in_bytes,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_merges_docs_total",
            "Cumulative docs merged",
            node.indices.merges.total_docs,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_merges_total_size_bytes_total",
            "Total merge size in bytes",
            node.indices.merges.total_size_in_bytes,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_merges_total_time_seconds_total",
            "Total time spent merging in seconds",
            node.indices.merges.total_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_merges_total_throttled_time_seconds_total",
            "Total throttled time of merges in seconds",
            node.indices.merges.total_throttled_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_jvm_memory_used_bytes",
            "JVM memory currently used by area",
            node.jvm.mem.heap_used_in_bytes,
            tags.with("area", "heap"),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_jvm_memory_used_bytes",
            "JVM memory currently used by area",
            node.jvm.mem.non_heap_used_in_bytes,
            tags.with("area", "non-heap"),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_jvm_memory_max_bytes",
            "JVM memory max",
            node.jvm.mem.heap_max_in_bytes,
            tags.with("area", "heap"),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_jvm_memory_committed_bytes",
            "JVM memory currently committed by area",
            node.jvm.mem.heap_committed_in_bytes,
            tags.with("area", "heap"),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_jvm_memory_committed_bytes",
            "JVM memory currently committed by area",
            node.jvm.mem.non_heap_committed_in_bytes,
            tags.with("area", "non-heap"),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_jvm_memory_pool_used_bytes",
            "JVM memory currently used by pool",
            node.jvm
                .mem
                .pools
                .get("young")
                .unwrap_or_default()
                .used_in_bytes,
            tags.with("pool", "young"),
        ),
        Metric::sum_with_tags(
            "elasticsearch_jvm_memory_pool_max_bytes",
            "JVM memory max by pool",
            node.jvm
                .mem
                .pools
                .get("young")
                .unwrap_or_default()
                .max_in_bytes,
            tags.with("pool", "young"),
        ),
        Metric::sum_with_tags(
            "elasticsearch_jvm_memory_pool_peak_used_bytes",
            "JVM memory peak used by pool",
            node.jvm
                .mem
                .pools
                .get("young")
                .unwrap_or_default()
                .peak_used_in_bytes,
            tags.with("pool", "young"),
        ),
        Metric::sum_with_tags(
            "elasticsearch_jvm_memory_pool_peak_max_bytes",
            "JVM memory peak max by pool",
            node.jvm
                .mem
                .pools
                .get("young")
                .unwrap_or_default()
                .peak_max_in_bytes,
            tags.with("pool", "young"),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_jvm_memory_pool_used_bytes",
            "JVM memory currently used by pool",
            node.jvm
                .mem
                .pools
                .get("survivor")
                .unwrap_or_default()
                .used_in_bytes,
            tags.with("pool", "survivor"),
        ),
        Metric::sum_with_tags(
            "elasticsearch_jvm_memory_pool_max_bytes",
            "JVM memory max by pool",
            node.jvm
                .mem
                .pools
                .get("survivor")
                .unwrap_or_default()
                .max_in_bytes,
            tags.with("pool", "survivor"),
        ),
        Metric::sum_with_tags(
            "elasticsearch_jvm_memory_pool_peak_used_bytes",
            "JVM memory peak used by pool",
            node.jvm.mem.pools["survivor"].peak_used_in_bytes,
            tags.with("pool", "survivor"),
        ),
        Metric::sum_with_tags(
            "elasticsearch_jvm_memory_pool_peak_max_bytes",
            "JVM memory peak max by pool",
            node.jvm.mem.pools["survivor"].peak_max_in_bytes,
            tags.with("pool", "survivor"),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_jvm_memory_pool_used_bytes",
            "JVM memory currently used by pool",
            node.jvm
                .mem
                .pools
                .get("old")
                .unwrap_or_default()
                .used_in_bytes,
            tags.with("pool", "old"),
        ),
        Metric::sum_with_tags(
            "elasticsearch_jvm_memory_pool_max_bytes",
            "JVM memory max by pool",
            node.jvm
                .mem
                .pools
                .get("old")
                .unwrap_or_default()
                .max_in_bytes,
            tags.with("pool", "old"),
        ),
        Metric::sum_with_tags(
            "elasticsearch_jvm_memory_pool_peak_used_bytes",
            "JVM memory peak used by pool",
            node.jvm
                .mem
                .pools
                .get("old")
                .unwrap_or_default()
                .peak_used_in_bytes,
            tags.with("pool", "old"),
        ),
        Metric::sum_with_tags(
            "elasticsearch_jvm_memory_pool_peak_max_bytes",
            "JVM memory peak max by pool",
            node.jvm
                .mem
                .pools
                .get("old")
                .unwrap_or_default()
                .peak_max_in_bytes,
            tags.with("pool", "old"),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_jvm_buffer_pool_used_bytes",
            "JVM buffer currently used",
            node.jvm
                .buffer_pools
                .get("direct")
                .unwrap_or_default()
                .used_in_bytes,
            tags.with("type", "direct"),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_jvm_buffer_pool_used_bytes",
            "JVM buffer currently used",
            node.jvm
                .buffer_pools
                .get("mapped")
                .unwrap_or_default()
                .used_in_bytes,
            tags.with("type", "mapped"),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_jvm_uptime_seconds",
            "JVM process uptime in seconds",
            node.jvm.uptime_in_millis / 1000,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_process_cpu_percent",
            "Percent CPU used by process",
            node.process.cpu.percent,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_process_mem_resident_size_bytes",
            "Resident memory in use by process in bytes",
            node.process.mem.resident_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_process_mem_share_size_bytes",
            "Shared memory in use by process in bytes",
            node.process.mem.share_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_process_virtual_size_bytes",
            "Total virtual memory used in bytes",
            node.process.mem.total_virtual_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_process_open_files_count",
            "Open file descriptor",
            node.process.open_file_descriptors,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_process_max_files_descriptors",
            "Max file descriptors",
            node.process.max_file_descriptors,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_process_cpu_seconds_total",
            "Process CPU time in seconds",
            node.process.cpu.total_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_transport_rx_packets_total",
            "Count of packets received",
            node.transport.rx_count,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_transport_rx_size_bytes_total",
            "Total number of bytes received",
            node.transport.rx_size_in_bytes,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_transport_tx_packets_total",
            "Count of packets sent",
            node.transport.tx_count,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_transport_tx_size_bytes_total",
            "Total number of bytes sent",
            node.transport.tx_size_in_bytes,
            tags.clone(),
        ),
    ]
}

fn gc_metrics(tags: Attributes, stats: &JvmGcCollector) -> Vec<Metric> {
    vec![
        Metric::sum_with_tags(
            "elasticsearch_jvm_gc_collection_seconds_count",
            "Count of JVM GC runs",
            stats.collection_count,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_jvm_gc_collection_seconds_sum",
            "GC run time in seconds",
            stats.collection_time_in_millis / 1000,
            tags,
        ),
    ]
}

fn breaker_metrics(tags: Attributes, stats: Breaker) -> Vec<Metric> {
    vec![
        Metric::gauge_with_tags(
            "elasticsearch_breakers_estimated_size_bytes",
            "Estimated size in bytes of breaker",
            stats.estimated_size_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_breakers_limit_size_bytes",
            "Limit size in bytes for breaker",
            stats.limit_size_in_bytes,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_breakers_tripped",
            "Tripped for breaker",
            stats.tripped,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_breakers_overhead",
            "Overhead of circuit breakers",
            stats.overhead,
            tags,
        ),
    ]
}

fn thread_pool_metrics(tags: Attributes, stats: ThreadPool) -> Vec<Metric> {
    vec![
        Metric::sum_with_tags(
            "elasticsearch_thread_pool_completed_count",
            "Thread pool operations completed",
            stats.completed,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_thread_pool_rejected_count",
            "Thread pool operation rejected",
            stats.rejected,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_thread_pool_active_count",
            "Thread pool thread active",
            stats.active,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_thread_pool_largest_count",
            "Thread pool largest threads count",
            stats.largest,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_thread_pool_queue_count",
            "Thread pool operations queued",
            stats.queue,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_thread_pool_thread_count",
            "Thread pool current threads count",
            stats.threads,
            tags,
        ),
    ]
}

fn filesystem_data_metrics(tags: Attributes, stats: FsData) -> Vec<Metric> {
    let mut tags = tags.clone();
    tags.insert("mount", stats.mount);
    tags.insert("path", stats.path);

    vec![
        Metric::gauge_with_tags(
            "elasticsearch_filesystem_data_available_bytes",
            "Available space on block device in bytes",
            stats.available_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_filesystem_data_free_bytes",
            "Free space on block device in bytes",
            stats.free_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_filesystem_data_size_bytes",
            "Size of block device in bytes",
            stats.total_in_bytes,
            tags,
        ),
    ]
}

fn filesystem_io_metrics(tags: Attributes, stats: FsIoStatsDevice) -> Vec<Metric> {
    let mut tags = tags.clone();
    tags.insert("device", stats.device_name);

    vec![
        Metric::sum_with_tags(
            "elasticsearch_filesystem_io_stats_device_operations_count",
            "Count of disk operations",
            stats.operations,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_filesystem_io_stats_device_read_operations_count",
            "Count of disk raed operations",
            stats.read_operations,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_filesystem_io_stats_device_write_operations_count",
            "Count of disk write operations",
            stats.write_operations,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_filesystem_io_stats_device_read_size_kilobytes_sum",
            "Total kilobytes read from disk",
            stats.read_kilobytes,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_filesystem_io_stats_device_write_size_kilobytes_sum",
            "Total kilobytes written to disk",
            stats.write_kilobytes,
            tags,
        ),
    ]
}
