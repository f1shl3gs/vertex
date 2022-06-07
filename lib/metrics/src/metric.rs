use std::collections::BTreeMap;
use std::sync::Arc;

use crate::attributes::Attributes;
use crate::histogram::HistogramObservation;
use parking_lot::{MappedMutexGuard, Mutex, MutexGuard};

/// A `Metric` records an `Observation` for each unique set of `Attributes`
#[derive(Debug, Clone)]
pub enum Observation {
    Counter(u64),
    Gauge(u64),
    Histogram(HistogramObservation),
}

/// Types that wish to be used with `Metric` must implement this trait
/// that exposes the necessary reporting API
///
/// `Metric` maintains a distinct `MetricObserver` for each unique set of `Attributes`
pub trait MetricObserver: MakeMetricObserver + std::fmt::Debug + Send + 'static {
    /// The type that is used to modify the value reported by this MetricObserver
    ///
    /// Most commonly this will be `Self` but see `CumulativeGauge` for an example
    /// of where it is not
    type Recorder;

    /// Return a `Self::Recorder` that can be used to mutate the value reported
    /// by this `MetricObserver`
    fn recorder(&self) -> Self::Recorder;

    /// Return the current value for this
    fn observe(&self) -> Observation;
}

/// All `MetricObserver` must also implement `MakeMetricObserver` which defines
/// how to construct new instances of `Self`
///
/// A blanket impl is provided for types that implement Default
///
/// See `U64Histogram` for an example of how this is used
pub trait MakeMetricObserver {
    type Options: Sized + Send + Sync + std::fmt::Debug;

    fn create(options: &Self::Options) -> Self;
}

impl<T: Default> MakeMetricObserver for T {
    type Options = ();

    fn create(_: &Self::Options) -> Self {
        Default::default()
    }
}

#[derive(Clone)]
pub struct Metric<T: MetricObserver> {
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) shard: Arc<Mutex<BTreeMap<Attributes, T>>>,

    pub(crate) options: T::Options,
}

impl<T: MetricObserver> Metric<T> {
    pub fn recorder(&self, attributes: impl Into<Attributes>) -> T::Recorder {
        self.observer(attributes).recorder()
    }

    pub fn observer(&self, attributes: impl Into<Attributes>) -> MappedMutexGuard<'_, T> {
        MutexGuard::map(self.shard.lock(), |values| {
            values
                .entry(attributes.into())
                .or_insert_with(|| T::create(&self.options))
        })
    }
}
