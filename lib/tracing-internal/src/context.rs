use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Debug, Formatter};
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::{Arc, Mutex};

use event::trace::{KeyValue, Span, SpanContext};
use tracing::span;
use tracing_core::Dispatch;

use crate::tracer::{PreSampledTracer, TraceData};

lazy_static::lazy_static! {
    static ref NOOP_SPAN: SynchronizedSpan = SynchronizedSpan {
        span_context: SpanContext::empty_context(),
        inner: None,
    };
}

/// This function "remembers" the type of the subscriber so that we
/// can downcast to something aware of them without knowing those
/// types at the callsite.
///
/// See https://github.com/tokio-rs/tracing/blob/4dad420ee1d4607bad79270c1520673fa6266a3d/tracing-error/src/layer.rs
pub(crate) struct WithContext(
    pub fn(&Dispatch, &span::Id, f: &mut dyn FnMut(&mut TraceData, &dyn PreSampledTracer)),
);

impl WithContext {}

thread_local! {
    static CURRENT_CONTEXT: RefCell<TraceContext> = RefCell::new(TraceContext::default());
    static DEFAULT_CONTEXT: TraceContext = TraceContext::default();
}

/// With TypeIds as keys, there's no need to hash them. They are already hashes
/// themselves, coming from the compiler. The IdHasher holds the u64 of
/// the TypeId, and then returns it, instead of doing any bit fiddling.
#[derive(Clone, Default, Debug)]
struct IdHasher(u64);

impl Hasher for IdHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, _: &[u8]) {
        unreachable!("TypeId calls write_u64");
    }

    #[inline]
    fn write_u64(&mut self, id: u64) {
        self.0 = id;
    }
}

/// An execution-scoped collection of values.
///
/// A `Context` is a propagation mechanism which carries execution-scoped values
/// across API boundaries and between logically associated execution units.
/// Cross-cutting concerns access their data in-process using the same shared
/// context object.
#[derive(Clone, Default)]
pub struct TraceContext {
    entries: HashMap<TypeId, Arc<dyn Any + Sync + Send>, BuildHasherDefault<IdHasher>>,
}

impl Debug for TraceContext {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TraceContext")
            .field("entries", &self.entries.len())
            .finish()
    }
}

impl TraceContext {
    /// Create an empty `TraceContext`
    pub fn new() -> Self {
        TraceContext::default()
    }

    pub fn current() -> Self {
        get_current(|cx| cx.clone())
    }

    /// Used to see if a span has been marked as active
    pub fn has_active_span(&self) -> bool {
        // TODO:
        false
    }

    /// Returns a reference to the entry for the corresponding value type.
    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.entries
            .get(&TypeId::of::<T>())
            .and_then(|rc| (&*rc).downcast_ref())
    }

    /// Returns a reference to this context's span, or the default no-op span if
    /// none has been set.
    pub fn span(&self) -> SpanRef<'_> {
        if let Some(span) = self.get::<SynchronizedSpan>() {
            SpanRef(span)
        } else {
            SpanRef(&*NOOP_SPAN)
        }
    }
}

fn get_current<F: FnMut(&TraceContext) -> T, T>(mut f: F) -> T {
    CURRENT_CONTEXT
        .try_with(|cx| f(&*cx.borrow()))
        .unwrap_or_else(|_| DEFAULT_CONTEXT.with(|cx| f(&*cx)))
}

/// A reference to the currently active span in this context.
#[derive(Debug)]
pub struct SpanRef<'a>(&'a SynchronizedSpan);

#[derive(Debug)]
struct SynchronizedSpan {
    /// Immutable span context
    span_context: SpanContext,
    /// Mutable span inner that requires synchronization
    inner: Option<Mutex<Span>>,
}

impl SpanRef<'_> {
    fn with_inner_mut<F: FnOnce(&mut Span)>(&self, f: F) {
        if let Some(ref inner) = self.0.inner {
            match inner.lock() {
                Ok(mut locked) => f(&mut *locked),
                Err(err) => {
                    eprintln!("Trace error occurred. {}", err)
                }
            }
        }
    }
}

impl SpanRef<'_> {
    /// An API to record events in the context of a given `Span`.
    pub fn add_event<T>(&self, name: T, attributes: Vec<KeyValue>)
    where
        T: Into<Cow<'static, str>>,
    {
        self.with_inner_mut(|inner| inner.add_event(name, attributes))
    }

    /// Convenience method to record an exception/error as an `Event`
    pub fn record_exception(&self, err: &dyn Error) {
        self.with_inner_mut(|inner| inner.record_exception(err))
    }

    // /// Convenience method to record a exception/error as an `Event` with custom stacktrace
    // pub fn record_exception_with_stacktrace<T>(&self, err: &dyn Error, stacktrace: T)
    // where
    //     T: Into<Cow<'static, str>>,
    // {
    //     self.with_inner_mut(|inner| inner.record_exception_with_stacktrace(err, stacktrace))
    // }

    // /// An API to record events at a specific time in the context of a given `Span`.
    // pub fn add_event_with_timestamp<T>(
    //     &self,
    //     name: T,
    //     timestamp: std::time::SystemTime,
    //     attributes: Vec<KeyValue>,
    // ) where
    //     T: Into<Cow<'static, str>>,
    // {
    //     self.with_inner_mut(move |inner| {
    //         inner.add_event_with_timestamp(name, timestamp, attributes)
    //     })
    // }

    /// Returns the `SpanContext` for the given `Span`.
    pub fn span_context(&self) -> &SpanContext {
        &self.0.span_context
    }

    // /// Returns true if this `Span` is recording information like events with the `add_event`
    // /// operation, attributes using `set_attributes`, status with `set_status`, etc.
    // pub fn is_recording(&self) -> bool {
    //     self.0
    //         .inner
    //         .as_ref()
    //         .and_then(|inner| inner.lock().ok().map(|active| active.is_recording()))
    //         .unwrap_or(false)
    // }

    // /// An API to set a single `Attribute` where the attribute properties are passed
    // /// as arguments. To avoid extra allocations some implementations may offer a separate API for
    // /// each of the possible value types.
    // pub fn set_attribute(&self, attribute: KeyValue) {
    //     self.with_inner_mut(move |inner| inner.set_attribute(attribute))
    // }

    // /// Sets the status of the `Span`. If used, this will override the default `Span`
    // /// status, which is `Unset`. `message` MUST be ignored when the status is `OK` or `Unset`
    // pub fn set_status(&self, code: StatusCode, message: String) {
    //     self.with_inner_mut(move |inner| inner.set_status(code, message))
    // }

    // /// Updates the `Span`'s name. After this update, any sampling behavior based on the
    // /// name will depend on the implementation.
    // pub fn update_name<T>(&self, new_name: String)
    // where
    //     T: Into<Cow<'static, str>>,
    // {
    //     self.with_inner_mut(move |inner| inner.update_name(new_name))
    // }

    // /// Finishes the `Span`.
    // pub fn end(&self) {
    //     self.end_with_timestamp(SystemTime::now());
    // }

    // /// Finishes the `Span` with given timestamp
    // pub fn end_with_timestamp(&self, timestamp: std::time::SystemTime) {
    //     self.with_inner_mut(move |inner| inner.end_with_timestamp(timestamp))
    // }
}
