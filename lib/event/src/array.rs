use std::{iter, slice, vec};

use bytesize::ByteSizeOf;
use finalize::{AddBatchNotifier, BatchNotifier, EventFinalizer, EventFinalizers, Finalizable};

use crate::{Event, EventMetadata, EventRef, LogRecord, Metric, Trace};

/// An array of one of the `Event` variants exclusively
#[derive(Clone, Debug, PartialEq)]
pub enum Events {
    /// An array of type `LogRecord`
    Logs(Vec<LogRecord>),
    /// An array of type `Metric`
    Metrics(Vec<Metric>),
    /// An array of type `Trace`
    Traces(Vec<Trace>),
}

impl From<Event> for Events {
    fn from(event: Event) -> Self {
        match event {
            Event::Log(log) => Self::Logs(vec![log]),
            Event::Metric(metric) => Self::Metrics(vec![metric]),
            Event::Trace(trace) => Self::Traces(vec![trace]),
        }
    }
}

impl From<LogRecord> for Events {
    fn from(log: LogRecord) -> Self {
        Self::Logs(vec![log])
    }
}

impl From<Metric> for Events {
    fn from(metric: Metric) -> Self {
        Self::Metrics(vec![metric])
    }
}

impl From<Trace> for Events {
    fn from(trace: Trace) -> Self {
        Self::Traces(vec![trace])
    }
}

impl From<Vec<LogRecord>> for Events {
    fn from(logs: Vec<LogRecord>) -> Self {
        Self::Logs(logs)
    }
}

impl From<Vec<Metric>> for Events {
    fn from(metrics: Vec<Metric>) -> Self {
        Self::Metrics(metrics)
    }
}

impl AddBatchNotifier for Events {
    fn add_batch_notifier(&mut self, notifier: BatchNotifier) {
        match self {
            Events::Logs(array) => array
                .iter_mut()
                .for_each(|item| item.add_finalizer(EventFinalizer::new(notifier.clone()))),
            Events::Metrics(array) => array
                .iter_mut()
                .for_each(|item| item.add_finalizer(EventFinalizer::new(notifier.clone()))),
            Events::Traces(array) => array
                .iter_mut()
                .for_each(|item| item.add_finalizer(EventFinalizer::new(notifier.clone()))),
        }
    }
}

impl ByteSizeOf for Events {
    fn allocated_bytes(&self) -> usize {
        match self {
            Self::Logs(logs) => logs.allocated_bytes(),
            Self::Metrics(metrics) => metrics.allocated_bytes(),
            Self::Traces(spans) => spans.allocated_bytes(),
        }
    }
}

impl Finalizable for Events {
    fn take_finalizers(&mut self) -> EventFinalizers {
        match self {
            Events::Logs(array) => array.iter_mut().map(Finalizable::take_finalizers).collect(),
            Events::Metrics(array) => array.iter_mut().map(Finalizable::take_finalizers).collect(),
            Events::Traces(array) => array.iter_mut().map(Finalizable::take_finalizers).collect(),
        }
    }
}

/// The core trait to abstract over any type that may work as an
/// array of events. This is effectively the same as the standard
/// `IntoIterator<Item = Event>` implementations, but that would
/// conflict with the base implementation for the type aliases
/// below.
pub trait EventContainer: ByteSizeOf {
    /// The type of `Iterator` used to turn this container into events.
    type IntoIter: Iterator<Item = Event>;

    /// The number of events in this container.
    fn len(&self) -> usize;

    /// Is this container empty?
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Turn this container into an iterator of events.
    fn into_events(self) -> Self::IntoIter;
}

impl EventContainer for Event {
    type IntoIter = iter::Once<Event>;

    fn len(&self) -> usize {
        1
    }

    fn is_empty(&self) -> bool {
        false
    }

    fn into_events(self) -> Self::IntoIter {
        iter::once(self)
    }
}

impl EventContainer for Events {
    type IntoIter = EventsIntoIter;

    fn len(&self) -> usize {
        match self {
            Events::Logs(logs) => logs.len(),
            Events::Metrics(metrics) => metrics.len(),
            Events::Traces(traces) => traces.len(),
        }
    }

    fn into_events(self) -> Self::IntoIter {
        match self {
            Events::Logs(logs) => EventsIntoIter::Logs(logs.into_iter()),
            Events::Metrics(metrics) => EventsIntoIter::Metrics(metrics.into_iter()),
            Events::Traces(traces) => EventsIntoIter::Traces(traces.into_iter()),
        }
    }
}

impl Events {
    pub fn for_each_log(&mut self, update: impl FnMut(&mut LogRecord)) {
        if let Self::Logs(logs) = self {
            logs.iter_mut().for_each(update);
        }
    }

    pub fn for_each_metric(&mut self, update: impl FnMut(&mut Metric)) {
        if let Self::Metrics(metrics) = self {
            metrics.iter_mut().for_each(update);
        }
    }

    pub fn for_each_trace(&mut self, update: impl FnMut(&mut Trace)) {
        if let Self::Traces(traces) = self {
            traces.iter_mut().for_each(update);
        }
    }

    pub fn into_logs(self) -> Option<Vec<LogRecord>> {
        if let Self::Logs(logs) = self {
            Some(logs)
        } else {
            None
        }
    }

    pub fn into_metrics(self) -> Option<Vec<Metric>> {
        if let Self::Metrics(metrics) = self {
            Some(metrics)
        } else {
            None
        }
    }

    pub fn for_each_event(&mut self, mut update: impl FnMut(EventMutRef<'_>)) {
        match self {
            Self::Logs(logs) => logs.iter_mut().for_each(|log| update(log.into())),
            Self::Metrics(metrics) => metrics.iter_mut().for_each(|metric| update(metric.into())),
            Self::Traces(traces) => traces.iter_mut().for_each(|trace| update(trace.into())),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Events::Logs(logs) => logs.len(),
            Events::Metrics(metrics) => metrics.len(),
            Events::Traces(traces) => traces.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Events::Logs(logs) => logs.is_empty(),
            Events::Metrics(metrics) => metrics.is_empty(),
            Events::Traces(traces) => traces.is_empty(),
        }
    }

    pub fn merge(&mut self, other: Self) {
        match (self, other) {
            (Events::Logs(logs), Events::Logs(others)) => logs.extend(others),
            (Events::Metrics(metrics), Events::Metrics(others)) => metrics.extend(others),
            (Events::Traces(traces), Events::Traces(others)) => traces.extend(others),
            _ => {}
        }
    }

    pub fn iter_events(&self) -> impl Iterator<Item = EventRef<'_>> {
        match self {
            Events::Logs(logs) => EventsIter::Logs(logs.iter()),
            Events::Metrics(metrics) => EventsIter::Metrics(metrics.iter()),
            Events::Traces(traces) => EventsIter::Traces(traces.iter()),
        }
    }
}

/// A wrapper for mutable references to inner event types, where reconstituting
/// a full `Event` from a `LogEvent` or `Metric` might be inconvenient.
#[derive(Debug)]
pub enum EventMutRef<'a> {
    /// Reference to a `LogEvent`
    Log(&'a mut LogRecord),
    /// Reference to a `Metric`
    Metric(&'a mut Metric),
    /// Reference to a `TraceEvent`
    Trace(&'a mut Trace),
}

impl<'a> EventMutRef<'a> {
    /// Extract the `LogEvent` reference in this.
    ///
    /// # Panics
    ///
    /// This will panic if this is not a `LogRecord` reference.
    pub fn as_log(self) -> &'a LogRecord {
        match self {
            Self::Log(log) => log,
            _ => panic!("Failed type coercion, {self:?} is not a log reference"),
        }
    }

    /// Convert this reference into a new `LogEvent` by cloning.
    ///
    /// # Panics
    ///
    /// This will panic if this is not a `LogEvent` reference.
    pub fn into_log(self) -> LogRecord {
        match self {
            Self::Log(log) => log.clone(),
            _ => panic!("Failed type coercion, {self:?} is not a log reference"),
        }
    }

    /// Extract the `Metric` reference in this.
    ///
    /// # Panics
    ///
    /// This will panic if this is not a `Metric` reference.
    pub fn as_metric(self) -> &'a Metric {
        match self {
            Self::Metric(metric) => metric,
            _ => panic!("Failed type coercion, {self:?} is not a metric reference"),
        }
    }

    /// Convert this reference into a new `Metric` by cloning.
    ///
    /// # Panics
    ///
    /// This will panic if this is not a `Metric` reference.
    pub fn into_metric(self) -> Metric {
        match self {
            Self::Metric(metric) => metric.clone(),
            _ => panic!("Failed type coercion, {self:?} is not a metric reference"),
        }
    }

    /// Access the metadata in this reference.
    pub fn metadata(&self) -> &EventMetadata {
        match self {
            Self::Log(event) => event.metadata(),
            Self::Metric(event) => event.metadata(),
            Self::Trace(event) => event.metadata(),
        }
    }

    /// Access the metadata mutably in this reference.
    pub fn metadata_mut(&mut self) -> &mut EventMetadata {
        match self {
            Self::Log(event) => event.metadata_mut(),
            Self::Metric(event) => event.metadata_mut(),
            Self::Trace(event) => event.metadata_mut(),
        }
    }
}

impl<'a> From<&'a mut Event> for EventMutRef<'a> {
    fn from(event: &'a mut Event) -> Self {
        match event {
            Event::Log(event) => event.into(),
            Event::Metric(event) => event.into(),
            Event::Trace(event) => event.into(),
        }
    }
}

impl<'a> From<&'a mut LogRecord> for EventMutRef<'a> {
    fn from(log: &'a mut LogRecord) -> Self {
        Self::Log(log)
    }
}

impl<'a> From<&'a mut Metric> for EventMutRef<'a> {
    fn from(metric: &'a mut Metric) -> Self {
        Self::Metric(metric)
    }
}

impl<'a> From<&'a mut Trace> for EventMutRef<'a> {
    fn from(trace: &'a mut Trace) -> Self {
        Self::Trace(trace)
    }
}

/// The iterator type for `EventArray::iter_events`.
#[derive(Debug)]
pub enum EventsIter<'a> {
    /// An iterator over type `LogRecord`.
    Logs(slice::Iter<'a, LogRecord>),
    /// An iterator over type `Metric`.
    Metrics(slice::Iter<'a, Metric>),
    /// An iterator over type `Trace`.
    Traces(slice::Iter<'a, Trace>),
}

impl<'a> Iterator for EventsIter<'a> {
    type Item = EventRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Logs(i) => i.next().map(EventRef::from),
            Self::Metrics(i) => i.next().map(EventRef::from),
            Self::Traces(i) => i.next().map(EventRef::from),
        }
    }
}

/// The iterator type for `EventArray::into_events`.
#[derive(Debug)]
pub enum EventsIntoIter {
    /// An iterator over type `Log`.
    Logs(vec::IntoIter<LogRecord>),
    /// An iterator over type `Metric`.
    Metrics(vec::IntoIter<Metric>),
    /// An iterator over type `TraceEvent`.
    Traces(vec::IntoIter<Trace>),
}

impl Iterator for EventsIntoIter {
    type Item = Event;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Logs(i) => i.next().map(Into::into),
            Self::Metrics(i) => i.next().map(Into::into),
            Self::Traces(i) => i.next().map(Event::Trace),
        }
    }
}

/// Turn a container into a futures stream over the contained `Event`
/// type.  This would ideally be implemented as a default method on
/// `trait EventContainer`, but the required feature (associated type
/// defaults) is still unstable.
/// See <https://github.com/rust-lang/rust/issues/29661>
pub fn into_event_stream(
    container: impl EventContainer,
) -> impl futures::Stream<Item = Event> + Unpin {
    futures::stream::iter(container.into_events())
}
