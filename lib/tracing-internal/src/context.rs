use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Debug, Formatter};
use std::hash::{BuildHasherDefault, Hasher};
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use event::trace::{KeyValue, Span, SpanContext};
use once_cell::sync::Lazy;
use tracing::span;
use tracing_core::Dispatch;

use crate::tracer::{PreSampledTracer, TraceData};

static NOOP_SPAN: Lazy<SynchronizedSpan> = Lazy::new(|| SynchronizedSpan {
    span_context: SpanContext::empty_context(),
    inner: None,
});

/// This function "remembers" the type of the subscriber so that we
/// can downcast to something aware of them without knowing those
/// types at the callsite.
///
/// See https://github.com/tokio-rs/tracing/blob/4dad420ee1d4607bad79270c1520673fa6266a3d/tracing-error/src/layer.rs
pub(crate) struct WithContext(
    pub fn(&Dispatch, &span::Id, f: &mut dyn FnMut(&mut TraceData, &dyn PreSampledTracer)),
);

impl WithContext {
    // This function allows a function to be called in the context of
    // the "remembered" subscriber.
    pub fn with_context(
        &self,
        dispatch: &Dispatch,
        id: &span::Id,
        mut f: impl FnMut(&mut TraceData, &dyn PreSampledTracer),
    ) {
        (self.0)(dispatch, id, &mut f)
    }
}

thread_local! {
    static CURRENT_CONTEXT: RefCell<Context> = RefCell::new(Context::default());
    static DEFAULT_CONTEXT: Context = Context::default();
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
pub struct Context {
    entries: HashMap<TypeId, Arc<dyn Any + Sync + Send>, BuildHasherDefault<IdHasher>>,
}

impl Debug for Context {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TraceContext")
            .field("entries", &self.entries.len())
            .finish()
    }
}

impl Context {
    /// Create an empty `TraceContext`
    pub fn new() -> Self {
        Context::default()
    }

    pub fn current() -> Self {
        get_current(|cx| cx.clone())
    }

    /// Returns a clone of the current thread's context with the given value.
    ///
    /// This is a more efficient from of `Context::current().with_value(value)`
    /// as it avoids the intermediate context clone.
    pub fn current_with_value<T: 'static + Send + Sync>(value: T) -> Self {
        let mut new_context = Context::current();
        new_context
            .entries
            .insert(TypeId::of::<T>(), Arc::new(value));

        new_context
    }

    /// Used to see if a span has been marked as active
    pub fn has_active_span(&self) -> bool {
        self.get::<SynchronizedSpan>().is_some()
    }

    /// Returns a copy of this context with the span context included.
    pub fn with_remote_span_context(&self, span_context: SpanContext) -> Self {
        self.with_value(SynchronizedSpan {
            span_context,
            inner: None,
        })
    }

    /// Returns a reference to the entry for the corresponding value type.
    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.entries
            .get(&TypeId::of::<T>())
            .and_then(|rc| rc.downcast_ref())
    }

    /// Returns a copy of the context with the new value included.
    pub fn with_value<T: 'static + Send + Sync>(&self, value: T) -> Self {
        let mut new_context = self.clone();

        new_context
            .entries
            .insert(TypeId::of::<T>(), Arc::new(value));

        new_context
    }

    /// Replaces the current context on this thread with this context.
    ///
    /// Dropping the returned [`ContextGuard`] will reset the current
    /// context to the previous value.
    pub fn attach(self) -> ContextGuard {
        let previous_cx = Some(CURRENT_CONTEXT.replace(self));

        ContextGuard {
            previous_cx,
            _marker: PhantomData,
        }
    }

    /// Returns a reference to this context's span, or the default no-op span if
    /// none has been set.
    pub fn span(&self) -> SpanRef<'_> {
        if let Some(span) = self.get::<SynchronizedSpan>() {
            SpanRef(span)
        } else {
            SpanRef(&NOOP_SPAN)
        }
    }
}

/// A guard that resets the current context to the prior context when dropped.
#[allow(missing_debug_implementations)]
pub struct ContextGuard {
    previous_cx: Option<Context>,
    // ensure this type is !Send as it relies on thread locals
    _marker: PhantomData<*const ()>,
}

impl Drop for ContextGuard {
    fn drop(&mut self) {
        if let Some(previous_cx) = self.previous_cx.take() {
            CURRENT_CONTEXT.replace(previous_cx);
        }
    }
}

fn get_current<F: FnMut(&Context) -> T, T>(mut f: F) -> T {
    CURRENT_CONTEXT.with_borrow(|cx| f(cx))
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
    #![allow(clippy::print_stderr)]
    fn with_inner_mut<F: FnOnce(&mut Span)>(&self, f: F) {
        if let Some(ref inner) = self.0.inner {
            match inner.lock() {
                Ok(mut locked) => f(&mut locked),
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
        T: Into<String>,
    {
        self.with_inner_mut(|inner| inner.add_event(name, attributes))
    }

    /// Convenience method to record an exception/error as an `Event`
    pub fn record_exception(&self, err: &dyn Error) {
        self.with_inner_mut(|inner| inner.record_exception(err))
    }

    /// Returns the `SpanContext` for the given `Span`.
    pub fn span_context(&self) -> &SpanContext {
        &self.0.span_context
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_context() {
        #[derive(Debug, PartialEq)]
        struct Foo(&'static str);

        #[derive(Debug, PartialEq)]
        struct Bar(u64);

        let _outer_guard = Context::new().with_value(Foo("a")).attach();

        // Only value `a` is set
        let current = Context::current();
        assert_eq!(current.get(), Some(&Foo("a")));
        assert_eq!(current.get::<Bar>(), None);

        {
            let _inner_guard = Context::current_with_value(Bar(42)).attach();
            // Both values are set in inner context
            let current = Context::current();
            assert_eq!(current.get(), Some(&Foo("a")));
            assert_eq!(current.get(), Some(&Bar(42)));
        }

        // Resets to only value `a` when inner guard is dropped.
        let current = Context::current();
        assert_eq!(current.get(), Some(&Foo("a")));
        assert_eq!(current.get::<Bar>(), None);
    }
}
