use std::borrow::Cow;
use std::collections::HashMap;
use std::time::SystemTime;

use super::{
    context::Context, Event, Key, KeyValue, Link, SpanId, SpanKind, Status, TraceId, TraceState,
};
use crate::tags::Value;

/// This interface for constructing [`Span`]s.
pub trait Tracer {
    /// The Span type used by this tracer
    type Span: Span;

    /// Starts a new `Span`
    ///
    /// By default the currently active `Span` is set as the new `Span`'s parent.
    ///
    /// Each span has zero or one parent span and zero or more child spans, which
    /// represent causally releated operations. A tree of related spans compress
    /// a trace. A span is said to be a root span if it does not have a parent.
    /// Each trace includes a single root span, which is the shared ancestor of
    /// all other spans in the trace.
    fn start<T>(&self, name: T) -> Self::Span
    where
        T: Into<Cow<'static, str>>,
    {
        self.build_with_context(SpanBuilder::from_name(name), &Context::current())
    }

    /// Starts a new [`Span`] with a given context.
    ///
    /// If this context contains a span, the newly created span will be a child of
    /// that span.
    ///
    /// Each span has zero or one parent span and zero or more child spans, which
    /// represent causally related operations. A tree of related spans comprises a
    /// trace. A span is said to be a root span if it does not have a parent. Each
    /// trace includes a single root span, which is the shared ancestor of all other
    /// spans in the trace.
    fn start_with_context<T>(&self, name: T, parent_cx: &Context) -> Self::Span
    where
        T: Into<Cow<'static, str>>,
    {
        self.build_with_context(SpanBuilder::from_name(name), parent_cx)
    }

    /// Creates a span builder.
    /// [`SpanBuilder`]s allow you to specify all attributes of a [`Span`] before
    /// the span is started.
    fn span_builder<T>(&self, name: T) -> SpanBuilder
    where
        T: Into<Cow<'static, str>>,
    {
        SpanBuilder::from_name(name)
    }

    /// Start a [`Span`] from a [`SpanBuilder`]
    fn build(&self, builder: SpanBuilder) -> Self::Span {
        self.build_with_context(builder, &Context::current())
    }

    /// Start a span from a [`SpanBuilder`] with a parent context.
    fn build_with_context(&self, builder: SpanBuilder, parent_cx: &Context) -> Self::Span;

    /// Start a new span and execute the given closure with reference to the
    /// context in which the span is active.
    ///
    /// This method starts a new span and sets it as the active span for the given
    /// function. It then executes the body. It ends the span before returning the
    /// execution result.
    fn in_span<T, F, N>(&self, name: N, f: F) -> T
    where
        F: FnOnce(Context) -> T,
        N: Into<Cow<'static, str>>,
        Self::Span: Send + Sync + 'static,
    {
        let span = self.start(name);
        let cx = Context::curent_with_span(span);
        let _guard = cx.clone().attach();

        f(cx)
    }
}

/// `SpanBuilder` allows span attributes to be configured before the span has
/// started.
#[derive(Clone, Debug, Default)]
pub struct SpanBuilder {
    /// Trace id, useful for integrations with external tracing systems.
    pub trace_id: Option<TraceId>,

    /// Span id, useful for integrations with external tracing systems.
    pub span_id: Option<SpanId>,

    /// Span kind
    pub span_kind: Option<SpanKind>,

    /// Span name
    pub name: Cow<'static, str>,

    /// Span start time.
    pub start_time: Option<SystemTime>,

    /// Span end time
    pub end_time: Option<SystemTime>,

    /// Span attributes
    pub attributes: Option<HashMap<Key, Value>>,

    /// Span events
    pub events: Option<Vec<Event>>,

    /// Span links
    pub links: Option<Vec<Link>>,

    /// Span status
    pub status: Status,

    /// Sampling result
    pub sampling_result: Option<SamplingResult>,
}

impl SpanBuilder {
    /// Create a new span builder from a span name
    pub fn from_name<T: Into<Cow<'static, str>>>(name: T) -> Self {
        SpanBuilder {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Specify trace id to use if no parent context exists.
    pub fn with_trace_id(self, trace_id: TraceId) -> Self {
        SpanBuilder {
            trace_id: Some(trace_id),
            ..self
        }
    }

    /// Assign span id
    pub fn with_span_id(self, span_id: SpanId) -> Self {
        SpanBuilder {
            span_id: Some(span_id),
            ..self
        }
    }

    /// Assign span kind
    pub fn with_kind(self, span_kind: SpanKind) -> Self {
        SpanBuilder {
            span_kind: Some(span_kind),
            ..self
        }
    }

    /// Assign span start time
    pub fn with_start_time<T: Into<SystemTime>>(self, start_time: T) -> Self {
        SpanBuilder {
            start_time: Some(start_time.into()),
            ..self
        }
    }

    /// Assign span end time
    pub fn with_end_time<T: Into<SystemTime>>(self, end_time: T) -> Self {
        SpanBuilder {
            end_time: Some(end_time.into()),
            ..self
        }
    }

    /// Assign span attributes from an iterable.
    pub fn with_attributes<I>(self, attrs: I) -> Self
    where
        I: IntoIterator<Item = KeyValue>,
    {
        SpanBuilder {
            attributes: Some(HashMap::from_iter(attrs)),
            ..self
        }
    }

    /// Assign span attributes.
    pub fn with_attributes_map(self, attributes: HashMap<Key, Value>) -> Self {
        SpanBuilder {
            attributes: Some(attributes),
            ..self
        }
    }

    /// Assign events
    pub fn with_events(self, events: Vec<Event>) -> Self {
        SpanBuilder {
            events: Some(events),
            ..self
        }
    }

    /// Assign links
    pub fn with_links(self, mut links: Vec<Link>) -> Self {
        links.retain(|l| l.span_context.is_valid());

        SpanBuilder {
            links: Some(links),
            ..self
        }
    }

    /// Assign status code
    pub fn with_status(self, status: Status) -> Self {
        SpanBuilder { status, ..self }
    }

    /// Assign sampling result
    pub fn with_sampling_result(self, sampling_result: SamplingResult) -> Self {
        SpanBuilder {
            sampling_result: Some(sampling_result),
            ..self
        }
    }

    /// Builds a span with the given tracer from this configuration.
    pub fn start<T: Tracer>(self, tracer: &T) -> T::Span {
        tracer.build_with_context(self, &Context::current())
    }

    /// Builds a span with the given tracer from this conguration and parent.
    pub fn start_with_context<T: Tracer>(self, tracer: &T, parent_cx: &Context) -> T::Span {
        tracer.build_with_context(self, parent_cx)
    }
}

/// The result of sampling logic for a given span.
#[derive(Clone, Debug, PartialEq)]
pub struct SamplingResult {
    /// The decision about whether or not to sample.
    pub decision: SamplingDecision,

    /// Extra attributes to be added to the span by the sampler.
    pub attributes: Vec<KeyValue>,

    /// Trace state from parent context, may be modified by samplers.
    pub trace_state: TraceState,
}

/// Decision about whether or not to sample.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SamplingDecision {
    /// Span will not be record and all events and attributes will be dropped.
    Drop,

    /// Span data will be record, but not exported.
    RecordOnly,

    /// Span data will be recorded and exported.
    RecordAndSample,
}
