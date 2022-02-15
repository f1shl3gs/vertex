use crate::layer::TraceContextRegistry;
use event::trace::{EvictedHashMap, SpanId, TraceId};
use std::fmt::Debug;
use std::time::SystemTime;
use tracing::field::Field;
use tracing_subscriber::registry::LookupSpan;

/// Register the current span as the local root of a distributed trace.
pub fn register_dist_tracing_root<SpanId, TraceId>(
    trace_id: TraceId,
    remote_parent_span: Option<SpanId>,
) -> Result<(), TraceContextError>
where
    SpanId: 'static + Clone + Send + Sync,
    TraceId: 'static + Clone + Send + Sync,
{
    let span = tracing::Span::current();
    span.with_subscriber(|(current_span_id, dispatch)| {
        if let Some(trace_ctx_registry) =
            dispatch.downcast_ref::<TraceContextRegistry<SpanId, TraceId>>()
        {
            trace_ctx_registry.record_trace_ctx(
                trace_id,
                remote_parent_span,
                current_span_id.clone(),
            );
            Ok(())
        } else {
            Err(TraceContextError::TelemetryLayerNotRegistered)
        }
    })
    .ok_or(TraceContextError::NoEnabledSpan)?
}

/// Retrieve the distributed trace context associated with the current span. Returns the
/// `TraceId`, if any, that the current span is associated with along with the `SpanId`
/// belonging to the current span.
pub fn current_dist_trace_ctx<SpanId, TraceId>() -> Result<(TraceId, SpanId), TraceContextError>
where
    SpanId: 'static + Clone + Send + Sync,
    TraceId: 'static + Clone + Send + Sync,
{
    let span = tracing::Span::current();
    span.with_subscriber(|(current_span_id, dispatch)| {
        let trace_ctx_registry = dispatch
            .downcast_ref::<TraceContextRegistry<SpanId, TraceId>>()
            .ok_or(TraceContextError::TelemetryLayerNotRegistered)?;

        let registry = dispatch
            .downcast_ref::<tracing_subscriber::Registry>()
            .ok_or(TraceContextError::RegistrySubscriberNotRegistered)?;

        let iter = itertools::unfold(Some(current_span_id.clone()), |st| match st {
            Some(target_id) => {
                // failure here indicates a broken parent id span link, panic is valid
                let res = registry
                    .span(target_id)
                    .expect("span data not found during eval_ctx for current_trace_ctx");
                *st = res.parent().map(|x| x.id());
                Some(res)
            }
            None => None,
        });

        trace_ctx_registry
            .eval_ctx(iter)
            .map(|x| {
                (
                    x.trace_id,
                    trace_ctx_registry.promote_span_id(current_span_id.clone()),
                )
            })
            .ok_or(TraceContextError::NoParentNodeHasTraceCtx)
    })
    .ok_or(TraceContextError::NoEnabledSpan)?
}

/// Errors that can occur while registering the current span as a distributed trace root or
/// attempting to retrieve the current trace context.
#[derive(PartialEq, Eq, Hash, Clone, Debug)]
#[non_exhaustive]
pub enum TraceContextError {
    /// Expected a `TelemetryLayer` to be registered as a subscriber associated with the current Span.
    TelemetryLayerNotRegistered,
    /// Expected a `tracing_subscriber::Registry` to be registered as a subscriber associated with the current Span.
    RegistrySubscriberNotRegistered,
    /// Expected the span returned by `tracing::Span::current()` to be enabled, with an associated subscriber.
    NoEnabledSpan,
    /// Attempted to evaluate the current distributed trace context but none was found. If this occurs, you should check to make sure that `register_dist_tracing_root` is called in some parent of the current span.
    NoParentNodeHasTraceCtx,
}

impl std::fmt::Display for TraceContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use TraceContextError::*;
        write!(f, "{}",
               match self {
                   TelemetryLayerNotRegistered => "`TelemetryLayer` is not a registered subscriber of the current Span",
                   RegistrySubscriberNotRegistered => "no `tracing_subscriber::Registry` is a registered subscriber of the current Span",
                   NoEnabledSpan => "the span is not enabled with an associated subscriber",
                   NoParentNodeHasTraceCtx => "unable to evaluate trace context; assert `register_dist_tracing_root` is called in some parent span",
               })
    }
}

impl std::error::Error for TraceContextError {}

pub struct SpanAttributeVisitor<'a>(&'a mut EvictedHashMap);

impl<'a> tracing::field::Visit for SpanAttributeVisitor<'a> {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        todo!()
    }
}

pub struct SpanEventVisitor<'a>(&'a mut event::trace::Event);

impl<'a> tracing::field::Visit for SpanEventVisitor<'a> {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        todo!()
    }
}

/// A `Span` holds ready-to-publish information gathered during the lifetime of a `tracing::Span`.
#[derive(Debug, Clone)]
pub struct Span<Visitor> {
    /// id identifying this span
    pub id: SpanId,
    /// `TraceId` identifying the trace to which this span belongs
    pub trace_id: TraceId,
    /// optional parent span id
    pub parent_id: Option<SpanId>,
    /// UTC time at which this span was initialized
    pub initialized_at: SystemTime,
    /// `chrono::Duration` elapsed between the time this span was initialized and the time it was completed
    pub completed_at: SystemTime,
    /// `tracing::Metadata` for this span
    pub meta: &'static tracing::Metadata<'static>,
    /// name of the service on which this span occurred
    pub service_name: &'static str,
    /// values accumulated by visiting fields observed by the `tracing::Span` this span was derived from
    pub values: Visitor,
}

/// An `Event` holds ready-to-publish information derived from a `tracing::Event`.
#[derive(Clone, Debug)]
pub struct Event<Visitor> {
    /// `TraceId` identifying the trace to which this event belongs
    pub trace_id: TraceId,
    /// optional parent span id
    pub parent_id: Option<SpanId>,
    /// UTC time at which this event was initialized
    pub initialized_at: SystemTime,
    /// `tracing::Metadata` for this event
    pub meta: &'static tracing::Metadata<'static>,
    /// name of the service on which this event occurred
    pub service_name: &'static str,
    /// values accumulated by visiting the fields of the `tracing::Event` this event was derived from
    pub values: Visitor,
}
