use event::trace::generator::IdGenerator;
use event::trace::{RngGenerator, Span, SpanId, TraceId};

use super::context::TraceContext;

/// Per-span OpenTelemetry data tracked by this crate.
///
/// Useful for implementing [PreSampledTracer] in alternate otel SDKs.
#[derive(Debug, Clone)]
pub struct TraceData {
    /// The parent otel `Context` for the current tracing span.
    pub parent_cx: TraceContext,

    /// The otel span data recorded during the current tracing span.
    pub span: Span,
}

/// An interface for building pre-sampled tracers.
///
/// The OpenTelemetry spec does not allow trace ids to be updated after a
/// span has been created. In order to associate extracted parent trace ids
/// with existing `tracing` spans. Creates / exports full spans only when
/// the associated `tracing` span is closed. However, in order to properly
/// inject `Context` information to downstream requests, the sampling stats
/// must now be known before the span has been created.
pub trait PreSampledTracer {
    /// Produce an otel context containing an active and pre-sampled span for
    /// the given span builder data.
    ///
    /// The sampling decision, span context information, and parent context
    /// values must match the values recorded when the tracing span is closed.
    fn sampled_context(&self, data: &mut TraceData) -> TraceContext;

    /// Generate a new trace id.
    fn new_trace_id(&self) -> TraceId;

    /// Generate a new span id.
    fn new_span_id(&self) -> SpanId;
}

pub struct Tracer {
    id_gen: RngGenerator,
}

impl PreSampledTracer for Tracer {
    fn sampled_context(&self, data: &mut TraceData) -> TraceContext {
        let parent_cx = &data.parent_cx;

        // Gather trace state
        // let (no_parent, trace_id, remote_parent, parent_trace_flags) = current_trace_state()

        todo!()
    }

    fn new_trace_id(&self) -> TraceId {
        self.id_gen.new_trace_id()
    }

    fn new_span_id(&self) -> SpanId {
        self.id_gen.new_span_id()
    }
}

fn current_trace_state(span: Span, parent_cx: &TraceContext) -> (bool, TraceId, bool, TraceFlags) {}
