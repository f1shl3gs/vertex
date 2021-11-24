use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Duration;

use crate::WhenFull;

pub struct BufferUsageData {
    received_events: AtomicU64,
    received_bytes: AtomicUsize,
    sent_events: AtomicU64,
    send_bytes: AtomicUsize,
    dropped_events: AtomicU64,
    max_size_bytes: Option<usize>,
}

impl BufferUsageData {
    pub fn new(
        max_size_bytes: Option<usize>,
    ) -> Arc<Self> {
        let buffer_usage_data = Arc::new(Self {
            received_events: AtomicU64::new(0),
            received_bytes: AtomicUsize::new(0),
            sent_events: AtomicU64::new(0),
            send_bytes: AtomicUsize::new(0),
            dropped_events: AtomicU64::new(0),
            max_size_bytes,
        });

        let usage = buffer_usage_data.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(2));

            loop {
                interval.tick().await;

                // TODO: emit events
            }
        });

        usage
    }

    pub fn increment_received_events(&self, count: u64, byte_size: usize) {
        self.received_events.fetch_add(count, Ordering::Relaxed);
        self.received_bytes.fetch_add(byte_size, Ordering::Relaxed);
    }

    pub fn increment_sent_events(&self, count: u64, byte_size: usize) {
        self.sent_events.fetch_add(count, Ordering::Relaxed);
        self.send_bytes.fetch_add(byte_size, Ordering::Relaxed);
    }

    pub fn increment_dropped_events(&self, count: u64) {
        self.dropped_events.fetch_add(count, Ordering::Relaxed);
    }
}
