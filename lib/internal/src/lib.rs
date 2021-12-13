mod common;
pub mod metric;

extern crate metrics;

pub use common::*;

pub trait InternalEvent {
    fn emit_logs(&self) {}
    fn emit_metrics(&self) {}

    fn name(&self) -> Option<&str> {
        None
    }
}

#[inline]
pub fn emit(ev: &impl InternalEvent) {
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
                Ordering::Relaxed,
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

#[cfg(test)]
mod tests {
    #[macro_export]
    macro_rules! emit2 {
        // metrics only
        (
            [
                // [$($name:expr, $type:ident, $value:expr,  $($label_key:expr => $label_value:literal),* ), *],*
                $( [ $name:expr, $type:ident, $value:expr, $( $label_key:expr => $label_value:expr),* ] ),*
            ]
        ) => {
            $(  $type!(
                $name,
                $value,
                // workable expr
                $($label_key => $label_value,)*
            );)*
        };

        // metrics and logs
        // todo
    }

    #[test]
    fn test() {
        emit2!(
            [
                ["metric_1", counter, 1,],
                ["metric_2", counter, 2, "foo" => "bar"],
                ["metric_3", counter, 3, "foo" => "bar", "key" => "value"]
            ]
        );
    }
}
