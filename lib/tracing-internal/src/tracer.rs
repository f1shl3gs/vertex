use event::trace::{Span, SpanContext, SpanId, TraceFlags, TraceId, TraceState};

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
        let (_no_parent, trace_id, _remote_parent, _parent_trace_flags) =
            current_trace_state(span, parent_cx);
        let trace_id = if trace_id == TraceId::INVALID {
            self.new_trace_id()
        } else {
            trace_id
        };

        // TODO: Sample or defer to existing sampling decisions

        let span_id = span.span_context.span_id;
        let span_context = SpanContext::new(
            trace_id,
            span_id,
            TraceFlags::SAMPLED,
            false,
            TraceState::default(),
        );

        parent_cx.with_remote_span_context(span_context)
    }

    fn new_trace_id(&self) -> TraceId;

    fn new_span_id(&self) -> SpanId;
}

fn current_trace_state(span: &Span, parent_cx: &Context) -> (bool, TraceId, bool, TraceFlags) {
    if parent_cx.has_active_span() {
        let span = parent_cx.span();
        let sc = span.span_context();
        (false, sc.trace_id, sc.is_remote, sc.trace_flags)
    } else {
        (
            true,
            span.trace_id().unwrap_or(TraceId::INVALID),
            false,
            Default::default(),
        )
    }
}
