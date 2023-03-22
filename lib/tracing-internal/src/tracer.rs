use event::trace::{Span, SpanContext, SpanId, TraceFlags, TraceId};

use crate::context::Context;

/// Per-span data tracked by this crate
#[derive(Clone, Debug)]
pub struct TraceData {
    /// The parent `TraceContext` for the current tracing span.
    pub parent_cx: Context,

    /// The span data recorded during the current tracing span.
    pub span: Span,
}

/// An interface for building pre-sampled tracers
pub trait PreSampledTracer {
    // Export the span to back end
    fn export(&self, span: Span);

    fn sampled_context(&self, data: &mut TraceData) -> Context {
        let parent_cx = &data.parent_cx;
        let span = &mut data.span;

        // Gather trace state
        let (mut trace_id, parent_trace_flags) = current_trace_state(span, parent_cx);
        if trace_id == TraceId::INVALID {
            trace_id = self.new_trace_id();
        }

        // TODO: implement sampling
        let trace_flags = parent_trace_flags | TraceFlags::SAMPLED;
        let trace_state = parent_cx.span().span_context().trace_state.clone();

        let span_id = span.span_id();
        let span_context = SpanContext::new(trace_id, span_id, trace_flags, false, trace_state);

        parent_cx.with_remote_span_context(span_context)
    }

    fn new_trace_id(&self) -> TraceId;

    fn new_span_id(&self) -> SpanId;
}

fn current_trace_state(span: &Span, parent_cx: &Context) -> (TraceId, TraceFlags) {
    if parent_cx.has_active_span() {
        let span = parent_cx.span();
        let sc = span.span_context();
        (sc.trace_id, sc.trace_flags)
    } else {
        (
            span.trace_id().unwrap_or(TraceId::INVALID),
            TraceFlags::default(),
        )
    }
}
