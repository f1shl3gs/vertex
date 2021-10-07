pub mod metric;
mod common;

pub use common::*;

pub trait InternalEvent {
    fn emit_logs(&self) {}
    fn emit_metrics(&self) {}
}

#[inline]
pub fn emit(ev: &impl InternalEvent) {
    ev.emit_logs();
    ev.emit_metrics();
}

#[macro_export]
macro_rules! emit {
    ($event: expr) => {
        internal::emit($event)
    };
}