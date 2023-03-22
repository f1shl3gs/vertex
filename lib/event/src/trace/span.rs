use chrono::Utc;
use std::borrow::Cow;
use std::error::Error;
use std::fmt::{Display, Formatter};

use measurable::ByteSizeOf;
use serde::{Deserialize, Serialize};

use super::{EvictedHashMap, EvictedQueue, KeyValue, SpanId, TraceFlags, TraceId, TraceState};

#[derive(Clone, Debug, PartialOrd, PartialEq, Deserialize, Serialize)]
pub struct Event {
    /// name of the event.
    pub name: Cow<'static, str>,

    /// `timestamp` is the time the event occurred.
    pub timestamp: i64,

    pub attributes: EvictedHashMap,
}

impl ByteSizeOf for Event {
    fn allocated_bytes(&self) -> usize {
        self.name.len() + self.attributes.allocated_bytes()
    }
}

/// `SpanKind` describes the relationship between the Span, its parents,
/// and its children in a `Trace`. `SpanKind` describes two independent
/// properties that benefit tracing systems during analysis.
///
/// The first property described by `SpanKind` reflects whether the `Span`
/// is a remote child or parent. `Span`s with a remote parent are
/// interesting because they are sources of external load. `Span`s with a
/// remote child are interesting because they reflect a non-local system
/// dependency.
///
/// The second property described by `SpanKind` reflects whether a child
/// `Span` represents a synchronous call.  When a child span is synchronous,
/// the parent is expected to wait for it to complete under ordinary
/// circumstances.  It can be useful for tracing systems to know this
/// property, since synchronous `Span`s may contribute to the overall trace
/// latency. Asynchronous scenarios can be remote or local.
///
/// In order for `SpanKind` to be meaningful, callers should arrange that
/// a single `Span` does not serve more than one purpose.  For example, a
/// server-side span should not be used directly as the parent of another
/// remote span.  As a simple guideline, instrumentation should create a
/// new `Span` prior to extracting and serializing the span context for a
/// remote call.
///
/// To summarize the interpretation of these kinds:
///
/// | `SpanKind` | Synchronous | Asynchronous | Remote Incoming | Remote Outgoing |
/// |------------|-----|-----|-----|-----|
/// | `Client`   | yes |     |     | yes |
/// | `Server`   | yes |     | yes |     |
/// | `Producer` |     | yes |     | yes |
/// | `Consumer` |     | yes | yes |     |
/// | `Internal` |     |     |     |     |
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, PartialOrd)]
pub enum SpanKind {
    /// Indicates that the span describes a synchronous request to
    /// some remote service.  This span is the parent of a remote `Server`
    /// span and waits for its response.
    Client,
    /// Indicates that the span covers server-side handling of a
    /// synchronous RPC or other remote request.  This span is the child of
    /// a remote `Client` span that was expected to wait for a response.
    Server,
    /// Indicates that the span describes the parent of an
    /// asynchronous request.  This parent span is expected to end before
    /// the corresponding child `Consumer` span, possibly even before the
    /// child span starts. In messaging scenarios with batching, tracing
    /// individual messages requires a new `Producer` span per message to
    /// be created.
    Producer,
    /// Indicates that the span describes the child of an
    /// asynchronous `Producer` request.
    Consumer,
    /// Default value. Indicates that the span represents an
    /// internal operation within an application, as opposed to an
    /// operations with remote parents or children.
    Internal,

    Unspecified,
}

impl Display for SpanKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SpanKind::Client => write!(f, "client"),
            SpanKind::Server => write!(f, "server"),
            SpanKind::Producer => write!(f, "producer"),
            SpanKind::Consumer => write!(f, "consumer"),
            SpanKind::Internal => write!(f, "internal"),
            SpanKind::Unspecified => write!(f, "unspecified"),
        }
    }
}

/// A pointer from the current span to another span in the same trace or
/// in a different trace. For example, this can be used in batching
/// operations, where a single batch handler processes multiple requests
/// from different traces or when the handler receives a request from a
/// different project.
#[derive(Clone, Debug, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct Link {
    /// The span context of the linked span.
    pub span_context: SpanContext,

    /// `attributes` is a collection of key/value pairs on the link.
    pub attributes: Vec<KeyValue>,
}

impl Link {
    pub fn new(span_context: SpanContext, attributes: Vec<KeyValue>) -> Self {
        Self {
            span_context,
            attributes,
        }
    }

    pub fn trace_id(&self) -> TraceId {
        self.span_context.trace_id
    }

    pub fn span_id(&self) -> SpanId {
        self.span_context.span_id
    }
}

/// For the semantics of status codes see
///
/// <https://github.com/open-telemetry/opentelemetry-specification/blob/main/specification/trace/api.md#set-status>
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, Debug, Deserialize, PartialEq, PartialOrd, Serialize)]
pub enum StatusCode {
    /// The default status
    Unset,
    /// The Span has been validated by an Application developers or Operator
    /// to have completed successfully.
    Ok,
    /// The Span contains an error.
    Error,
}

impl StatusCode {
    /// Return a static str that represent the status code
    pub fn as_str(&self) -> &'static str {
        match self {
            StatusCode::Unset => "",
            StatusCode::Ok => "OK",
            StatusCode::Error => "ERROR",
        }
    }

    pub fn is_unset(&self) -> bool {
        matches!(self, StatusCode::Unset)
    }
}

/// The Status type defines a logical error model  that is suitable for
/// different programing environments, including REST APIs and RPC APIs.
#[derive(Clone, Debug, Deserialize, PartialEq, PartialOrd, Serialize)]
pub struct Status {
    /// A developer-facing human readable error message.
    pub message: Cow<'static, str>,
    /// The status code
    pub status_code: StatusCode,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            message: Cow::from(""),
            status_code: StatusCode::Unset,
        }
    }
}

/// Immutable portion of a `Span` which can be serialized and propagated.
///
/// Spans that do not have the `sampled` flag set in their [`TraceFlags`] will
/// be ignored by most tracing tools.
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash, Eq, Deserialize, Serialize)]
pub struct SpanContext {
    /// A unique identifier for a trace. All spans from the same trace share the same
    /// `trace_id`.
    ///
    /// This field is semantically required. Receiver should generate new random
    /// trace_id if empty or invalid trace_id was received.
    ///
    /// This field is required.
    pub trace_id: TraceId,

    /// A unique identifier for a span within a trace, assigned when the span is
    /// created. Zero is considered invalid.
    ///
    /// This field is semantically required. Receiver should generate new random
    /// `span_id` if empty or invalid `span_id` was received
    pub span_id: SpanId,

    pub trace_flags: TraceFlags,
    pub is_remote: bool,
    pub trace_state: TraceState,
}

impl SpanContext {
    /// Create an invalid empty span context
    pub fn empty_context() -> Self {
        Self {
            trace_id: TraceId::INVALID,
            span_id: SpanId::INVALID,
            trace_flags: TraceFlags::default(),
            is_remote: false,
            trace_state: TraceState::default(),
        }
    }

    /// Construct a new `SpanContext`
    pub fn new(
        trace_id: TraceId,
        span_id: SpanId,
        trace_flags: TraceFlags,
        is_remote: bool,
        trace_state: TraceState,
    ) -> Self {
        Self {
            trace_id,
            span_id,
            trace_flags,
            is_remote,
            trace_state,
        }
    }

    /// Returns `true` if the span context has a valid (non-zero) `trace_id` and
    /// a valid (non-zero) `span_id`.
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.trace_id != TraceId::INVALID && self.span_id != SpanId::INVALID
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, PartialOrd, Serialize)]
pub struct Span {
    pub span_context: SpanContext,

    /// The `span_id` of this span's parent span. If this is a root span, then this
    /// field must be zero.
    pub parent_span_id: SpanId,

    /// A description of the span's operation.
    pub name: String,

    /// Distinguishes between spans generated in a particular context.
    /// For example, two spans with the same name may be distinguished using
    /// `CLIENT`(caller) and `SERVER` (callee) to identify queueing latency
    /// associated with the span.
    pub kind: SpanKind,

    /// start_time is the start time of the span. On the client side, this is the time
    /// kept by the local machine where the span execution starts. On the server side,
    /// this is the time when the server's application handler starts running.
    /// Value is UNIX Epoch time in nanoseconds since 00:000::00 UTC on 1 January 1970.
    ///
    /// This field is semantically required and it is expected that end_time >= start_time.
    pub start_time: i64,

    /// `end_time` is the end time of the span. On the client side, this is the time kept
    /// by the local machine where the span execution ends. On the server side, this is
    /// the time when the server application handler stops running.
    /// Value is UNIX Epoch time in nanoseconds since 00::00:00 UTC on 1 January 1970.
    ///
    /// This field is semantically required and it is expected that end_time >= start_time.
    pub end_time: i64,

    /// `tags` is a collection of key/value pairs. The value can be a string, an
    /// integer, a double or the Boolean value `true` or `false`.
    pub tags: EvictedHashMap,

    /// `events` is a collection of event items.
    pub events: EvictedQueue<Event>,

    /// links is a collection of Links, which are references from this span to a span
    /// in the same or different trace.
    pub links: EvictedQueue<Link>,

    /// An optional final status for this span. Semantically when Status isn't set,
    /// it span's status code is unset, i.e. assume STATUS_CODE_UNSET (code = 0)
    pub status: Status,
}

impl Span {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            span_context: SpanContext {
                trace_id: TraceId::INVALID,
                span_id: SpanId::INVALID,
                trace_flags: TraceFlags::default(),
                is_remote: false,
                trace_state: TraceState::default(),
            },
            parent_span_id: SpanId::INVALID,
            name: name.into(),
            kind: SpanKind::Client,
            start_time: 0,
            end_time: 0,
            tags: EvictedHashMap::new(128, 0),
            events: EvictedQueue::new(128),
            links: EvictedQueue::new(128),
            status: Status::default(),
        }
    }

    pub fn trace_id(&self) -> Option<TraceId> {
        if self.span_context.trace_id == TraceId::INVALID {
            None
        } else {
            Some(self.span_context.trace_id)
        }
    }

    pub fn span_id(&self) -> SpanId {
        self.span_context.span_id
    }

    #[must_use]
    pub fn with_start_time(mut self, start_time: i64) -> Self {
        self.start_time = start_time;
        self
    }

    #[must_use]
    pub fn with_span_id(mut self, id: SpanId) -> Self {
        self.span_context.span_id = id;
        self
    }

    #[must_use]
    pub fn with_trace_id(mut self, id: TraceId) -> Self {
        self.span_context.trace_id = id;
        self
    }

    #[must_use]
    pub fn with_parent_span_id(mut self, id: SpanId) -> Self {
        self.parent_span_id = id;
        self
    }

    #[must_use]
    pub fn with_end_time(mut self, end_time: i64) -> Self {
        self.end_time = end_time;
        self
    }

    /// Returns the `SpanContext` for the given `Span`
    pub fn span_context(&self) -> &SpanContext {
        &self.span_context
    }

    /// Records events in the context of a given `Span`.
    ///
    /// Events have a time associated with the moment when they are added to the `Span`.
    ///
    /// Events should preserve the order in which they're set. This will typically
    /// match the ordering of the events' timestamp.
    pub fn add_event(&mut self, name: impl Into<Cow<'static, str>>, attributes: Vec<KeyValue>) {
        let timestamp = Utc::now().timestamp_nanos();

        self.events.push_back(Event {
            name: name.into(),
            timestamp,
            attributes: attributes.into(),
        });
    }

    /// Convenience method to record an exception/error as an `Event`.
    ///
    /// An exception SHOULD be recorded as an Event on the span during which it occurred.
    /// The name of the event MUST be "exception".
    pub fn record_exception(&mut self, err: &dyn Error) {
        let attrs = vec![KeyValue::new("exception.message", err.to_string())];
        self.add_event("exception", attrs);
    }
}

impl ByteSizeOf for Span {
    fn allocated_bytes(&self) -> usize {
        self.name.allocated_bytes() + self.tags.allocated_bytes() + self.events.allocated_bytes()
    }
}
