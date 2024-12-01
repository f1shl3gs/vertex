use event::{tags, Metric};
use tikv_jemalloc_ctl::stats;

pub fn alloc_metrics() -> Vec<Metric> {
    let allocated = stats::allocated::read().unwrap();
    let active = stats::active::read().unwrap();
    let metadata = stats::metadata::read().unwrap();
    let resident = stats::resident::read().unwrap();
    let mapped = stats::mapped::read().unwrap();
    let retained = stats::retained::read().unwrap();

    let version = tikv_jemalloc_ctl::version::read().unwrap()
        .trim_end_matches('\0');
    let background_thread = tikv_jemalloc_ctl::background_thread::read().unwrap();
    let max_background_threads = tikv_jemalloc_ctl::max_background_threads::read().unwrap();
    let epoch = tikv_jemalloc_ctl::epoch::read().unwrap();

    vec![
        Metric::sum(
            "jemalloc_allocated_bytes",
            "The number of bytes allocated by the application",
            allocated as f64
        ),
        Metric::gauge(
            "jemalloc_active_bytes",
            "Total number of bytes in active pages allocated by the application",
            active as f64
        ),
        Metric::gauge(
            "jemalloc_metadata_bytes",
            "Total number of bytes dedicated to jemalloc metadata",
            metadata as f64
        ),
        Metric::gauge(
            "jemalloc_resident_bytes",
            "Total number of bytes in physically resident data pages mapped by the allocator",
            resident as f64
        ),
        Metric::gauge(
            "jemalloc_mapped_bytes",
            "Total number of bytes in active extents mapped by the allocator",
            mapped as f64
        ),
        Metric::gauge(
            "jemalloc_retained_bytes",
            "Total number of bytes in virtual memory mappings that were retained rather than being returned to the operating system",
            retained as f64
        ),
        Metric::gauge_with_tags(
            "jemalloc_version",
            "Jemalloc version",
            1,
            tags!(
                "version" => version
            )
        ),
        Metric::gauge(
            "jemalloc_background_thread_total",
            "State of internal background worker threads",
            background_thread
        ),
        Metric::gauge(
            "jemalloc_max_background_threads",
            "Maximum number of background threads that will be created",
            max_background_threads
        ),
        Metric::gauge(
            "jemalloc_epoch",
            "Many of the statistics tracked by `jemalloc` are cached. The epoch control when they are refreshed",
            epoch
        )
    ]
}
