#![forbid(unsafe_code)]
#![deny(unused)]
#![deny(dead_code)]

mod attributes;
mod counter;
mod gauge;
mod histogram;
mod metric;
mod registry;

pub use attributes::Attributes;
pub use counter::Counter;
pub use gauge::Gauge;
pub use histogram::{exponential_buckets, linear_buckets, Histogram, HistogramObservation};
pub use metric::{Metric, Observation};
pub use registry::{
    global_registry, register_counter, register_gauge, register_histogram, Registry, Reporter,
};
