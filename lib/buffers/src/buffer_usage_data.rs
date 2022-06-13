use std::borrow::Cow;
use std::{
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use tokio::time::interval;
use tracing::{Instrument, Span};

use crate::WhenFull;

#[derive(Clone, Debug)]
pub struct BufferUsageHandle {
    state: Arc<BufferUsageData>,
}

impl BufferUsageHandle {
    /// Creates a no-op [`BufferUsageHandle`] handle.
    ///
    /// No usage data is written or stored.
    pub(crate) fn noop(when_full: WhenFull) -> Self {
        BufferUsageHandle {
            state: Arc::new(BufferUsageData::new(when_full, 0)),
        }
    }

    /// Gets a snapshot of the buffer usage data, representing an instantaneous view of the
    /// different values.
    pub fn snapshot(&self) -> BufferUsageSnapshot {
        self.state.snapshot()
    }

    /// Sets the limits for this buffer component.
    ///
    /// Limits are exposed as gauges to provide stable values when superimposed on dashboards/graphs
    /// with the "actual" usage amounts.
    pub fn set_buffer_limits(&self, max_bytes: Option<u64>, max_events: Option<usize>) {
        if let Some(max_bytes) = max_bytes {
            self.state
                .max_size_bytes
                .store(max_bytes, Ordering::Relaxed);
        }

        if let Some(max_events) = max_events {
            self.state
                .max_size_events
                .store(max_events, Ordering::Relaxed);
        }
    }

    /// Increments the number of events (and their total size) received by this buffer component.
    ///
    /// This represents the events being sent into the buffer.
    pub fn increment_received_event_count_and_byte_size(&self, count: u64, byte_size: u64) {
        self.state
            .received_event_count
            .fetch_add(count, Ordering::Relaxed);
        self.state
            .received_byte_size
            .fetch_add(byte_size, Ordering::Relaxed);
    }

    /// Increments the number of events (and their total size) sent by this buffer component.
    ///
    /// This represents the events being read out of the buffer.
    pub fn increment_sent_event_count_and_byte_size(&self, count: u64, byte_size: u64) {
        self.state
            .sent_event_count
            .fetch_add(count, Ordering::Relaxed);
        self.state
            .sent_byte_size
            .fetch_add(byte_size, Ordering::Relaxed);
    }

    /// Attempts to increment the count of dropped events for this buffer component.
    ///
    /// If the component itself is not configured to drop events, this call does nothing.
    pub fn try_increment_dropped_event_count(&self, count: u64) {
        if let Some(dropped_event_count) = &self.state.dropped_event_count {
            dropped_event_count.fetch_add(count, Ordering::Relaxed);
        }
    }
}

#[derive(Debug)]
pub struct BufferUsageData {
    idx: usize,
    received_event_count: AtomicU64,
    received_byte_size: AtomicU64,
    sent_event_count: AtomicU64,
    sent_byte_size: AtomicU64,
    dropped_event_count: Option<AtomicU64>,
    max_size_bytes: AtomicU64,
    max_size_events: AtomicUsize,
}

impl BufferUsageData {
    pub fn new(mode: WhenFull, idx: usize) -> Self {
        let dropped_event_count = match mode {
            WhenFull::Block | WhenFull::Overflow => None,
            WhenFull::DropNewest => Some(AtomicU64::new(0)),
        };

        Self {
            idx,
            received_event_count: AtomicU64::new(0),
            received_byte_size: AtomicU64::new(0),
            sent_event_count: AtomicU64::new(0),
            sent_byte_size: AtomicU64::new(0),
            dropped_event_count,
            max_size_bytes: AtomicU64::new(0),
            max_size_events: AtomicUsize::new(0),
        }
    }

    fn snapshot(&self) -> BufferUsageSnapshot {
        BufferUsageSnapshot {
            received_event_count: self.received_event_count.load(Ordering::Relaxed),
            received_byte_size: self.received_byte_size.load(Ordering::Relaxed),
            sent_event_count: self.sent_event_count.load(Ordering::Relaxed),
            sent_byte_size: self.sent_byte_size.load(Ordering::Relaxed),
            dropped_event_count: self
                .dropped_event_count
                .as_ref()
                .map(|inner| inner.load(Ordering::Relaxed)),
            max_size_bytes: self.max_size_bytes.load(Ordering::Relaxed),
            max_size_events: self.max_size_events.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug)]
pub struct BufferUsageSnapshot {
    pub received_event_count: u64,
    pub received_byte_size: u64,
    pub sent_event_count: u64,
    pub sent_byte_size: u64,
    pub dropped_event_count: Option<u64>,
    pub max_size_bytes: u64,
    pub max_size_events: usize,
}

pub struct BufferUsage {
    span: Span,
    stages: Vec<Arc<BufferUsageData>>,
}

impl BufferUsage {
    /// Creates an instance of [`BufferUsage`] attached to the given span.
    ///
    /// As buffers can have multiple stages, callers have the ability to register each stage via [`add_stage`]
    pub fn from_span(span: Span) -> BufferUsage {
        Self {
            span,
            stages: Vec::new(),
        }
    }

    /// Adds a new stage to track usage for.
    ///
    /// A [`BufferUsageHandle`] is returned that the caller can use to actually update the usage
    /// metrics with.  This handle will only update the usage metrics for the particular stage it
    /// was added for.
    pub fn add_stage(&mut self, idx: usize, mode: WhenFull) -> BufferUsageHandle {
        let data = Arc::new(BufferUsageData::new(mode, idx));
        let handle = BufferUsageHandle {
            state: Arc::clone(&data),
        };

        self.stages.push(data);
        handle
    }

    pub fn install(self) {
        let span = self.span;
        let stages = self.stages;

        tokio::spawn(
            async move {
                let mut interval = interval(Duration::from_secs(2));
                let max_event_size = metrics::register_gauge("buffer_max_event_size", "");
                let max_byte_size = metrics::register_gauge("buffer_max_byte_size", "");
                let received_events = metrics::register_counter(
                    "buffer_received_events_total",
                    "The number of events received by this buffer.",
                );
                let received_bytes = metrics::register_counter(
                    "buffer_received_event_bytes_total",
                    "The number of bytes received by this buffer.",
                );
                let sent_events = metrics::register_counter(
                    "buffer_sent_events_total",
                    "The number of events sent by this buffer.",
                );
                let sent_bytes = metrics::register_counter(
                    "buffer_sent_event_bytes_total",
                    "The number of bytes sent by this buffer.",
                );
                let dropped_events = metrics::register_counter(
                    "buffer_discarded_events_total",
                    "The number of events dropped by this non-blocking buffer.",
                );

                loop {
                    interval.tick().await;

                    for stage in &stages {
                        let index = Cow::from(stage.idx.to_string());
                        let attrs = metrics::Attributes::from([("stage", index)]);

                        match stage.max_size_bytes.load(Ordering::Relaxed) {
                            0 => {}
                            value => max_byte_size.recorder(attrs.clone()).set(value as u64),
                        };
                        match stage.max_size_events.load(Ordering::Relaxed) {
                            0 => {}
                            value => max_event_size.recorder(attrs.clone()).set(value as u64),
                        };

                        if let Some(dropped_event_count) = &stage.dropped_event_count {
                            dropped_events
                                .recorder(attrs.clone())
                                .inc(dropped_event_count.swap(0, Ordering::Relaxed));
                        }

                        received_events
                            .recorder(attrs.clone())
                            .inc(stage.received_event_count.swap(0, Ordering::Relaxed));
                        received_bytes
                            .recorder(attrs.clone())
                            .inc(stage.received_event_count.swap(0, Ordering::Relaxed));
                        sent_events
                            .recorder(attrs.clone())
                            .inc(stage.sent_event_count.swap(0, Ordering::Relaxed));
                        sent_bytes
                            .recorder(attrs)
                            .inc(stage.sent_byte_size.swap(0, Ordering::Relaxed));
                    }
                }
            }
            .instrument(span.or_current()),
        );
    }
}
