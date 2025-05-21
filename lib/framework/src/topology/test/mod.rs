// TODO: enable theos in the future
// #[cfg(all(test, feature = "sinks-blackhole", feature = "transforms-json_parser"))]
// mod transient_state;

// #[cfg(all(test, feature = "sources-generate"))]
// mod source_finished;

// #[cfg(all(
//    test,
//    feature = "sources-generate",
//    feature = "sinks-prometheus_exporter",
//    feature = "transforms-metricalize",
//    feature = "sinks-socket",
// ))]
// mod reload;

// #[cfg(all(test, feature = "sinks-console", feature = "sources-socket"))]
// mod doesnt_reload;

// #[cfg(test)]
// mod backpressure;

mod utils;

pub use utils::start_topology;
