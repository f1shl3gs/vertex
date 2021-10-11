pub mod metric;
mod common;
mod sample;
mod kafka;

#[macro_use]
extern crate metrics;

pub use common::*;
pub use kafka::*;

pub trait InternalEvent {
    fn emit_logs(&self) {}
    fn emit_metrics(&self) {}
}

#[inline]
pub fn emit(ev: impl InternalEvent) {
    ev.emit_logs();
    ev.emit_metrics();
}

#[macro_export]
macro_rules! emit {
    ($event: expr) => {
        $crate::emit($event)
    };
}

#[macro_export]
macro_rules! update_counter {
    ($label: literal, $value: expr) => {{
        use ::std::sync::atomic::{AtomicU64, Ordering};

        static PREVIOUS_VALUE: AtomicU64 = AtomicU64::new(0);

        let new = $value;
        let mut prev = PREVIOUS_VALUE.load(Ordering::Relaxed);

        loop {
            // Either a new greater value has been emitted before this thread updated the counter
            // or values were provided that are not in strictly monotonically increasing order.
            // Ignore.
            if new <= prev {
                break;
            }

            match PREVIOUS_VALUE.compare_exchange_weak(
                prev,
                new,
                Ordering::SeqCst,
                Ordering::Relaxed
            ) {
                Err(val) => prev = val,
                Ok(_) => {
                    let delta = new - prev;
                    // note that this sequence of deltas might be emitted in a different
                    // order than they were calculated.
                    counter!($label, delta);
                    break;
                }
            }
        }
    }};
}