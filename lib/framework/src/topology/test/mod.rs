#[cfg(all(test, feature = "sinks-blackhole", feature = "transforms-json_parser"))]
mod transient_state;

#[cfg(all(test, feature = "sources-demo_logs"))]
mod source_finished;

#[cfg(all(
    test,
    feature = "sources-demo_logs",
    feature = "sinks-prometheus_exporter",
    feature = "transforms-metricalize",
    feature = "sinks-socket",
))]
mod reload;

#[cfg(all(test, feature = "sinks-console", feature = "sources-socket"))]
mod doesnt_reload;

// TODO: enable this in the future
// #[cfg(test)]
// mod backpressure;
mod utils;

pub use utils::start_topology;
