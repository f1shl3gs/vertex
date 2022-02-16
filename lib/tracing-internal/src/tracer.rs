use event::trace::generator::IdGenerator;
use event::trace::{RngGenerator, Span, SpanId, TraceId};

use crate::context::TraceContext;

/// Per-span data tracked by this crate
#[derive(Clone, Debug)]
pub struct TraceData {
    /// The parent `TraceContext` for the current tracing span.
    pub parent_cx: TraceContext,

    /// The span data recorded during the current tracing span.
    pub span: Span,
}

/// An interface for building pre-sampled tracers
pub trait PreSampledTracer {
    // Export the span to back end
    fn export(&self, span: Span);

    fn sampled_context(&self, data: &mut TraceData) -> TraceContext {
        println!("{} {}", data.span.span_id().into_i64(), data.span.name);

        data.parent_cx.clone()
    }

    fn new_trace_id(&self) -> TraceId;

    fn new_span_id(&self) -> SpanId;
}

pub struct FormatTracer {
    gen: RngGenerator,
}

impl FormatTracer {
    pub fn new() -> Self {
        Self {
            gen: RngGenerator::default(),
        }
    }
}

impl PreSampledTracer for FormatTracer {
    fn export(&self, span: Span) {
        println!("{:#?}", span);
    }

    fn sampled_context(&self, data: &mut TraceData) -> TraceContext {
        data.parent_cx.clone()
    }

    fn new_trace_id(&self) -> TraceId {
        self.gen.new_trace_id()
    }

    fn new_span_id(&self) -> SpanId {
        self.gen.new_span_id()
    }
}
