#![allow(dead_code)]

use std::collections::BTreeMap;
use std::time::Instant;

use event::tags::Tags;
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
    #[serde(default)]
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
#[derive(Deserialize, Default)]
#[serde(default)]
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
    #[serde(default)]
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
    #[serde(default)]
    actual_free_in_bytes: i64,
    #[serde(default)]
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
    #[serde(default)]
    uptime_in_millis: i64,
    // LoadAvg was an array of per-cpu values pre-2.0, and is a string in 2.0
    // Leaving this here in case we want to implement parsing logic later.
    //
    cpu: OsCpu,
    mem: OsMem,
    swap: OsSwap,
}

/// `FsData` defines node stats filesystem data structure
#[derive(Deserialize)]
struct FsData {
    path: String,
    mount: String,
    #[serde(default)]
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
    #[serde(default)]
    devices: Vec<FsIoStatsDevice>,
}

#[derive(Deserialize)]
struct Fs {
    timestamp: i64,
    data: Vec<FsData>,
    io_stats: FsIoStats,
}

/// `JvmGcCollector` defines node stats JVM garbage collector collection information.
#[derive(Deserialize)]
struct JvmGcCollector {
    collection_count: i64,
    collection_time_in_millis: i64,
}

/// `JvmGc` defines node stats JVM garbage collector information.
#[derive(Deserialize)]
struct JvmGc {
    collectors: BTreeMap<String, JvmGcCollector>,
}

/// `JvmMemPool` defines node status JVM memory pool information.
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
    #[serde(default)]
    resident_in_bytes: i64,
    #[serde(default)]
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
    overhead: f64,
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

#[derive(Deserialize)]
struct HttpClient {
    id: i64,
}

#[derive(Deserialize)]
struct Http {
    #[serde(default)]
    clients: Vec<HttpClient>,
}

/// `NodeStats` defines node stats information structure for nodes
#[allow(dead_code)]
#[derive(Deserialize)]
struct NodeStats {
    name: String,
    host: String,
    timestamp: i64,
    transport_address: String,
    #[serde(default)]
    hostname: String,
    roles: Vec<String>,
    #[serde(default)]
    attributes: BTreeMap<String, String>,
    indices: Indices,
    os: Os,
    fs: Fs,
    thread_pool: BTreeMap<String, ThreadPool>,
    jvm: Jvm,
    breakers: BTreeMap<String, Breaker>,
    #[serde(default)]
    http: Option<Http>,
    transport: Transport,
    process: Process,
}

fn get_roles(node: &NodeStats) -> Vec<String> {
    // default settings(2.x) and map, which roles to consider
    let mut roles = vec!["client".to_string()];

    // assumption: a 5.x node has at least one role, otherwise it's a
    // 1.7 or 2.x node.
    if !node.roles.is_empty() {
        for role in node.roles.iter() {
            if role == "master" || role == "data" || role == "ingest" || role == "client" {
                roles.push(role.to_string());
            }
        }
    } else {
        for (role, setting) in &node.attributes {
            if !roles.contains(role) {
                continue;
            }

            if setting == "false" {
                roles.retain(|x| x != role)
            }
        }
    }

    if node.http.is_none() {
        roles.retain(|x| x != "client");
    }

    roles
}

/// `NodeStatsResp` is a representation of an Elasticsearch Node Stats.
#[derive(Deserialize)]
struct NodeStatsResp {
    cluster_name: String,
    nodes: BTreeMap<String, NodeStats>,
}

impl Elasticsearch {
    pub async fn node_stats(&self, node: &str) -> Vec<Metric> {
        let url = format!("/_nodes/{}/stats", node);
        let start = Instant::now();
        let result = self.fetch::<NodeStatsResp>(url.as_str()).await;
        let elapsed = start.elapsed().as_secs_f64();
        let up = result.is_ok();

        let mut metrics = match result {
            Ok(stats) => {
                let mut metrics = vec![];

                for (_name, node) in stats.nodes {
                    let roles = get_roles(&node);

                    for role in ["master", "data", "client", "ingest"] {
                        if roles.iter().any(|s| s == role) {
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

                    let es_master_node = roles.iter().any(|s| s == "master").to_string();
                    let es_data_node = roles.iter().any(|s| s == "data").to_string();
                    let es_ingest_node = roles.iter().any(|s| s == "ingest").to_string();
                    let es_client_node = roles.iter().any(|s| s == "client").to_string();

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

                    // Jvm stats
                    metrics.extend(jvm_metrics(tags.clone(), node.jvm));

                    // Process stats
                    metrics.extend(process_metrics(tags.clone(), node.process));

                    // transport stats
                    metrics.extend(transport_metrics(tags.clone(), node.transport));

                    // Indices
                    metrics.extend(indices_metrics(tags.clone(), node.indices));

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

                metrics
            }
            Err(err) => {
                warn!(message = "Fetch node stats failed", ?err);

                vec![]
            }
        };

        metrics.extend_from_slice(&[
            Metric::gauge_with_tags(
                "elasticsearch_node_stats_up",
                "Was the last scrape of the Elasticsearch nodes endpoint successful",
                up,
                tags!(
                    "node" => node.to_string(),
                ),
            ),
            Metric::gauge_with_tags(
                "elasticsearch_node_scrape_duration_seconds",
                "Duration of scraping node stats",
                elapsed,
                tags!(
                    "node" => node.to_string(),
                ),
            ),
        ]);

        metrics
    }
}

fn os_metrics(tags: Tags, stats: Os) -> Vec<Metric> {
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

fn indices_metrics(tags: Tags, indices: Indices) -> Vec<Metric> {
    vec![
        Metric::gauge_with_tags(
            "elasticsearch_indices_fielddata_memory_size_bytes",
            "Field data cache memory usage in bytes",
            indices.field_data.memory_size_in_bytes,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_fielddata_evictions",
            "Evictions from field data",
            indices.field_data.evictions,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_completion_size_in_bytes",
            "Completion in bytes",
            indices.completion.size_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_filter_cache_memory_size_bytes",
            "Filter cache memory usage in bytes",
            indices.filter_cache.memory_size_in_bytes,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_filter_cache_evictions",
            "Evictions from filter cache",
            indices.filter_cache.evictions,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_query_cache_memory_size_bytes",
            "Query cache memory usage in bytes",
            indices.query_cache.memory_size_in_bytes,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_query_cache_evictions",
            "Evictions from query cache",
            indices.query_cache.evictions,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_query_cache_total",
            "Query cache total count",
            indices.query_cache.total_count,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_query_cache_cache_size",
            "Query cache cache size",
            indices.query_cache.cache_size,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_query_cache_cache_total",
            "Query cache cache count",
            indices.query_cache.cache_count,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_query_cache_count",
            "Query cache count",
            indices.query_cache.hit_count,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_query_miss_count",
            "Query miss count",
            indices.query_cache.miss_count,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_request_cache_memory_size_bytes",
            "Request cache memory usage in bytes",
            indices.request_cache.memory_size_in_bytes,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_request_cache_evictions",
            "Evictions from request cache",
            indices.request_cache.evictions,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_request_cache_count",
            "Request cache count",
            indices.request_cache.hit_count,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_request_miss_count",
            "Request miss count",
            indices.request_cache.miss_count,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_translog_operations",
            "Total translog operations",
            indices.translog.operations,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_translog_size_in_bytes",
            "Total translog size in bytes",
            indices.translog.size_in_bytes,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_get_time_seconds",
            "Total get time in seconds",
            indices.get.time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_get_total",
            "Total get",
            indices.get.total,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_get_missing_time_seconds",
            "Total time of get missing in seconds",
            indices.get.missing_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_get_missing_total",
            "Total get missing",
            indices.get.missing_total,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_get_exists_time_seconds",
            "Total time get exists in seconds",
            indices.get.exists_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_get_exists_total",
            "Total get exists operation",
            indices.get.exists_total,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_refresh_time_seconds_total",
            "Total time spent refreshing in seconds",
            indices.refresh.total_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_refresh_total",
            "Total refreshes",
            indices.refresh.total,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_search_query_time_seconds",
            "Total search query time in seconds",
            indices.search.query_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_search_query_total",
            "Total number of queries",
            indices.search.query_total,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_search_fetch_time_seconds",
            "Total search fetch time in seconds",
            indices.search.fetch_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_search_fetch_total",
            "Total number of fetches",
            indices.search.fetch_total,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_search_suggest_total",
            "Total number of suggests",
            indices.search.suggest_total,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_search_suggest_time_seconds",
            "Total suggest time in seconds",
            indices.search.suggest_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_search_scroll_total",
            "Total number of scrolls",
            indices.search.scroll_total,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_search_scroll_time_seconds",
            "Total scroll time in seconds",
            indices.search.scroll_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_docs",
            "Count of documents on this node",
            indices.docs.count,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_docs_deleted",
            "Count of deleted documents on this node",
            indices.docs.deleted,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_store_size_bytes",
            "Current size of stored index data in bytes",
            indices.store.size_in_bytes,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_store_throttle_time_seconds_total",
            "Throttle time for index store in seconds",
            indices.store.throttle_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_segments_memory_bytes",
            "Current memory size of segments in bytes",
            indices.segments.memory_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_segments_count",
            "Count of index segments on this node",
            indices.segments.count,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_segments_terms_memory_in_bytes",
            "Count of terms in memory for this node",
            indices.segments.terms_memory_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_segments_index_writer_memory_in_bytes",
            "Count of memory for index writer on this node",
            indices.segments.index_writer_memory_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_segments_norms_memory_in_bytes",
            "Count of memory used by norms",
            indices.segments.norms_memory_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_segments_stored_fields_memory_in_bytes",
            "Count of stored fields memory",
            indices.segments.stored_fields_memory_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_segments_doc_values_memory_in_bytes",
            "Count of doc values memory",
            indices.segments.doc_values_memory_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_segments_fixed_bit_set_memory_in_bytes",
            "count of fixed bit set",
            indices.segments.fixed_bit_set_memory_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_segments_term_vectors_memory_in_bytes",
            "term vectors memory usage in bytes",
            indices.segments.term_vectors_memory_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_segments_points_memory_in_bytes",
            "Point values memory usage in bytes",
            indices.segments.points_memory_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_segments_version_map_memory_in_bytes",
            "Version map memory usage in bytes",
            indices.segments.version_map_memory_in_bytes,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_flush_total",
            "Total flushes",
            indices.flush.total,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_flush_time_seconds",
            "Cumulative flush time in seconds",
            indices.flush.total_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_warmer_total",
            "total warmer count",
            indices.warmer.total,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_warmer_time_seconds_total",
            "Total warmer time in seconds",
            indices.warmer.total_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_indexing_index_time_seconds_total",
            "Cumulative index time in seconds",
            indices.indexing.index_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_indexing_index_total",
            "Total index calls",
            indices.indexing.index_total,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_indexing_delete_time_seconds_total",
            "Total time indexing delete in seconds",
            indices.indexing.delete_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_indexing_delete_total",
            "Total indexing deletes",
            indices.indexing.delete_total,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_indexing_is_throttled",
            "Indexing throttling",
            indices.indexing.is_throttled,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_indexing_throttle_time_seconds_total",
            "Cumulative indexing throttling time",
            indices.indexing.throttle_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_merges_total",
            "Total merges",
            indices.merges.total,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_merges_current",
            "Current merge",
            indices.merges.current,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_indices_merges_current_size_in_bytes",
            "Size of a current merges in bytes",
            indices.merges.current_size_in_bytes,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_merges_docs_total",
            "Cumulative docs merged",
            indices.merges.total_docs,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_merges_total_size_bytes_total",
            "Total merge size in bytes",
            indices.merges.total_size_in_bytes,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_merges_total_time_seconds_total",
            "Total time spent merging in seconds",
            indices.merges.total_time_in_millis / 1000,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_indices_merges_total_throttled_time_seconds_total",
            "Total throttled time of merges in seconds",
            indices.merges.total_throttled_time_in_millis / 1000,
            tags,
        ),
    ]
}

fn jvm_metrics(tags: Tags, mut jvm: Jvm) -> Vec<Metric> {
    let young = jvm.mem.pools.remove("young").unwrap_or_default();
    let old = jvm.mem.pools.remove("old").unwrap_or_default();
    let survivor = jvm.mem.pools.remove("survivor").unwrap_or_default();

    let mut metrics = vec![
        Metric::gauge_with_tags(
            "elasticsearch_jvm_memory_used_bytes",
            "JVM memory currently used by area",
            jvm.mem.heap_used_in_bytes,
            tags.with("area", "heap"),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_jvm_memory_used_bytes",
            "JVM memory currently used by area",
            jvm.mem.non_heap_used_in_bytes,
            tags.with("area", "non-heap"),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_jvm_memory_max_bytes",
            "JVM memory max",
            jvm.mem.heap_max_in_bytes,
            tags.with("area", "heap"),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_jvm_memory_committed_bytes",
            "JVM memory currently committed by area",
            jvm.mem.heap_committed_in_bytes,
            tags.with("area", "heap"),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_jvm_memory_committed_bytes",
            "JVM memory currently committed by area",
            jvm.mem.non_heap_committed_in_bytes,
            tags.with("area", "non-heap"),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_jvm_memory_pool_used_bytes",
            "JVM memory currently used by pool",
            young.used_in_bytes,
            tags.with("pool", "young"),
        ),
        Metric::sum_with_tags(
            "elasticsearch_jvm_memory_pool_max_bytes",
            "JVM memory max by pool",
            young.max_in_bytes,
            tags.with("pool", "young"),
        ),
        Metric::sum_with_tags(
            "elasticsearch_jvm_memory_pool_peak_used_bytes",
            "JVM memory peak used by pool",
            young.peak_used_in_bytes,
            tags.with("pool", "young"),
        ),
        Metric::sum_with_tags(
            "elasticsearch_jvm_memory_pool_peak_max_bytes",
            "JVM memory peak max by pool",
            young.peak_max_in_bytes,
            tags.with("pool", "young"),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_jvm_memory_pool_used_bytes",
            "JVM memory currently used by pool",
            survivor.used_in_bytes,
            tags.with("pool", "survivor"),
        ),
        Metric::sum_with_tags(
            "elasticsearch_jvm_memory_pool_max_bytes",
            "JVM memory max by pool",
            survivor.max_in_bytes,
            tags.with("pool", "survivor"),
        ),
        Metric::sum_with_tags(
            "elasticsearch_jvm_memory_pool_peak_used_bytes",
            "JVM memory peak used by pool",
            survivor.peak_used_in_bytes,
            tags.with("pool", "survivor"),
        ),
        Metric::sum_with_tags(
            "elasticsearch_jvm_memory_pool_peak_max_bytes",
            "JVM memory peak max by pool",
            survivor.peak_max_in_bytes,
            tags.with("pool", "survivor"),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_jvm_memory_pool_used_bytes",
            "JVM memory currently used by pool",
            old.used_in_bytes,
            tags.with("pool", "old"),
        ),
        Metric::sum_with_tags(
            "elasticsearch_jvm_memory_pool_max_bytes",
            "JVM memory max by pool",
            old.max_in_bytes,
            tags.with("pool", "old"),
        ),
        Metric::sum_with_tags(
            "elasticsearch_jvm_memory_pool_peak_used_bytes",
            "JVM memory peak used by pool",
            old.peak_used_in_bytes,
            tags.with("pool", "old"),
        ),
        Metric::sum_with_tags(
            "elasticsearch_jvm_memory_pool_peak_max_bytes",
            "JVM memory peak max by pool",
            old.peak_max_in_bytes,
            tags.with("pool", "old"),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_jvm_buffer_pool_used_bytes",
            "JVM buffer currently used",
            old.used_in_bytes,
            tags.with("type", "direct"),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_jvm_buffer_pool_used_bytes",
            "JVM buffer currently used",
            old.used_in_bytes,
            tags.with("type", "mapped"),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_jvm_uptime_seconds",
            "JVM process uptime in seconds",
            jvm.uptime_in_millis / 1000,
            tags.clone(),
        ),
    ];

    // GC stats
    for (name, collector) in jvm.gc.collectors {
        metrics.extend(gc_metrics(tags.with("gc", name), collector));
    }

    metrics
}

fn process_metrics(tags: Tags, process: Process) -> Vec<Metric> {
    vec![
        Metric::gauge_with_tags(
            "elasticsearch_process_cpu_percent",
            "Percent CPU used by process",
            process.cpu.percent,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_process_mem_resident_size_bytes",
            "Resident memory in use by process in bytes",
            process.mem.resident_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_process_mem_share_size_bytes",
            "Shared memory in use by process in bytes",
            process.mem.share_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_process_virtual_size_bytes",
            "Total virtual memory used in bytes",
            process.mem.total_virtual_in_bytes,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_process_open_files_count",
            "Open file descriptor",
            process.open_file_descriptors,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "elasticsearch_process_max_files_descriptors",
            "Max file descriptors",
            process.max_file_descriptors,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_process_cpu_seconds_total",
            "Process CPU time in seconds",
            process.cpu.total_in_millis / 1000,
            tags,
        ),
    ]
}

fn transport_metrics(tags: Tags, transport: Transport) -> Vec<Metric> {
    vec![
        Metric::sum_with_tags(
            "elasticsearch_transport_rx_packets_total",
            "Count of packets received",
            transport.rx_count,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_transport_rx_size_bytes_total",
            "Total number of bytes received",
            transport.rx_size_in_bytes,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_transport_tx_packets_total",
            "Count of packets sent",
            transport.tx_count,
            tags.clone(),
        ),
        Metric::sum_with_tags(
            "elasticsearch_transport_tx_size_bytes_total",
            "Total number of bytes sent",
            transport.tx_size_in_bytes,
            tags,
        ),
    ]
}

fn gc_metrics(tags: Tags, stats: JvmGcCollector) -> Vec<Metric> {
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

fn breaker_metrics(tags: Tags, stats: Breaker) -> Vec<Metric> {
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

fn thread_pool_metrics(tags: Tags, stats: ThreadPool) -> Vec<Metric> {
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

fn filesystem_data_metrics(mut tags: Tags, stats: FsData) -> Vec<Metric> {
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

fn filesystem_io_metrics(mut tags: Tags, stats: FsIoStatsDevice) -> Vec<Metric> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::trace_init;
    use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};
    use framework::config::ProxyConfig;
    use framework::http::Auth;
    use http::{Method, Request, Response};
    use hyper::service::{make_service_fn, service_fn};
    use hyper::Body;
    use std::sync::Arc;
    use std::time::Duration;
    use testify::http::{file_send, not_found, unauthorized};
    use testify::pick_unused_local_port;

    struct Context {
        version: &'static str,
        auth: Option<(&'static str, &'static str)>,
    }

    async fn handle(req: Request<Body>, cx: Arc<Context>) -> hyper::Result<Response<Body>> {
        if req.method() != Method::GET {
            return Ok(not_found());
        }

        if let Some((username, password)) = cx.auth {
            let av = match req.headers().get("Authorization") {
                Some(value) => value,
                None => return Ok(unauthorized()),
            };

            let n = av
                .to_str()
                .unwrap()
                .split_ascii_whitespace()
                .last()
                .unwrap();
            let d = BASE64_STANDARD.decode(n).unwrap();
            let decoded = std::str::from_utf8(&d).unwrap();
            let (k, v) = decoded.split_once(':').unwrap();
            if k != username || v != password {
                return Ok(unauthorized());
            }
        }

        file_send(format!(
            "tests/fixtures/elasticsearch/nodestats/{}.json",
            cx.version
        ))
        .await
    }

    async fn start_server_and_fetch(cx: Arc<Context>) {
        let version = cx.version;
        let auth = cx
            .auth
            .map(|(user, password)| Auth::basic(user.into(), password.into()));
        let port = pick_unused_local_port();
        let endpoint = format!("127.0.0.1:{}", port);
        let service = make_service_fn(move |_conn| {
            let cx = Arc::clone(&cx);

            async move {
                let cx = Arc::clone(&cx);

                Ok::<_, hyper::Error>(service_fn(move |req| {
                    let cx = Arc::clone(&cx);

                    async move { handle(req, cx).await }
                }))
            }
        });
        let addr = endpoint.parse().unwrap();
        let server = hyper::Server::bind(&addr).serve(service);

        tokio::spawn(async move {
            if let Err(err) = server.await {
                error!(message = "server error", ?err);
            }
        });

        // wait for http server start
        tokio::time::sleep(Duration::from_secs(1)).await;

        let http_client = framework::http::HttpClient::new(&None, &ProxyConfig::default()).unwrap();
        let es = Elasticsearch {
            endpoint: format!("http://{}", endpoint),
            http_client,
            auth,
            slm: false,
            snapshot: false,
        };

        let metrics = es.node_stats("_all").await;
        assert!(metrics.len() > 2, "version: {}", version);
    }

    #[tokio::test]
    async fn node_stats() {
        trace_init();

        for version in [
            "5.4.2", "5.6.16", "6.5.4", "6.8.8", "7.3.0", "7.6.2", "7.13.1",
        ] {
            for auth in [None, Some(("elastic", "changeme"))] {
                let cx = Context { version, auth };

                start_server_and_fetch(Arc::new(cx)).await
            }
        }
    }

    #[test]
    fn decode_5() {
        use bytes::Buf;

        let data = std::fs::read("tests/fixtures/elasticsearch/nodestats/5.4.2.json").unwrap();
        let xd = &mut serde_json::Deserializer::from_reader(data.reader());
        let result: Result<NodeStatsResp, _> = serde_path_to_error::deserialize(xd);
        if let Err(err) = result {
            let inner = err.inner();
            let path = err.path();
            panic!("{} {:?}", path, inner)
        }
    }
}
