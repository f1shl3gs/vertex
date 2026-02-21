use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

use bytes::Bytes;
use event::tags::Tags;
use event::{Metric, Quantile, tags};

use super::Error;
use super::connection::Connection;
use super::sentinel;

static GAUGE_METRICS: LazyLock<BTreeMap<&'static str, (&'static str, &'static str)>> =
    LazyLock::new(|| {
        let mut m = BTreeMap::new();

        // # Server
        m.insert(
            "uptime_in_seconds",
            (
                "uptime_in_seconds",
                "Number of seconds since Redis server start",
            ),
        );
        m.insert("process_id", ("process_id", "PID of the server process"));
        m.insert(
            "io_threads_active",
            (
                "io_threads_active",
                "Flag indicating if I/O threads are active",
            ),
        );

        // # Clients
        m.insert(
            "connected_clients",
            (
                "connected_clients",
                "Number of client connections (excluding connections from replicas)",
            ),
        );
        m.insert(
            "blocked_clients",
            (
                "blocked_clients",
                "Number of clients pending on a blocking call",
            ),
        );
        m.insert("maxclients", ("max_clients", "The value of the maxclients configuration directive. This is the upper limit for the sum of connected_clients, connected_slaves and cluster_connections."));
        m.insert(
            "tracking_clients",
            ("tracking_clients", "Number of clients being tracked"),
        );
        m.insert(
            "clients_in_timeout_table",
            (
                "clients_in_timeout_table",
                "Number of clients in the clients timeout table",
            ),
        );
        m.insert(
            "pubsub_clients",
            ("pubsub_clients", "Number of clients in pubsub mode"),
        ); // Added in Redis 7.4
        m.insert(
            "watching_clients",
            ("watching_clients", "Number of clients in watching mode"),
        ); // Added in Redis 7.4
        m.insert(
            "total_watched_keys",
            ("total_watched_keys", "Number of watched keys"),
        ); // Added in Redis 7.4
        m.insert(
            "total_blocking_keys",
            ("total_blocking_keys", "Number of blocking keys"),
        ); // Added in Redis 7.2
        m.insert(
            "total_blocking_keys_on_nokey",
            ("total_blocking_keys_on_nokey", "Number of blocking keys that one or more clients that would like to be unblocked when the key is deleted"),
        ); // Added in Redis 7.2

        // redis 2,3,4.x
        m.insert(
            "client_longest_output_list",
            ("client_longest_output_list", ""),
        );
        m.insert("client_biggest_input_buf", ("client_biggest_input_buf", ""));

        // the above two metrics were renamed in redis 5.x
        m.insert(
            "client_recent_max_output_buffer",
            (
                "client_recent_max_output_buffer_bytes",
                "Biggest output buffer among current client connections",
            ),
        );
        m.insert(
            "client_recent_max_input_buffer",
            (
                "client_recent_max_input_buffer_bytes",
                "Biggest input buffer among current client connections",
            ),
        );

        // # Memory
        m.insert(
            "allocator_active",
            (
                "allocator_active_bytes",
                "Total bytes in the allocator active pages, this includes external-fragmentation",
            ),
        );
        m.insert("allocator_allocated", ("allocator_allocated_bytes", "Total bytes allocated form the allocator, including internal-fragmentation. Normally the same as used_memory"));
        m.insert("allocator_resident", ("allocator_resident_bytes", "Total bytes resident (RSS) in the allocator, this includes pages that can be released to the OS (by MEMORY PURGE, or just waiting)"));
        m.insert("allocator_frag_ratio", ("allocator_frag_ratio", "Ratio between allocator_active and allocator_allocated. This is the true (external) fragmentation metric (not mem_fragmentation_ratio)"));
        m.insert(
            "allocator_frag_bytes",
            (
                "allocator_frag_bytes",
                "Delta between allocator_active and allocator_allocated",
            ),
        );
        m.insert(
            "allocator_muzzy",
            (
                "allocator_muzzy_bytes",
                "Total bytes of 'muzzy' memory (RSS) in the allocator. Muzzy memory is memory that has been freed, but not yet fully returned to the operating system. It can be reused immediately when needed or reclaimed by the OS when system pressure increases."
            ),
        );
        m.insert("allocator_rss_ratio", ("allocator_rss_ratio", "Ratio between allocator_resident and allocator_active. This usually indicates pages that the allocator can and probably will soon release back to the OS"));
        m.insert(
            "allocator_rss_bytes",
            (
                "allocator_rss_bytes",
                "Delta between allocator_resident and allocator_active",
            ),
        );

        m.insert(
            "used_memory",
            (
                "memory_used_bytes",
                "Total number of bytes allocated by Redis using its allocator",
            ),
        );
        m.insert("used_memory_rss", ("memory_used_rss_bytes", "Number of bytes that Redis allocated as seen by the operating system (a.k.a resident set size)"));
        m.insert(
            "used_memory_peak",
            (
                "memory_used_peak_bytes",
                "Peak memory consumed by Redis in bytes",
            ),
        );
        // Deprecated in Redis 7.0, renamed to used_memory_vm_eval
        m.insert(
            "used_memory_lua",
            (
                "memory_used_lua_bytes",
                "Number of bytes used by the Lua engine for EVAL scripts",
            ),
        );
        // Added in Redis 7.0
        m.insert("used_memory_vm_eval", ("memory_used_vm_eval_bytes", "Number of bytes used by the script VM engines for EVAL framework (not part of used_memory)"));
        // Added in Redis 7.0
        m.insert(
            "used_memory_scripts_eval",
            (
                "memory_used_scripts_eval_bytes",
                "Number of bytes overhead by the EVAL scripts (part of used_memory)",
            ),
        );
        m.insert("used_memory_overhead", ("memory_used_overhead_bytes", "The sum in bytes of all overheads that the server allocated for managing its internal data structures"));
        m.insert(
            "used_memory_startup",
            (
                "memory_used_startup_bytes",
                "Initial amount of memory consumed by Redis at startup in bytes",
            ),
        );
        m.insert("used_memory_dataset", ("memory_used_dataset_bytes", "The size in bytes of the dataset (used_memory_overhead subtracted from used_memory)"));
        // Added in Redis 7.0
        m.insert(
            "number_of_cached_scripts",
            (
                "number_of_cached_scripts",
                "The number of EVAL scripts cached by the server",
            ),
        );
        // Added in Redis 7.0
        m.insert(
            "number_of_functions",
            ("number_of_functions", "The number of functions"),
        );
        // Added in Redis 7.4
        m.insert(
            "number_of_libraries",
            ("number_of_libraries", "The number of libraries"),
        );
        // Added in Redis 7.0
        m.insert("used_memory_vm_functions", ("memory_used_vm_functions_bytes", "Number of bytes used by the script VM engines for Functions framework (not part of used_memory)"));
        // Added in Redis 7.0
        m.insert(
            "used_memory_scripts",
            (
                "memory_used_scripts_bytes",
                "used_memory_scripts_eval + used_memory_functions (part of used_memory)",
            ),
        );
        // Added in Redis 7.0
        m.insert(
            "used_memory_functions",
            (
                "memory_used_functions_bytes",
                "Number of bytes overhead by Function scripts (part of used_memory)",
            ),
        );
        // Added in Redis 7.0
        m.insert(
            "used_memory_vm_total",
            (
                "memory_used_vm_total",
                "used_memory_vm_eval + used_memory_vm_functions (not part of used_memory)",
            ),
        );
        m.insert(
            "maxmemory",
            (
                "memory_max_bytes",
                " The value of the maxmemory configuration directive",
            ),
        );

        m.insert(
            "maxmemory_reservation",
            ("memory_max_reservation_bytes", ""),
        );
        m.insert(
            "maxmemory_desired_reservation",
            ("memory_max_reservation_desired_bytes", ""),
        );

        // Azure Cache for Redis
        m.insert(
            "maxfragmentationmemory_reservation",
            ("memory_max_fragmentation_reservation_bytes", ""),
        );
        // Azure Cache for Redis
        m.insert(
            "maxfragmentationmemory_desired_reservation",
            ("memory_max_fragmentation_reservation_desired_bytes", ""),
        );

        m.insert("mem_fragmentation_ratio", ("mem_fragmentation_ratio", "Ratio between used_memory_rss and used_memory. Note that this doesn't only includes fragmentation, but also other process overheads (see the allocator_* metrics), and also overheads like code, shared libraries, stack, etc"));
        m.insert("mem_fragmentation_bytes", ("mem_fragmentation_bytes", "Delta between used_memory_rss and used_memory. Note that when the total fragmentation bytes is low (few megabytes), a high ratio (e.g. 1.5 and above) is not an indication of an issue"));
        m.insert("mem_clients_slaves", ("mem_clients_slaves", "Memory used by replica clients - Starting Redis 7.0, replica buffers share memory with the replication backlog, so this field can show 0 when replicas don't trigger an increase of memory usage"));
        m.insert(
            "mem_clients_normal",
            ("mem_clients_normal", "Memory used by normal clients"),
        );
        m.insert(
            "mem_cluster_links",
            (
                "mem_cluster_links_bytes",
                "Memory used by links to peers on the cluster bus when cluster mode is enabled",
            ),
        );
        m.insert(
            "mem_aof_buffer",
            (
                "mem_aof_buffer_bytes",
                "Transient memory used for AOF and AOF rewrite buffers",
            ),
        );
        m.insert(
            "mem_replication_backlog",
            (
                "mem_replication_backlog_bytes",
                "Memory used by replication backlog",
            ),
        );

        m.insert(
            "expired_stale_perc",
            (
                "expired_stale_percentage",
                "The percentage of keys probably expired",
            ),
        );

        // https://github.com/antirez/redis/blob/17bf0b25c1171486e3a1b089f3181fff2bc0d4f0/src/evict.c#L349-L352
        // ... the sum of AOF and slaves buffer ...
        m.insert(
            "mem_not_counted_for_evict",
            ("mem_not_counted_for_eviction_bytes", "Used memory that's not counted for key eviction. This is basically transient replica and AOF buffers")
        );
        // Added in Redis 7.0
        m.insert(
            "mem_total_replication_buffers",
            (
                "mem_total_replication_buffers_bytes",
                "Total memory consumed for replication buffers",
            ),
        );
        // Added in Redis 7.4
        m.insert(
            "mem_overhead_db_hashtable_rehashing",
            (
                "mem_overhead_db_hashtable_rehashing_bytes",
                "Temporary memory overhead of database dictionaries currently being rehashed",
            ),
        );

        m.insert(
            "lazyfree_pending_objects",
            (
                "lazyfree_pending_objects",
                "The number of objects waiting to be freed",
            ),
        );
        m.insert(
            "lazyfreed_objects",
            (
                "lazyfreed_objects",
                "The number of objects that have been lazy freed",
            ),
        );
        m.insert("active_defrag_running", ("active_defrag_running", "When activedefrag is enabled, this indicates whether defragmentation is currently active, and the CPU percentage it intends to utilize"));

        m.insert(
            "migrate_cached_sockets",
            (
                "migrate_cached_sockets_total",
                "The number of sockets open for MIGRATE purposes",
            ),
        );

        m.insert(
            "active_defrag_hits",
            (
                "defrag_hits",
                "Number of value reallocations performed by active the defragmentation process",
            ),
        );
        m.insert("active_defrag_misses", ("defrag_misses", "Number of aborted value reallocations started by the active defragmentation process"));
        m.insert(
            "active_defrag_key_hits",
            (
                "defrag_key_hits",
                "Number of keys that were actively defragmented",
            ),
        );
        m.insert(
            "active_defrag_key_misses",
            (
                "defrag_key_misses",
                "Number of keys that were skipped by the active defragmentation process",
            ),
        );

        // https://github.com/antirez/redis/blob/0af467d18f9d12b137af3b709c0af579c29d8414/src/expire.c#L297-L299
        m.insert(
            "expired_time_cap_reached_count",
            (
                "expired_time_cap_reached_total",
                "The count of times that active expiry cycles have stopped early",
            ),
        );

        // # Persistence
        m.insert(
            "loading",
            (
                "loading_dump_file",
                "Flag indicating if the load of a dump file is on-going",
            ),
        );
        // Added in Redis 7.0
        m.insert("async_loading", ("async_loading", "Currently loading replication data-set asynchronously while serving old data. This means repl-diskless-load is enabled and set to swapdb"));
        m.insert(
            "rdb_changes_since_last_save",
            (
                "rdb_changes_since_last_save",
                "Number of changes since the last dump",
            ),
        );
        m.insert(
            "rdb_bgsave_in_progress",
            (
                "rdb_bgsave_in_progress",
                "Flag indicating a RDB save is on-going",
            ),
        );
        m.insert(
            "rdb_last_save_time",
            (
                "rdb_last_save_timestamp_seconds",
                "Epoch-based timestamp of last successful RDB save",
            ),
        );
        m.insert(
            "rdb_last_bgsave_status",
            (
                "rdb_last_bgsave_status",
                "Status of the last RDB save operation",
            ),
        );
        m.insert(
            "rdb_last_bgsave_time_sec",
            (
                "rdb_last_bgsave_duration_sec",
                "Duration of the last RDB save operation in seconds",
            ),
        );
        m.insert(
            "rdb_current_bgsave_time_sec",
            (
                "rdb_current_bgsave_duration_sec",
                "Duration of the on-going RDB save operation if any",
            ),
        );
        m.insert(
            "rdb_saves",
            (
                "rdb_saves_total",
                "Number of RDB snapshots performed since startup",
            ),
        );
        m.insert(
            "rdb_last_cow_size",
            (
                "rdb_last_cow_size_bytes",
                "The size in bytes of copy-on-write memory during the last RDB save operation",
            ),
        );
        // Added in Redis 7.0
        m.insert(
            "rdb_last_load_keys_expired",
            (
                "rdb_last_load_expired_keys",
                "Number of volatile keys deleted during the last RDB loading",
            ),
        );
        // Added in Redis 7.0
        m.insert(
            "rdb_last_load_keys_loaded",
            (
                "rdb_last_load_loaded_keys",
                "Number of keys loaded during the last RDB loading",
            ),
        );
        m.insert(
            "aof_enabled",
            ("aof_enabled", "Flag indicating AOF logging is activated"),
        );
        m.insert(
            "aof_rewrite_in_progress",
            (
                "aof_rewrite_in_progress",
                "Flag indicating a AOF rewrite operation is on-going",
            ),
        );
        m.insert("aof_rewrite_scheduled", ("aof_rewrite_scheduled", "Flag indicating an AOF rewrite operation will be scheduled once the on-going RDB save is complete"));
        m.insert(
            "aof_last_rewrite_time_sec",
            (
                "aof_last_rewrite_duration_sec",
                "Duration of the last AOF rewrite operation in seconds",
            ),
        );
        m.insert(
            "aof_current_rewrite_time_sec",
            (
                "aof_current_rewrite_duration_sec",
                "Duration of the on-going AOF rewrite operation if any",
            ),
        );
        m.insert(
            "aof_last_cow_size",
            (
                "aof_last_cow_size_bytes",
                "The size in bytes of copy-on-write memory during the last AOF rewrite operation",
            ),
        );
        m.insert(
            "aof_current_size",
            ("aof_current_size_bytes", "AOF current file size"),
        );
        m.insert(
            "aof_base_size",
            (
                "aof_base_size_bytes",
                "AOF file size on latest startup or rewrite",
            ),
        );
        m.insert("aof_pending_rewrite", ("aof_pending_rewrite", "Flag indicating an AOF rewrite operation will be scheduled once the on-going RDB save is complete"));
        m.insert(
            "aof_buffer_length",
            ("aof_buffer_length", "Size of the AOF buffer"),
        );
        // Removed in Redis 7.0
        m.insert(
            "aof_rewrite_buffer_length",
            (
                "aof_rewrite_buffer_length",
                "Size of the AOF rewrite buffer",
            ),
        );
        m.insert(
            "aof_pending_bio_fsync",
            (
                "aof_pending_bio_fsync",
                "Number of fsync pending jobs in background I/O queue",
            ),
        );
        m.insert(
            "aof_delayed_fsync",
            ("aof_delayed_fsync", "Delayed fsync counter"),
        );
        m.insert(
            "aof_last_bgrewrite_status",
            (
                "aof_last_bgrewrite_status",
                "Status of the last AOF rewrite operation",
            ),
        );
        m.insert(
            "aof_last_write_status",
            (
                "aof_last_write_status",
                "Status of the last write operation to the AOF",
            ),
        );
        m.insert(
            "module_fork_in_progress",
            (
                "module_fork_in_progress",
                "Flag indicating a module fork is on-going",
            ),
        );
        m.insert(
            "module_fork_last_cow_size",
            (
                "module_fork_last_cow_size",
                "The size in bytes of copy-on-write memory during the last module fork operation",
            ),
        );

        // # Stats
        m.insert(
            "current_eviction_exceeded_time",
            (
                "current_eviction_exceeded_time_ms",
                "The time passed since used_memory last rose above maxmemory, in milliseconds",
            ),
        );
        m.insert(
            "pubsub_channels",
            (
                "pubsub_channels",
                "Global number of pub/sub channels with client subscriptions",
            ),
        );
        m.insert(
            "pubsub_patterns",
            (
                "pubsub_patterns",
                "Global number of pub/sub pattern with client subscriptions",
            ),
        );
        // Added in Redis 7.0.3
        m.insert(
            "pubsubshard_channels",
            (
                "pubsubshard_channels",
                "Global number of pub/sub shard channels with client subscriptions",
            ),
        );
        m.insert(
            "latest_fork_usec",
            (
                "latest_fork_usec",
                "Duration of the latest fork operation in microseconds",
            ),
        );
        m.insert(
            "tracking_total_keys",
            (
                "tracking_total_keys",
                "Number of keys being tracked by the server",
            ),
        );
        m.insert("tracking_total_items", ("tracking_total_items", "Number of items, that is the sum of clients number for each key, that are being tracked"));
        m.insert("tracking_total_prefixes", ("tracking_total_prefixes", "Number of tracked prefixes in server's prefix table (only applicable for broadcast mode)"));

        // # Replication
        m.insert(
            "connected_slaves",
            ("connected_slaves", "Number of connected replicas"),
        );
        m.insert(
            "repl_backlog_size",
            (
                "replication_backlog_bytes",
                "Total size in bytes of the replication backlog buffer",
            ),
        );
        m.insert(
            "repl_backlog_active",
            (
                "repl_backlog_is_active",
                "Flag indicating replication backlog is active",
            ),
        );
        m.insert(
            "repl_backlog_first_byte_offset",
            (
                "repl_backlog_first_byte_offset",
                "The master offset of the replication backlog buffer",
            ),
        );
        m.insert(
            "repl_backlog_histlen",
            (
                "repl_backlog_history_bytes",
                "Size in bytes of the data in the replication backlog buffer",
            ),
        );
        m.insert(
            "master_repl_offset",
            (
                "master_repl_offset",
                "The server's current replication offset",
            ),
        );
        m.insert(
            "second_repl_offset",
            (
                "second_repl_offset",
                "The offset up to which replication IDs are accepted",
            ),
        );
        m.insert("slave_expires_tracked_keys", ("slave_expires_tracked_keys", "The number of keys tracked for expiry purposes (applicable only to writable replicas)"));
        m.insert(
            "slave_priority",
            (
                "slave_priority",
                "The priority of the instance as a candidate for failover",
            ),
        );
        m.insert(
            "sync_full",
            (
                "replica_resyncs_full",
                "The number of full resyncs with replicas",
            ),
        );
        m.insert(
            "sync_partial_ok",
            (
                "replica_partial_resync_accepted",
                "The number of accepted partial resync requests",
            ),
        );
        m.insert(
            "sync_partial_err",
            (
                "replica_partial_resync_denied",
                "The number of denied partial resync requests",
            ),
        );

        // # Cluster elasticache
        m.insert(
            "cluster_enabled",
            ("redis_cluster_enabled", "Indicate Redis cluster is enabled"),
        );
        m.insert(
            "cluster_stats_messages_sent",
            ("cluster_messages_sent_total", ""),
        );
        m.insert(
            "cluster_stats_messages_received",
            ("cluster_messages_received_total", ""),
        );

        // # Tile38
        // based on https://tile38.com/commands/server/
        m.insert("tile38_aof_size", ("tile38_aof_size_bytes", ""));
        m.insert("tile38_avg_point_size", ("tile38_avg_item_size_bytes", ""));
        m.insert("tile38_sys_cpus", ("tile38_cpus_total", ""));
        m.insert(
            "tile38_heap_released_bytes",
            ("tile38_heap_released_bytes", ""),
        );
        m.insert("tile38_heap_alloc_bytes", ("tile38_heap_size_bytes", ""));
        m.insert("tile38_http_transport", ("tile38_http_transport", ""));
        m.insert("tile38_in_memory_size", ("tile38_in_memory_size_bytes", ""));
        m.insert("tile38_max_heap_size", ("tile38_max_heap_size_bytes", ""));
        m.insert("tile38_alloc_bytes", ("tile38_mem_alloc_bytes", ""));
        m.insert(
            "tile38_num_collections",
            ("tile38_num_collections_total", ""),
        );
        m.insert("tile38_num_hooks", ("tile38_num_hooks_total", ""));
        m.insert("tile38_num_objects", ("tile38_num_objects_total", ""));
        m.insert("tile38_num_points", ("tile38_num_points_total", ""));
        m.insert("tile38_pointer_size", ("tile38_pointer_size_bytes", ""));
        m.insert("tile38_read_only", ("tile38_read_only", ""));
        m.insert("tile38_go_threads", ("tile38_threads_total", ""));
        m.insert("tile38_go_goroutines", ("tile38_go_goroutines_total", ""));
        m.insert(
            "tile38_last_gc_time_seconds",
            ("tile38_last_gc_time_seconds", ""),
        );
        m.insert("tile38_next_gc_bytes", ("tile38_next_gc_bytes", ""));

        // addtl. KeyDB metrics
        m.insert("server_threads", ("server_threads_total", ""));
        m.insert("long_lock_waits", ("long_lock_waits_total", ""));
        m.insert("current_client_thread", ("current_client_thread", ""));

        // Redis Modules metrics, RediSearch module
        m.insert(
            "search_number_of_indexes",
            (
                "search_number_of_indexes",
                "The total number of indexes in the shard",
            ),
        );
        m.insert(
            "search_used_memory_indexes",
            (
                "search_used_memory_indexes_bytes",
                "The total memory allocated by all indexes in the shard in bytes",
            ),
        );
        // Deprecated in Redis 8.0
        m.insert("search_global_idle", ("search_global_idle", "The total number of user and internal cursors currently holding pending results in the shard"));
        // Deprecated in Redis 8.0
        m.insert("search_global_total", ("search_global_total", "The total number of user and internal cursors in the shard, either holding pending results or actively executing FT.CURSOR READ"));
        m.insert("search_bytes_collected", ("search_collected_bytes", "The total amount of memory freed by the garbage collectors from indexes in the shard memory in bytes"));
        m.insert("search_dialect_1", ("search_dialect_1", ""));
        m.insert("search_dialect_2", ("search_dialect_2", ""));
        m.insert("search_dialect_3", ("search_dialect_3", ""));
        m.insert("search_dialect_4", ("search_dialect_4", ""));

        // RediSearch module v8.0
        m.insert(
            "search_number_of_active_indexes",
            ("search_number_of_active_indexes", "The total number of indexes running a background indexing and/or background query processing operation")
        );
        m.insert(
            "search_number_of_active_indexes_running_queries",
            (
                "search_number_of_active_indexes_running_queries",
                "The total count of indexes currently running a background query process",
            ),
        );
        m.insert(
            "search_number_of_active_indexes_indexing",
            (
                "search_number_of_active_indexes_indexing",
                "The total count of indexes currently undergoing a background indexing process",
            ),
        );
        m.insert(
            "search_total_active_write_threads",
            ("search_total_active_write_threads", "The total count of background write (indexing) processes currently running in the shard"),
        );
        m.insert(
            "search_smallest_memory_index",
            ("search_smallest_memory_index_bytes", "The memory usage of the index with the smallest memory usage in the shard in bytes")
        );
        m.insert(
            "search_largest_memory_index",
            (
                "search_largest_memory_index_bytes",
                "The memory usage of the index with the largest memory usage in the shard in bytes",
            ),
        );
        m.insert(
            "search_used_memory_vector_index",
            (
                "search_used_memory_vector_index_bytes",
                "The total memory usage of all vector indexes in the shard",
            ),
        );
        m.insert("search_global_idle_user", ("search_global_idle_user", "")); // search_gc metrics were split into user and internal
        m.insert(
            "search_global_idle_internal",
            ("search_global_idle_internal", ""),
        ); // in PR, https://github.com/RediSearch/RediSearch/pull/5616
        m.insert("search_global_total_user", ("search_global_total_user", ""));
        m.insert(
            "search_global_total_internal",
            ("search_global_total_internal", ""),
        );
        // search_bytes_collected was renamed in https://github.com/RediSearch/RediSearch/pull/5616
        m.insert("search_gc_bytes_collected", ("search_gc_collected_bytes", "The total amount of memory freed by the garbage collectors from indexes in the shard's memory in bytes"));
        // Added in Redis 8.0
        m.insert(
            "search_gc_total_docs_not_collected",
            ("search_gc_total_docs_not_collected", "The number of documents marked as deleted, whose memory has not yet been freed by the garbage collector")
        );
        m.insert(
            "search_gc_marked_deleted_vectors",
            ("search_gc_marked_deleted_vectors", "The number of vectors marked as deleted in the vector indexes that have not yet been cleaned")
        );
        m.insert(
            "search_errors_indexing_failures",
            (
                "search_errors_indexing_failures",
                "The total number of indexing failures recorded across all indexes in the shard",
            ),
        );

        m
    });

static COUNTER_METRICS: LazyLock<BTreeMap<&'static str, (&'static str, &'static str)>> =
    LazyLock::new(|| {
        let mut m = BTreeMap::new();

        m.insert(
            "total_connections_received",
            (
                "connections_received_total",
                "Total number of connections accepted by the server",
            ),
        );
        m.insert(
            "total_commands_processed",
            (
                "commands_processed_total",
                "Total number of commands processed by the server",
            ),
        );

        m.insert(
            "rejected_connections",
            (
                "rejected_connections_total",
                "Number of connections rejected because of maxclients limit",
            ),
        );
        m.insert(
            "total_net_input_bytes",
            (
                "net_input_bytes_total",
                "The total number of bytes read from the network",
            ),
        );
        m.insert(
            "total_net_output_bytes",
            (
                "net_output_bytes_total",
                "The total number of bytes written to the network",
            ),
        );

        m.insert(
            "total_net_repl_input_bytes",
            (
                "net_repl_input_bytes_total",
                "The total number of bytes read from the network for replication purposes",
            ),
        );
        m.insert(
            "total_net_repl_output_bytes",
            (
                "net_repl_output_bytes_total",
                "The total number of bytes written to the network for replication purposes",
            ),
        );

        m.insert(
            "expired_subkeys",
            (
                "expired_subkeys_total",
                "The number of hash field expiration events",
            ),
        );
        m.insert(
            "expired_keys",
            (
                "expired_keys_total",
                "Total number of key expiration events",
            ),
        );
        m.insert(
            "expired_time_cap_reached_count",
            (
                "expired_time_cap_reached_total",
                "The count of times that active expiry cycles have stopped early",
            ),
        );
        m.insert(
            "expire_cycle_cpu_milliseconds",
            (
                "expire_cycle_cpu_time_ms_total",
                "The cumulative amount of time spent on active expiry cycles",
            ),
        );
        m.insert(
            "evicted_keys",
            (
                "evicted_keys_total",
                "Number of evicted keys due to maxmemory limit",
            ),
        );
        // Added in Redis 7.0
        m.insert(
            "evicted_clients",
            (
                "evicted_clients_total",
                "Number of evicted clients due to maxmemory-clients limit",
            ),
        );
        m.insert(
            "evicted_scripts",
            (
                "evicted_scripts_total",
                "Number of evicted EVAL scripts due to LRU policy, see EVAL for more details",
            ),
        ); // Added in Redis 7.4
        m.insert(
            "total_eviction_exceeded_time",
            ("eviction_exceeded_time_ms_total", "Total time used_memory was greater than maxmemory since server startup, in milliseconds")
        );
        m.insert(
            "keyspace_hits",
            (
                "keyspace_hits_total",
                "Number of successful lookup of keys in the main dictionary",
            ),
        );
        m.insert(
            "keyspace_misses",
            (
                "keyspace_misses_total",
                "Number of failed lookup of keys in the main dictionary",
            ),
        );

        m.insert("used_cpu_sys", ("cpu_sys_seconds_total", "System CPU consumed by the Redis server, which is the sum of system CPU consumed by all threads of the server process (main thread and background threads)"));
        m.insert("used_cpu_user", ("cpu_user_seconds_total", "User CPU consumed by the Redis server, which is the sum of user CPU consumed by all threads of the server process (main thread and background threads)"));
        m.insert(
            "used_cpu_sys_children",
            (
                "cpu_sys_children_seconds_total",
                "System CPU consumed by the background processes",
            ),
        );
        m.insert(
            "used_cpu_user_children",
            (
                "cpu_user_children_seconds_total",
                "User CPU consumed by the background processes",
            ),
        );
        m.insert(
            "used_cpu_sys_main_thread",
            (
                "cpu_sys_main_thread_seconds_total",
                "System CPU consumed by the Redis server main thread",
            ),
        );
        m.insert(
            "used_cpu_user_main_thread",
            (
                "cpu_user_main_thread_seconds_total",
                "User CPU consumed by the Redis server main thread",
            ),
        );

        m.insert("unexpected_error_replies", ("unexpected_error_replies", "Number of unexpected error replies, that are types of errors from an AOF load or replication"));
        m.insert("total_error_replies", ("total_error_replies", "Total number of issued error replies, that is the sum of rejected commands (errors prior command execution) and failed commands (errors within the command execution)"));
        m.insert("dump_payload_sanitizations", ("dump_payload_sanitizations", "Total number of dump payload deep integrity validations (see sanitize-dump-payload config)"));
        m.insert(
            "total_reads_processed",
            (
                "total_reads_processed",
                "Total number of read events processed",
            ),
        );
        m.insert(
            "total_writes_processed",
            (
                "total_writes_processed",
                "Total number of write events processed",
            ),
        );
        m.insert(
            "io_threaded_reads_processed",
            (
                "io_threaded_reads_processed",
                "Number of read events processed by I/O threads",
            ),
        );
        m.insert(
            "io_threaded_writes_processed",
            (
                "io_threaded_writes_processed",
                "Number of write events processed by I/O threads",
            ),
        );
        m.insert(
            "client_query_buffer_limit_disconnections",
            (
                "client_query_buffer_limit_disconnections_total",
                "Total number of disconnections due to client reaching query buffer limit",
            ),
        );
        m.insert(
            "client_output_buffer_limit_disconnections",
            (
                "client_output_buffer_limit_disconnections_total",
                "Total number of disconnections due to client reaching output buffer limit",
            ),
        );
        m.insert(
            "reply_buffer_shrinks",
            (
                "reply_buffer_shrinks_total",
                "Total number of output buffer shrinks",
            ),
        );
        m.insert(
            "reply_buffer_expands",
            (
                "reply_buffer_expands_total",
                "Total number of output buffer expands",
            ),
        );
        m.insert(
            "acl_access_denied_auth",
            (
                "acl_access_denied_auth_total",
                "Number of authentication failures",
            ),
        );
        m.insert(
            "acl_access_denied_cmd",
            (
                "acl_access_denied_cmd_total",
                "Number of commands rejected because of access denied to the command",
            ),
        );
        m.insert(
            "acl_access_denied_key",
            (
                "acl_access_denied_key_total",
                "Number of commands rejected because of access denied to a key",
            ),
        );
        m.insert(
            "acl_access_denied_channel",
            (
                "acl_access_denied_channel_total",
                "Number of commands rejected because of access denied to a channel",
            ),
        );

        // addtl. KeyDB metrics
        m.insert("cached_keys", ("cached_keys_total", ""));
        m.insert(
            "storage_provider_read_hits",
            ("storage_provider_read_hits", ""),
        );
        m.insert(
            "storage_provider_read_misses",
            ("storage_provider_read_misses", ""),
        );

        // Redis Modules metrics, RediSearch module
        m.insert(
            "search_total_indexing_time",
            ("search_indexing_time_ms_total", "The total time spent on indexing operations, excluding the background indexing of vectors in the HNSW graph")
        );
        m.insert(
            "search_total_cycles",
            (
                "search_cycles_total",
                "The total number of garbage collection cycles executed",
            ),
        );
        m.insert("search_total_ms_run", ("search_run_ms_total", "The total duration of all garbage collection cycles in the shard, measured in milliseconds"));

        // RediSearch module v8.0
        m.insert(
            "search_gc_total_cycles",
            (
                "search_gc_cycles_total",
                "The total number of garbage collection cycles executed",
            ),
        ); // search_gc metrics were renamed
        m.insert("search_gc_total_ms_run", ("search_gc_run_ms_total", "The total duration of all garbage collection cycles in the shard, measured in milliseconds")); // in PR, https://github.com/RediSearch/RediSearch/pull/5616
        m.insert(
            "search_total_queries_processed",
            ("search_queries_processed_total", "The total number of successful query executions (When using cursors, not counting reading from existing cursors) in the shard")
        );
        m.insert(
            "search_total_query_commands",
            (
                "search_query_commands_total",
                "The total number of successful query command executions",
            ),
        );
        m.insert(
            "search_total_query_execution_time_ms",
            ("search_query_execution_time_ms_total", "The cumulative execution time of all query commands, including FT.SEARCH, FT.AGGREGATE, and FT.CURSOR READ, measured in ms")
        );
        m.insert("search_total_active_queries", ("search_active_queries_total", "The total number of background queries currently being executed in the shard, excluding FT.CURSOR READ"));

        m
    });

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let config: BTreeMap<String, String> = conn.execute(&["config", "get", "*"]).await?;
    // sentinel does not support this field
    let mut databases = config
        .get("databases")
        .map(|s| s.parse::<u64>().unwrap_or_default())
        .unwrap_or(0);

    let resp = conn.execute::<Bytes>(&["info", "all"]).await?;
    let infos = std::str::from_utf8(&resp).unwrap();
    let mut metrics = vec![];

    if infos.contains("cluster_enabled:1") {
        match cluster_info(conn).await {
            Ok(partial) => {
                metrics.extend(partial);

                // in cluster mode Redis only supports one database so no extra DB
                // number padding needed
                databases = 1;
            }
            Err(err) => {
                warn!(
                    message = "Redis CLUSTER INFO failed",
                    ?err,
                    internal_log_rate_limit = true
                );
            }
        }
    } else if databases == 0 {
        // in non-cluster mode, if db_count is zero then "CONFIG" failed to retrieve a
        // valid number of databases and we use the Redis config default which is 16
        databases = 16
    }

    let partial = extract_info_metrics(infos, databases)?;
    metrics.extend(partial);

    if infos.contains("# Sentinel")
        && let Ok(partial) = sentinel::collect(conn).await
    {
        metrics.extend(partial);
    }

    Ok(metrics)
}

fn extract_info_metrics(infos: &str, databases: u64) -> Result<Vec<Metric>, Error> {
    let mut metrics = vec![];
    let mut handled_dbs = BTreeSet::new();
    let mut cmd_counts = BTreeMap::new();
    let mut cmd_sum = BTreeMap::new();
    let mut cmd_latencies = BTreeMap::new();
    let mut instance_infos = Tags::default();
    let mut slave_infos = Tags::default();
    let mut master_host = "";
    let mut master_port = "";
    let mut field_class = "";
    let mut role = "";
    let slave_info_fields = ["master_host", "master_port", "slave_read_only"];
    const INSTANCE_INFO_FIELD: [&str; 10] = [
        "role",
        "redis_version",
        "redis_build_id",
        "redis_mode",
        "os",
        "maxmemory_policy",
        "tcp_port",
        "run_id",
        "process_id",
        "master_replid",
    ];

    for line in infos.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(stripped) = line.strip_prefix("# ") {
            field_class = stripped;
            continue;
        }

        let Some((key, value)) = line.split_once(':') else {
            continue;
        };

        if key == "master_host" {
            master_host = value;
        }

        if key == "master_port" {
            master_port = value;
        }

        if INSTANCE_INFO_FIELD.contains(&key) {
            instance_infos.insert(key, value);
            continue;
        }

        if slave_info_fields.contains(&key) {
            slave_infos.insert(key.to_string(), value.to_string());
            continue;
        }

        match field_class {
            "Replication" => {
                if key == "role" {
                    role = value;
                    continue;
                }

                if let Ok(partial) =
                    handle_replication_metrics(master_host, master_port, key, value)
                    && !partial.is_empty()
                {
                    metrics.extend(partial);
                    continue;
                }
            }

            "Server" => {
                if let Ok(partial) = handle_server_metrics(key, value) {
                    metrics.extend(partial);
                }
            }

            "Commandstats" => {
                if let Ok((cmd, calls, rejected, failed, usec, extended)) =
                    parse_command_stats(key, value)
                {
                    cmd_counts.insert(cmd, calls);
                    cmd_sum.insert(cmd, usec);

                    metrics.extend([
                        Metric::sum_with_tags(
                            "redis_commands_total",
                            "Total number of calls per command",
                            calls,
                            tags!(
                                "cmd" => cmd,
                            ),
                        ),
                        Metric::sum_with_tags(
                            "redis_commands_duration_seconds_total",
                            "Total amount of time in seconds spent per command",
                            usec / 1_000_000.0,
                            tags!(
                                "cmd" => cmd,
                            ),
                        ),
                    ]);

                    if extended {
                        metrics.extend([
                            Metric::sum_with_tags(
                                "redis_commands_rejected_calls_total",
                                "Total number of errors within command execution per command",
                                rejected,
                                tags!(
                                    "cmd" => cmd,
                                ),
                            ),
                            Metric::sum_with_tags(
                                "redis_commands_failed_calls_total",
                                "Total number of errors prior command execution per command",
                                failed,
                                tags!(
                                    "cmd" => cmd,
                                ),
                            ),
                        ]);
                    }
                }

                continue;
            }

            "Latencystats" => {
                let (cmd, quantiles) = parse_latency_stats(key, value)?;
                cmd_latencies.insert(cmd, quantiles);

                continue;
            }

            "Errorstats" => {
                if let Ok((err, count)) = parse_error_stats(key, value) {
                    metrics.push(Metric::sum_with_tags(
                        "redis_errors_total",
                        "Total number of errors per error type",
                        count,
                        tags!(
                            "err" => err,
                        ),
                    ))
                }

                continue;
            }

            "Keyspace" => {
                if let Ok((keys, expired, avg_ttl, cached)) = parse_keyspace(key, value) {
                    metrics.extend([
                        Metric::gauge_with_tags(
                            "redis_db_keys",
                            "Total number of keys by DB",
                            keys,
                            tags!(
                                "db" => key
                            ),
                        ),
                        Metric::gauge_with_tags(
                            "redis_db_keys_expiring",
                            "Total number of expiring keys by DB",
                            expired,
                            tags!(
                                "db" => key
                            ),
                        ),
                    ]);

                    if cached > -1.0 {
                        metrics.push(Metric::gauge_with_tags(
                            "redis_db_keys_cached",
                            "Total number of cached keys by DB",
                            cached,
                            tags!(
                                "db" => key
                            ),
                        ));
                    }

                    if avg_ttl > -1.0 {
                        metrics.push(Metric::gauge_with_tags(
                            "redis_db_avg_ttl_seconds",
                            "Avg TTL in seconds",
                            avg_ttl,
                            tags!(
                                "db" => key
                            ),
                        ));
                    }

                    handled_dbs.insert(key.to_string());
                    continue;
                }
            }

            "Sentinel" => {
                if let Ok(partial) = handle_sentinel(key, value) {
                    metrics.extend(partial);
                }
            }

            _ => {}
        }

        if let Some(metric) = parse_generic(key, value) {
            metrics.push(metric);
        }
    }

    // build calls
    for (cmd, quantiles) in cmd_latencies {
        let Some(count) = cmd_counts.get(cmd) else {
            continue;
        };
        let Some(sum) = cmd_sum.get(cmd) else {
            continue;
        };

        metrics.push(Metric::summary_with_tags(
            "redis_latency_percentiles_usec",
            "A summary of latency percentile distribution per command",
            *count,
            *sum,
            quantiles,
            tags!(
                "cmd" => cmd,
            ),
        ));
    }

    for i in 0..databases {
        let name = format!("db{i}");

        if !handled_dbs.contains(name.as_str()) {
            metrics.extend([
                Metric::gauge_with_tags(
                    "redis_db_keys",
                    "Total number of keys by DB",
                    0,
                    tags!(
                        "db" => name.clone()
                    ),
                ),
                Metric::gauge_with_tags(
                    "redis_db_keys_expiring",
                    "Total number of expiring keys by DB",
                    0,
                    tags!(
                        "db" => name
                    ),
                ),
            ])
        }
    }

    metrics.push(Metric::gauge_with_tags(
        "redis_instance_info",
        "Information about the Redis instance",
        1,
        instance_infos,
    ));

    if role == "slave" {
        metrics.push(Metric::gauge_with_tags(
            "redis_slave_info",
            "Information about the Redis slave",
            1,
            slave_infos,
        ))
    }

    Ok(metrics)
}

async fn cluster_info(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let infos = conn.execute::<Bytes>(&["cluster", "info"]).await?;
    let infos = unsafe { std::str::from_utf8_unchecked(&infos) };

    let mut metrics = Vec::with_capacity(10);
    for line in infos.lines() {
        let Some((key, value)) = line.split_once(":") else {
            continue;
        };

        if let Some(metric) = parse_generic(key, value) {
            metrics.push(metric);
        }
    }

    Ok(metrics)
}

fn handle_server_metrics(key: &str, value: &str) -> Result<Vec<Metric>, Error> {
    match key {
        "uptime_in_seconds" => {
            let uptime = value.parse::<f64>()?;
            let now = std::time::SystemTime::now();
            let elapsed = now
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64();

            Ok(vec![Metric::gauge(
                "redis_start_time_seconds",
                "Start time of the Redis instance since unix epoch in seconds.",
                elapsed - uptime,
            )])
        }
        "configured_hz" => {
            let value = value.parse::<i64>()?;
            Ok(vec![Metric::gauge(
                "redis_configured_hz",
                "The server's configured frequency setting",
                value,
            )])
        }
        "hz" => {
            let value = value.parse::<i64>()?;
            Ok(vec![Metric::gauge(
                "redis_hz",
                "The server's current frequency setting",
                value,
            )])
        }
        _ => Ok(vec![]),
    }
}

/// ```text
/// Format:
///   errorstat_ERR:count=4
///   errorstat_NOAUTH:count=3
///
/// broken up like this:
///   Key   = errorstat_ERR
///   Value = count=3
/// ```
fn parse_error_stats<'a>(key: &'a str, value: &str) -> Result<(&'a str, u64), Error> {
    let Some(cmd) = key.strip_prefix("errorstat") else {
        return Err(Error::InvalidStatsLine("Errorstats"));
    };

    let Some((_key, value)) = value.split_once('=') else {
        return Err(Error::InvalidStatsLine("Errorstats"));
    };

    let value = value.parse::<u64>()?;

    Ok((cmd, value))
}

/// ```text
/// There are 2 formats. (One before Redis 6.2 and one after it)
/// Format before v6.2:
///     cmdstat_get:calls=21,usec=175,usec_per_call=8.33
///     cmdstat_set:calls=61,usec=3139,usec_per_call=51.46
///     cmdstat_setex:calls=75,usec=1260,usec_per_call=16.80
///     cmdstat_georadius_ro:calls=75,usec=1260,usec_per_call=16.80
/// Format from v6.2 forward:
///     cmdstat_get:calls=21,usec=175,usec_per_call=8.33,rejected_calls=0,failed_calls=0
///     cmdstat_set:calls=61,usec=3139,usec_per_call=51.46,rejected_calls=0,failed_calls=0
///     cmdstat_setex:calls=75,usec=1260,usec_per_call=16.80,rejected_calls=0,failed_calls=0
///     cmdstat_georadius_ro:calls=75,usec=1260,usec_per_call=16.80,rejected_calls=0,failed_calls=0
///
/// broken up like this:
///     key   = cmdstat_get
///     value = calls=21,usec=175,usec_per_call=8.33
/// ```
fn parse_command_stats<'a>(
    key: &'a str,
    value: &'a str,
) -> Result<(&'a str, u64, f64, f64, f64, bool), Error> {
    let mut calls = 0;
    let mut rejected = 0.0;
    let mut failed = 0.0;
    let mut usec = 0.0;
    let mut extended = false;

    let Some(cmd) = key.strip_prefix("cmdstat_") else {
        return Err(Error::InvalidStatsLine("Commandstats"));
    };

    for kv in value.split(',') {
        let Some((k, v)) = kv.split_once('=') else {
            continue;
        };

        match k {
            "calls" => calls = v.parse::<u64>()?,
            "usec" => usec = v.parse::<f64>()?,
            "rejected_calls" => {
                extended = true;
                rejected = v.parse::<f64>()?
            }
            "failed_calls" => {
                extended = true;
                failed = v.parse::<f64>()?
            }
            _ => {}
        }
    }

    Ok((cmd, calls, rejected, failed, usec, extended))
}

fn handle_replication_metrics(
    host: &str,
    port: &str,
    key: &str,
    value: &str,
) -> Result<Vec<Metric>, Error> {
    // only slaves have this field
    if key == "master_link_status" {
        return Ok(vec![Metric::gauge_with_tags(
            "redis_master_link_up",
            "Master link status on Redis slave",
            value == "up",
            tags!(
                "master_host" => host.to_string(),
                "master_port" => port.to_string()
            ),
        )]);
    }

    match key {
        "master_last_io_seconds_ago" | "slave_repl_offset" | "master_sync_in_progress" => {
            let value = value.parse::<i32>()?;
            let desc = if key == "master_last_io_seconds_ago" {
                "Master last io seconds ago"
            } else if key == "slave_repl_offset" {
                "Slave replication offset"
            } else if key == "master_sync_in_progress" {
                "Master sync in progress"
            } else {
                ""
            };

            return Ok(vec![Metric::gauge_with_tags(
                key.to_string(),
                desc,
                value,
                tags!(
                    "master_host" => host.to_string(),
                    "master_port" => port.to_string()
                ),
            )]);
        }

        _ => {}
    }

    // not a slave, try extracting master metrics
    if let Ok((offset, ip, port, state, lag)) = parse_connected_slave_string(key, value) {
        let mut events = vec![];
        events.push(Metric::gauge_with_tags(
            "redis_connected_slave_offset_bytes",
            "Offset of connected slave",
            offset,
            tags!(
                "slave_ip" => ip.to_string(),
                "slave_port" => port.to_string(),
                "slave_state" => state.to_string()
            ),
        ));

        if lag > -1.0 {
            events.push(Metric::gauge_with_tags(
                "redis_connected_slave_lag_seconds",
                "Lag of connected slave",
                lag,
                tags!(
                    "slave_ip" => ip.to_string(),
                    "slave_port" => port.to_string(),
                    "slave_state" => state.to_string()
                ),
            ))
        }

        return Ok(events);
    }

    Ok(vec![])
}

fn parse_generic(key: &str, value: &str) -> Option<Metric> {
    if let Some((name, desc)) = GAUGE_METRICS.get(key) {
        let val = match value {
            "ok" | "true" => 1.0,
            "err" | "fail" | "false" => 0.0,
            _ => value.parse().unwrap_or(0.0),
        };

        let metric = if key == "latest_fork_usec" {
            Metric::gauge("latest_fork_seconds", *desc, val / 1e6)
        } else {
            Metric::gauge(*name, *desc, val)
        };

        return Some(metric);
    }

    if let Some((name, desc)) = COUNTER_METRICS.get(key) {
        let val = match value {
            "ok" | "true" => 1.0,
            "err" | "fail" | "false" => 0.0,
            _ => value.parse().unwrap_or(0.0),
        };

        return Some(Metric::sum(*name, *desc, val));
    }

    None
}

// valid example: db0:keys=1,expires=0,avg_ttl=0,cached_keys=0
fn parse_keyspace(key: &str, value: &str) -> Result<(f64, f64, f64, f64), Error> {
    if !key.starts_with("db") {
        return Err(Error::InvalidStatsLine("Keyspace"));
    }

    let mut keys = 0.0;
    let mut expires = 0.0;
    let mut avg_ttl = -1.0;
    let mut cached_keys = 0.0;

    for pair in value.split(',') {
        let Some((k, v)) = pair.split_once('=') else {
            continue;
        };

        match k {
            "keys" => keys = v.parse()?,
            "expires" => expires = v.parse()?,
            "avg_ttl" => avg_ttl = v.parse::<f64>()? / 1000.0,
            "cached_keys" => cached_keys = v.parse()?,
            _ => {}
        }
    }

    Ok((keys, expires, avg_ttl, cached_keys))
}

/// the slave line looks like
///
/// ```text
/// slave0:ip=10.254.11.1,port=6379,state=online,offset=1751844676,lag=0
/// slave1:ip=10.254.11.2,port=6379,state=online,offset=1751844222,lag=0
/// ```
fn parse_connected_slave_string<'a>(
    key: &'a str,
    value: &'a str,
) -> Result<(f64, &'a str, &'a str, &'a str, f64), Error> {
    if !validate_slave_line(key) {
        return Err(Error::InvalidStatsLine("ConnectedSlave"));
    }

    let mut ip = "";
    let mut port = "";
    let mut state = "";
    let mut offset = 0.0;
    let mut lag = 0.0;

    for kv in value.split(',') {
        let Some((k, v)) = kv.split_once('=') else {
            continue;
        };

        match k {
            "ip" => ip = v,
            "port" => port = v,
            "state" => state = v,
            "offset" => offset = v.parse::<f64>()?,
            "lag" => lag = v.parse::<f64>()?,
            _ => {}
        }
    }

    Ok((offset, ip, port, state, lag))
}

// Example
// 	 master0:name=user03,status=sdown,address=192.169.2.52:6381,slaves=1,sentinels=5
// 	 master1:name=user02,status=ok,address=192.169.2.54:6380,slaves=1,sentinels=5
fn handle_sentinel(key: &str, value: &str) -> Result<Vec<Metric>, Error> {
    if let Some(desc) = match key {
        "sentinel_masters" => Some("The number of masters this sentinel is watching"),
        "sentinel_tilt" => Some("Sentinel is in TILT mode"),
        "sentinel_running_scripts" => Some("Number of scripts in execution right now"),
        "sentinel_scripts_queue_length" => Some("Queue of user scripts to execute"),
        "sentinel_simulate_failure_flags" => Some("Failures simulations"),
        _ => None,
    } {
        let value = value.parse::<f64>()?;

        return Ok(vec![Metric::gauge(format!("redis_{key}"), desc, value)]);
    }

    let mut metrics = Vec::with_capacity(3);
    if let Ok((name, status, address, slaves, sentinels)) = parse_sentinel_master(key, value) {
        let status = status == "ok";

        metrics.extend([
            Metric::gauge_with_tags(
                "redis_sentinel_master_status",
                "Master status on Sentinel",
                status,
                tags!(
                    "master" => name,
                    "master_address" => address,
                    "status" => status,
                ),
            ),
            Metric::gauge_with_tags(
                "redis_sentinel_master_slaves",
                "The number of slaves of the master",
                slaves,
                tags!(
                    "master" => name,
                    "master_address" => address,
                ),
            ),
            Metric::gauge_with_tags(
                "redis_sentinel_master_sentinels",
                "The number of sentinels monitoring this master",
                sentinels,
                tags!(
                    "master" => name,
                    "master_address" => address,
                ),
            ),
        ]);
    }

    Ok(metrics)
}

fn parse_sentinel_master<'a>(
    master: &str,
    info: &'a str,
) -> Result<(&'a str, &'a str, &'a str, f64, f64), Error> {
    if !master.starts_with("master") {
        return Err(Error::InvalidStatsLine("Sentinel"));
    }

    let mut name = "";
    let mut status = "";
    let mut address = "";
    let mut slaves = 0.0;
    let mut sentinels = 0.0;
    for kv in info.split(',') {
        let Some((key, value)) = kv.split_once('=') else {
            continue;
        };

        match key {
            "name" => name = value,
            "status" => status = value,
            "address" => address = value,
            "slaves" => slaves = value.parse()?,
            "sentinels" => sentinels = value.parse()?,
            _ => {}
        }
    }

    Ok((name, status, address, slaves, sentinels))
}

/// # Latencystats
/// latency_percentiles_usec_rpop:p50=0.001,p99=1.003,p99.9=4.015
/// latency_percentiles_usec_zadd:p50=0.001,p99=1.003,p99.9=4.015
/// latency_percentiles_usec_hset:p50=0.001,p99=1.003,p99.9=3.007
/// latency_percentiles_usec_set:p50=0.001,p99=1.003,p99.9=4.015
/// latency_percentiles_usec_lpop:p50=0.001,p99=1.003,p99.9=4.015
/// latency_percentiles_usec_lpush:p50=0.001,p99=1.003,p99.9=4.015
/// latency_percentiles_usec_lrange:p50=17.023,p99=21.119,p99.9=27.007
/// latency_percentiles_usec_get:p50=0.001,p99=1.003,p99.9=3.007
/// latency_percentiles_usec_mset:p50=1.003,p99=1.003,p99.9=1.003
/// latency_percentiles_usec_spop:p50=0.001,p99=1.003,p99.9=1.003
/// latency_percentiles_usec_incr:p50=0.001,p99=1.003,p99.9=3.007
/// latency_percentiles_usec_rpush:p50=0.001,p99=1.003,p99.9=4.015
/// latency_percentiles_usec_zpopmin:p50=0.001,p99=1.003,p99.9=3.007
/// latency_percentiles_usec_config|resetstat:p50=280.575,p99=280.575,p99.9=280.575
/// latency_percentiles_usec_config|get:p50=8.031,p99=27.007,p99.9=27.007
/// latency_percentiles_usec_ping:p50=0.001,p99=1.003,p99.9=1.003
/// latency_percentiles_usec_sadd:p50=0.001,p99=1.003,p99.9=3.007
///
/// broken up like this:
///   fieldKey  = latency_percentiles_usec_ping
///   fieldValue= p50=0.001,p99=1.003,p99.9=3.007
fn parse_latency_stats<'a>(
    key: &'a str,
    value: &'a str,
) -> Result<(&'a str, Vec<Quantile>), Error> {
    const COMMAND_PREFIX: &str = "latency_percentiles_usec_";

    match key.strip_prefix(COMMAND_PREFIX) {
        Some(cmd) => {
            let mut quantiles = Vec::with_capacity(3);

            for pair in value.split(',') {
                let Some((key, value)) = pair.split_once('=') else {
                    continue;
                };

                if let Some(s) = key.strip_prefix('p') {
                    let quantile = s.parse::<f64>()?;
                    let value = value.parse::<f64>()?;

                    quantiles.push(Quantile { quantile, value });
                }
            }

            Ok((cmd, quantiles))
        }
        None => Err(Error::InvalidStatsLine("Latencystats")),
    }
}

fn validate_slave_line(slave: &str) -> bool {
    let Some(num) = slave.strip_prefix("slave") else {
        return false;
    };

    num.parse::<u64>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_db_keyspace() {
        let key = "db0";
        let value = "keys=100,expires=50,avg_ttl=5";
        let (keys, expires, avg_ttl, cached) = parse_keyspace(key, value).unwrap();
        assert_eq!(keys, 100.0);
        assert_eq!(expires, 50.0);
        assert_eq!(avg_ttl, 5.0 / 1000.0);
        assert_eq!(cached, 0.0);
    }

    #[test]
    fn parse_db_keyspace_without_avg_ttl() {
        let key = "db1";
        let value = "keys=100,expires=50";
        let (keys, expires, avg_ttl, cached) = parse_keyspace(key, value).unwrap();
        assert_eq!(keys, 100.0);
        assert_eq!(expires, 50.0);
        assert_eq!(avg_ttl, -1.0);
        assert_eq!(cached, 0.0);
    }
}
