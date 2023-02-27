use std::borrow::Cow;
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use tokio::time::interval;
use tracing::{Instrument, Span};

#[derive(Clone, Debug)]
pub struct BufferUsageHandle {
    state: Arc<BufferUsageData>,
}

impl BufferUsageHandle {
    /// Creates a no-op [`BufferUsageHandle`] handle.
    ///
    /// No usage data is written or stored.
    pub(crate) fn noop() -> Self {
        BufferUsageHandle {
            state: Arc::new(BufferUsageData::new(0)),
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
        let max_events = max_events
            .and_then(|n| u64::try_from(n).ok().or(Some(u64::MAX)))
            .unwrap_or(0);
        let max_bytes = max_bytes.unwrap_or(0);

        self.state.max_size.set(max_events, max_bytes);
    }

    /// Increments the number of events (and their total size) received by this buffer component.
    ///
    /// This represents the events being sent into the buffer.
    pub fn increment_received_event_count_and_byte_size(&self, count: u64, byte_size: u64) {
        self.state.received.increase(count, byte_size);
    }

    /// Increments the number of events (and their total size) sent by this buffer component.
    ///
    /// This represents the events being read out of the buffer.
    pub fn increment_sent_event_count_and_byte_size(&self, count: u64, byte_size: u64) {
        self.state.sent.increase(count, byte_size);
    }

    /// Attempts to increment the count of dropped events for this buffer component.
    ///
    /// If the component itself is not configured to drop events, this call does nothing.
    pub fn increment_dropped_event_count(&self, count: u64, size: u64, intentional: bool) {
        if intentional {
            self.state.dropped_intentional.increase(count, size)
        } else {
            self.state.dropped.increase(count, size)
        }
    }
}

#[derive(Debug, Default)]
struct CountMetrics {
    count: AtomicU64,
    byte_size: AtomicU64,
}

impl CountMetrics {
    fn set(&self, count: u64, size: u64) {
        self.count.store(count, Ordering::SeqCst);
        self.byte_size.store(size, Ordering::SeqCst);
    }

    fn count(&self) -> u64 {
        self.count.load(Ordering::SeqCst)
    }

    fn byte_size(&self) -> u64 {
        self.byte_size.load(Ordering::SeqCst)
    }

    fn increase(&self, count: u64, byte_size: u64) {
        self.count.fetch_add(count, Ordering::SeqCst);
        self.byte_size.fetch_add(byte_size, Ordering::SeqCst);
    }
}

#[derive(Debug)]
pub struct BufferUsageData {
    idx: usize,

    received: CountMetrics,
    sent: CountMetrics,
    dropped: CountMetrics,
    dropped_intentional: CountMetrics,
    max_size: CountMetrics,
}

impl BufferUsageData {
    pub fn new(idx: usize) -> Self {
        Self {
            idx,

            received: CountMetrics::default(),
            sent: CountMetrics::default(),
            dropped: CountMetrics::default(),
            dropped_intentional: CountMetrics::default(),
            max_size: CountMetrics::default(),
        }
    }

    fn snapshot(&self) -> BufferUsageSnapshot {
        BufferUsageSnapshot {
            received_event_count: self.received.count(),
            received_byte_size: self.received.byte_size(),
            sent_event_count: self.sent.count(),
            sent_byte_size: self.sent.byte_size(),
            dropped_event_count: self.dropped.count(),
            dropped_event_byte_size: self.dropped.byte_size(),
            dropped_event_count_intentional: self.dropped_intentional.count(),
            dropped_event_byte_size_intentional: self.dropped_intentional.byte_size(),
            max_size_bytes: self.max_size.byte_size(),
            max_size_events: self.max_size.count() as usize,
        }
    }
}

#[derive(Debug)]
pub struct BufferUsageSnapshot {
    pub received_event_count: u64,
    pub received_byte_size: u64,
    pub sent_event_count: u64,
    pub sent_byte_size: u64,
    pub dropped_event_count: u64,
    pub dropped_event_byte_size: u64,
    pub dropped_event_count_intentional: u64,
    pub dropped_event_byte_size_intentional: u64,
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
    pub fn add_stage(&mut self, idx: usize) -> BufferUsageHandle {
        let data = Arc::new(BufferUsageData::new(idx));
        let handle = BufferUsageHandle {
            state: Arc::clone(&data),
        };

        self.stages.push(data);
        handle
    }

    #[allow(clippy::cast_precision_loss)]
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

                        match stage.max_size.byte_size() {
                            0 => {}
                            value => max_byte_size.recorder(attrs.clone()).set(value as f64),
                        };
                        match stage.max_size.count() {
                            0 => {}
                            value => max_event_size.recorder(attrs.clone()).set(value as f64),
                        };

                        dropped_events
                            .recorder(attrs.clone())
                            .set(stage.dropped.count());

                        received_events
                            .recorder(attrs.clone())
                            .set(stage.received.count());
                        received_bytes
                            .recorder(attrs.clone())
                            .set(stage.received.byte_size());
                        sent_events.recorder(attrs.clone()).set(stage.sent.count());
                        sent_bytes.recorder(attrs).set(stage.sent.byte_size());
                    }
                }
            }
            .instrument(span.or_current()),
        );
    }
}
