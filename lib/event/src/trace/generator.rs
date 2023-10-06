use std::cell::RefCell;
use std::fmt;

use rand::{rngs, Rng};

use super::{SpanId, TraceId};

/// Interface for generating IDs
pub trait IdGenerator: Send + Sync + fmt::Debug {
    /// Generate a new `TraceId`
    fn new_trace_id(&self) -> TraceId;

    /// Generate a new `SpanId`
    fn new_span_id(&self) -> SpanId;
}

thread_local! {
    /// Store random number generator for each thread
    static CURRENT_RNG: RefCell<rngs::ThreadRng> = RefCell::new(rngs::ThreadRng::default());
}

/// Generates Trace and Span ids using a random number generator
#[derive(Clone, Debug, Default)]
pub struct RngGenerator;

impl IdGenerator for RngGenerator {
    /// Generate new `TraceId` using thread local rng
    fn new_trace_id(&self) -> TraceId {
        CURRENT_RNG.with_borrow_mut(|rng| TraceId::from_bytes(rng.gen::<[u8; 16]>()))
    }

    fn new_span_id(&self) -> SpanId {
        CURRENT_RNG.with_borrow_mut(|rng| SpanId::from_bytes(rng.gen::<[u8; 8]>()))
    }
}
