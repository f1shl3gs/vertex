use chrono::Utc;
use std::any::TypeId;
use std::borrow::Cow;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::Instant;

use event::trace::generator::IdGenerator;
use event::trace::{AnyValue, EvictedHashMap, Key, RngGenerator, StatusCode};
use tracing_core::span::{Attributes, Id, Record};
use tracing_core::{Event, Field, Subscriber};
use tracing_log::NormalizeEvent;
use tracing_subscriber::layer::Context;
use tracing_subscriber::{registry, Layer};

use super::context::TraceContext;

pub struct TraceLayer<S, L>
where
    S: Subscriber,
    L: Layer<S> + Sized,
{
    tracked_inactivity: bool,
    event_location: bool,
    generator: RngGenerator,
    get_context: WithContext,

    inner: L,
    _subscriber: PhantomData<S>,
}

impl<S, L> Layer<S> for TraceLayer<S, L>
where
    L: Layer<S>,
    S: Subscriber + for<'a> registry::LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();

        if self.tracked_inactivity && extensions.get_mut::<Timings>().is_none() {
            extensions.insert(Timings::new())
        }

        let meta = attrs.metadata();
        let mut span = event::trace::Span::new(meta.name())
            .with_start_time(now())
            .with_span_id(self.generator.new_span_id());

        if let Some(filename) = meta.file() {
            span.attributes
                .insert("code.filename".into(), filename.into());
        }

        if let Some(module) = meta.module_path() {
            span.attributes
                .insert("code.namespace".into(), module.into());
        }

        if let Some(line) = meta.line() {
            span.attributes.insert("code.lineno".into(), line.into());
        }

        attrs.record(&mut SpanAttributeVisitor(&mut span.attributes));
        extensions.insert(span);
    }

    /// Record `attributes` for the given values.
    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();

        if let Some(data) = extensions.get_mut::<VData>() {
            values.record(&mut SpanAttributeVisitor(&mut data.span.attributes))
        }
    }

    fn on_follows_from(&self, id: &Id, follows: &Id, ctx: Context<'_, S>) {
        let span = ctx.span(id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();

        let follows_span = ctx
            .span(follows)
            .expect("Span to follow not found, this is a bug");

        let mut follows_extensions = follows_span.extensions_mut();

        // TODO: links
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        // Ignore events that are not in the context of a span
        if let Some(span) = ctx.lookup_current() {
            // Performing read operations before getting a write lock to avoid a deadlock
            // See https://github.com/tokio-rs/tracing/issues/763

            let meta = event.metadata();
            let normalized_meta = event.normalized_metadata();
            let target = Key::new("target").string(meta.target());

            let mut vtx_event = ::event::trace::Event {
                name: Cow::default(),
                timestamp: now(),
                attributes: vec![Key::new("level").string(meta.level().as_str()), target].into(),
            };

            event.record(&mut SpanEventVisitor(&mut vtx_event));

            let mut extensions = span.extensions_mut();
            if let Some(VData { mut span, .. }) = extensions.get_mut::<VData>() {
                if span.status.status_code.is_unset() && *meta.level() == tracing_core::Level::ERROR
                {
                    span.status.status_code = StatusCode::Error;
                }

                if self.event_location {
                    let mut attrs = &span.attributes;

                    let (file, module) = match &normalized_meta {
                        Some(meta) => (
                            meta.file().map(|s| AnyValue::from(s.to_owned())),
                            meta.module_path().map(|s| AnyValue::from(s.to_owned())),
                        ),

                        None => (
                            event.metadata().file().map(AnyValue::from),
                            event.metadata().module_path().map(AnyValue::from),
                        ),
                    };

                    if let Some(file) = file {
                        attrs.insert("code.filepaht".into(), file);
                    }
                    if let Some(module) = module {
                        attrs.insert("code.namespace".into(), module);
                    }
                    if let Some(line) = meta.line() {
                        attrs.insert("code.lineno".into(), line.into());
                    }
                }

                span.events.push(vtx_event);
            }
        }
    }

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

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        let span = ctx.span(&id).expect("Span not found, this is a bug");
        let mut extensions = span.extensions_mut();

        if let Some(VData {
            mut span,
            parent_cx,
        }) = extensions.remove::<VData>()
        {
            if self.tracked_inactivity {
                // Append busy/idle timings when enabled
                if let Some(timings) = extensions.get_mut::<Timings>() {
                    span.attributes
                        .insert("busy_ns".into(), timings.busy.into());
                    span.attributes
                        .insert("idle_ns".into(), timings.idle.into());
                }
            }

            // Assign end time, build and start span, drop span to export
            span.with_end_time(now());
        }
    }

    // SAFETY: this is safe because the `WithContext` function pointer is
    // valid for the lifetime of `&self`
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

impl<S, L> TraceLayer<S, L>
where
    L: Layer<S> + Sized,
    S: Subscriber + for<'a> registry::LookupSpan<'a>,
{
    /// Retrieve the parent `Context` from the current tracing
    /// `span` through the `Registry`. This `Context` links spans to their
    /// parent for proper hierarchical visualization.
    fn parent_context(&self, attrs: &Attributes<'_>, ctx: &Context<'_, S>) -> TraceContext {
        // If a span is specified, it _should_ exist in the underlying `Registry`.
        if let Some(parent) = attrs.parent() {
            let span = ctx.span(parent).expect("Span not found, this is a bug");
            let mut extensions = span.extensions_mut();
            extensions
                .get_mut::<VData>()
                .map(|builder| self.tracer.sampled_context(builder))
                .unwrap_or_default()
            // Else if the span is inferred from context, look up any available current span.
        } else if attrs.is_contextual() {
            ctx.lookup_current()
                .and_then(|span| {
                    let mut extensions = span.extensions_mut();
                    extensions
                        .get_mut::<VData>()
                        .map(|builder| self.tracer.sampled_context(builder))
                })
                .unwrap_or_else(TraceContext::current)
            // Explicit root spans should have no parent context.
        } else {
            TraceContext::new()
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

pub(crate) const DEFAULT_MAX_ATTRIBUTES_PER_SPAN: u32 = 128;

fn now() -> i64 {
    Utc::now().timestamp()
}

struct SpanAttributeVisitor<'a>(&'a mut EvictedHashMap);

impl<'a> tracing_subscriber::field::Visit for SpanAttributeVisitor<'a> {
    /// Set attributes on the underlying Span for `f64`
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.0.insert(Key::new(field.name()), value.into())
    }

    /// Set attributes on the underlying EvictedHashMap from `i64`
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.0.insert(Key::new(field.name), value.into())
    }

    /// Set attributes on the underlying EvictedHashMap from `bool` values.
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.0.insert(Key::new(field.name()), value.into())
    }

    /// Set attributes on the underlying EvictedHashMap from `&str` values
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(Key::new(field.name()), value.into())
    }

    /// Set attributes on the underlying EvictedHashmap from values that
    /// implement `Debug`.
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.0
            .insert(Key::new(field.name()), format!("{:?}", value).into())
    }
}

struct SpanEventVisitor<'a>(&'a mut event::trace::Event);

impl<'a> SpanEventVisitor<'a> {
    fn record(&mut self, field: &Field, value: impl Into<AnyValue>) {
        match field.name() {
            "message" => self.0.name = value.to_string().into(),
            name => {
                if name.starts_with("log.") {
                    return;
                }

                self.0.attributes.insert(name.into(), value);
            }
        }
    }
}

impl<'a> tracing::field::Visit for SpanEventVisitor<'a> {
    /// Record events on the underlying [`Span`] from `bool` values.
    fn record_bool(&mut self, field: &Field, value: bool) {
        self.record(field, value)
    }

    /// Record events on the underlying `Span` from `f64` values.
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.record(field, value)
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.record(field, format!("{:?}", value));
    }

    /// Record events on the underlying `Span` from `i64` values
    fn record_i64(&mut self, field: &Field, value: i64) {
        self.record(field, value)
    }

    /// Record events on the underlying `Span` from `&str` values.
    fn record_str(&mut self, field: &Field, value: &str) {
        self.record(field, value)
    }
}

// this function "remembers" the types of the subscriber so that we
// can downcast to something aware of them without knowing those
// types at the callsite.
//
// See https://github.com/tokio-rs/tracing/blob/4dad420ee1d4607bad79270c1520673fa6266a3d/tracing-error/src/layer.rs
pub(crate) struct WithContext(
    fn(
        &tracing::Dispatch,
        &tracing_core::span::Id,
        f: &mut dyn FnMut(&mut VData, &dyn super::tracer::PreSampledTracer),
    ),
);

impl WithContext {
    // This function allows a function to be called in the context of the
    // "remembered" subscriber.
    pub(crate) fn with_context<'a>(
        &self,
        dispatch: &'a tracing::Dispatch,
        id: &tracing_core::span::Id,
        mut f: impl FnMut(&mut VData, &dyn super::tracer::PreSampledTracer),
    ) {
        (self.0)(dispatch, id, &mut f)
    }
}
