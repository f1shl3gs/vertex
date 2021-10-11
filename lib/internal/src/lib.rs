pub mod metric;
mod common;
mod sample;

#[macro_use]
extern crate metrics;

pub use common::*;

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