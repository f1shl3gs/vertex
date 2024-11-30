use event::Metric;
use tikv_jemalloc_ctl::stats;

pub fn alloc_metrics() -> Vec<Metric> {
    let allocated = stats::allocated::read().unwrap();
    let active = stats::active::read().unwrap();
    let metadata = stats::metadata::read().unwrap();
    let resident = stats::resident::read().unwrap();
    let mapped = stats::mapped::read().unwrap();
    let retained = stats::retained::read().unwrap();

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
    ]
}
