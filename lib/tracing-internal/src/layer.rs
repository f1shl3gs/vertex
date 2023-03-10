use std::any::TypeId;
use std::fmt::Debug;
use std::marker;
use std::marker::PhantomData;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use event::trace::{EvictedHashMap, Link, Span, StatusCode, TraceId};
use tracing::span::Attributes;
use tracing::{span, Subscriber};
use tracing_core::span::{Id, Record};
use tracing_core::{Event, Field};
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

use crate::context::{Context as TraceContext, WithContext};
use crate::tracer::{PreSampledTracer, TraceData};

pub struct TracingLayer<S, T> {
    tracer: T,
    location: bool,
    tracked_inactivity: bool,
    get_context: WithContext,
    _registry: PhantomData<S>,
}

impl<S, T> TracingLayer<S, T>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    T: PreSampledTracer + 'static,
{
    pub fn new(tracer: T) -> Self {
        Self {
            tracer,
            location: true,
            tracked_inactivity: true,
            get_context: WithContext(Self::get_context),
            _registry: PhantomData,
        }
    }

    fn get_context(
        dispatch: &tracing::Dispatch,
        id: &span::Id,
        f: &mut dyn FnMut(&mut TraceData, &dyn PreSampledTracer),
    ) {
        let subscriber = dispatch
            .downcast_ref::<S>()
            .expect("Subscriber should downcast to expected type; this is a bug");
        let span = subscriber
            .span(id)
            .expect("Registry should have a span for the current ID");
        let subscriber = dispatch
            .downcast_ref::<TracingLayer<S, T>>()
            .expect("Subscriber should downcast to expected type; this is a bug");

        let mut extensions = span.extensions_mut();
        if let Some(data) = extensions.get_mut::<TraceData>() {
            f(data, &subscriber.tracer);
        }
    }

    /// Retrieve the parent `TraceContext` from the current tracing `span` through the
    /// `Registry`. This `TraceContext` links spans to their parent for proper
    /// hierarchical visualization.
    fn parent_context(&self, attrs: &Attributes<'_>, ctx: &Context<'_, S>) -> TraceContext {
        // If a span is specified, it should exist in the underlying `Registry`.
        if let Some(parent) = attrs.parent() {
            let span = ctx.span(parent).expect("Span not found, this is a bug");
            let mut extensions = span.extensions_mut();

            extensions
                .get_mut::<TraceData>()
                .map(|data| self.tracer.sampled_context(data))
                .unwrap_or_default()
        } else if attrs.is_contextual() {
            // Else if the span is inferred from context, look up any available current span.
            ctx.lookup_current()
                .and_then(|span| {
                    let mut extensions = span.extensions_mut();
                    extensions
                        .get_mut::<TraceData>()
                        .map(|data| self.tracer.sampled_context(data))
                })
                .unwrap_or_else(TraceContext::current)
        } else {
            // Explicit root spans should have no parent context
            TraceContext::new()
        }
    }
}

impl<S, T> Layer<S> for TracingLayer<S, T>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    T: PreSampledTracer + 'static,
{
    /// Creates a `Span` for the corresponding tracing::Span
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();

        if self.tracked_inactivity && extensions.get_mut::<Timings>().is_none() {
            extensions.insert(Timings::new());
        }

        let parent_cx = self.parent_context(attrs, &ctx);
        let metadata = attrs.metadata();
        let mut span = Span::new(metadata.name())
            .with_start_time(now())
            // Eagerly assign span id so children have stable parent id
            .with_span_id(self.tracer.new_span_id());

        // Record new trace id if there is no active parent span
        if !parent_cx.has_active_span() {
            span.span_context.trace_id = self.tracer.new_trace_id();
        }

        if self.location {
            if let Some(filename) = metadata.file() {
                span.tags.insert("code.filepath", filename);
            }

            if let Some(module) = metadata.module_path() {
                span.tags.insert("code.namespace", module);
            }

            if let Some(line) = metadata.line() {
                span.tags.insert("code.lineno", line)
            }
        }

        attrs.record(&mut SpanAttributeVisitor(&mut span));
        extensions.insert(TraceData { parent_cx, span })
    }

    /// Record `attributes` for the given values
    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();
        if let Some(data) = extensions.get_mut::<TraceData>() {
            values.record(&mut SpanAttributeVisitor(&mut data.span))
        }
    }

    /// Notifies this layer that a span with the `span_id` recorded that it
    /// follows from the span with the ID `follows`.
    fn on_follows_from(&self, id: &Id, follows: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();
        let data = extensions
            .get_mut::<TraceData>()
            .expect("Missing trace data span extensions");

        let follows_span = ctx
            .span(follows)
            .expect("Span to follow not found, this is a bug");
        let mut follows_extensions = follows_span.extensions_mut();
        let follows_data = follows_extensions
            .get_mut::<TraceData>()
            .expect("Missing trace data span extension");

        let follows_context = self
            .tracer
            .sampled_context(follows_data)
            .span()
            .span_context()
            .clone();

        let follows_link = Link::new(follows_context.trace_id, follows_context.span_id);
        let links = &mut data.span.links;
        links.push_back(follows_link);
    }

    /// Records `Event` data on event
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        // Ignore events that are not in the context of a span
        if let Some(span) = ctx.lookup_current() {
            // Performance read operations before getting a write lock to avoid
            // a dead lock.
            //
            // See https://github.com/tokio-rs/tracing/issues/763
            let metadata = event.metadata();

            let mut te = {
                let mut attributes = EvictedHashMap::new(128, 4);
                attributes.insert("level", metadata.level().as_str());
                attributes.insert("target", metadata.target());

                event::trace::Event {
                    name: Default::default(),
                    timestamp: now(),
                    attributes,
                }
            };

            event.record(&mut SpanEventVisitor(&mut te));

            let mut extensions = span.extensions_mut();
            if let Some(TraceData { span, .. }) = extensions.get_mut::<TraceData>() {
                if span.status.status_code.is_unset()
                    && *metadata.level() == tracing_core::Level::ERROR
                {
                    span.status.status_code = StatusCode::Error;
                }

                if self.location {
                    if let Some(file) = metadata.file() {
                        span.tags.insert("code.filepath", file);
                    }

                    if let Some(module) = metadata.module_path() {
                        span.tags.insert("code.namespace", module);
                    }

                    if let Some(line) = metadata.line() {
                        span.tags.insert("code.lineno", line);
                    }
                }

                span.events.push_back(te);
            }
        }
    }

    /// Notifies this layer that an event has occurred.
    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        if !self.tracked_inactivity {
            return;
        }

        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();

        if let Some(timings) = extensions.get_mut::<Timings>() {
            let now = Instant::now();
            timings.idle += (now - timings.last).as_nanos() as i64;
            timings.last = now;
        }
    }

    /// Notifies this layer that the span with the given ID was exited.
    fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
        if !self.tracked_inactivity {
            return;
        }

        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();

        if let Some(timings) = extensions.get_mut::<Timings>() {
            let now = Instant::now();
            timings.busy += (now - timings.last).as_nanos() as i64;
            timings.last = now;
        }
    }

    /// Exports a `Span` on close
    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let span = ctx.span(&id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();

        if let Some(TraceData {
            mut span,
            parent_cx,
        }) = extensions.remove::<TraceData>()
        {
            if self.tracked_inactivity {
                // Append busy/idle timings when enabled.
                if let Some(timings) = extensions.get_mut::<Timings>() {
                    span.tags.insert("busy_ns", timings.busy);
                    span.tags.insert("idle_ns", timings.idle);
                }
            }

            let parent_span = if parent_cx.has_active_span() {
                Some(parent_cx.span())
            } else {
                None
            };

            if let Some(psc) = parent_span.as_ref().map(|parent| parent.span_context()) {
                span.span_context.trace_id = psc.trace_id;
            } else if span.span_context.trace_id == TraceId::INVALID {
                span.span_context.trace_id = self.tracer.new_trace_id()
            }

            // Assign end time, build and start span, drop span to exporter
            span = span.with_end_time(now());
            span = span.with_parent_span_id(parent_cx.span().span_context().span_id);

            self.tracer.export(span);
        }
    }

    unsafe fn downcast_raw(&self, id: TypeId) -> Option<*const ()> {
        match id {
            id if id == TypeId::of::<Self>() => Some(self as *const Self as *const ()),
            id if id == TypeId::of::<WithContext>() => {
                Some(&self.get_context as *const WithContext as *const ())
            }
            _ => None,
        }
    }
}

struct Timings {
    idle: i64,
    busy: i64,
    last: Instant,
}

impl Timings {
    fn new() -> Self {
        Self {
            idle: 0,
            busy: 0,
            last: Instant::now(),
        }
    }
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as i64
}

struct SpanAttributeVisitor<'a>(&'a mut Span);

impl<'a> SpanAttributeVisitor<'a> {}

impl<'a> tracing::field::Visit for SpanAttributeVisitor<'a> {
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.0.tags.insert(field.name(), value)
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.0.tags.insert(field.name(), value)
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.0.tags.insert(field.name(), value)
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.tags.insert(field.name(), value)
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.0.tags.insert(field.name(), format!("{:?}", value))
    }
}

struct SpanEventVisitor<'a>(&'a mut event::trace::Event);

impl<'a> tracing::field::Visit for SpanEventVisitor<'a> {
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.0.attributes.insert(field.name(), value)
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.0.attributes.insert(field.name(), value)
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.0.attributes.insert(field.name(), value)
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.attributes.insert(field.name(), value)
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.0
            .attributes
            .insert(field.name(), format!("{:?}", value))
    }
}
