mod evicted_hash_map;
mod evicted_queue;

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Formatter};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use shared::ByteSizeOf;

use crate::{BatchNotifier, EventFinalizer, EventFinalizers, EventMetadata, Finalizable};
pub use evicted_hash_map::EvictedHashMap;
pub use evicted_queue::EvictedQueue;

/// Key used for metric `AttributeSet`s and trace `Span` attributes
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize)]
pub struct Key(Cow<'static, str>);

impl Key {
    /// Create a new `key`
    pub fn new<S: Into<Cow<'static, str>>>(value: S) -> Self {
        Key(value.into())
    }

    /// Returns a reference to the underlying key name
    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }

    /// Create a `KeyValue` pair for `bool` values
    pub fn bool<T: Into<bool>>(self, value: T) -> KeyValue {
        KeyValue {
            key: self,
            value: AnyValue::Boolean(value.into()),
        }
    }

    /// Create a `KeyValue` pair for `i64` values.
    pub fn i64(self, value: i64) -> KeyValue {
        KeyValue {
            key: self,
            value: AnyValue::Int64(value),
        }
    }

    /// Create a `KeyValue` pair for `f64` values.
    pub fn f64(self, value: f64) -> KeyValue {
        KeyValue {
            key: self,
            value: AnyValue::Float(value),
        }
    }

    /// Create a `KeyValue` pair for `String` values
    pub fn string<T: Into<Cow<'static, str>>>(self, value: T) -> KeyValue {
        KeyValue {
            key: self,
            value: AnyValue::String(value.into()),
        }
    }
}

impl From<String> for Key {
    fn from(s: String) -> Self {
        Key(Cow::from(s))
    }
}

impl From<Key> for String {
    fn from(k: Key) -> Self {
        k.0.into_owned()
    }
}

#[derive(Clone, Debug, PartialOrd, PartialEq, Deserialize, Serialize)]
pub enum AnyValue {
    String(Cow<'static, str>),
    Float(f64),
    Boolean(bool),
    Int64(i64),
}

#[derive(Clone, Debug, PartialOrd, PartialEq, Deserialize, Serialize)]
pub struct KeyValue {
    pub key: Key,
    pub value: AnyValue,
}

impl KeyValue {
    /// Create a new `KeyValue` pair.
    pub fn new<K, V>(key: K, value: V) -> Self
    where
        K: Into<Key>,
        V: Into<AnyValue>,
    {
        KeyValue {
            key: key.into(),
            value: value.into(),
        }
    }
}

impl ByteSizeOf for KeyValue {
    fn allocated_bytes(&self) -> usize {
        let key = 0;
        let value = match &self.value {
            AnyValue::String(s) => s.as_bytes().allocated_bytes(),
            _ => 0,
        };

        key + value
    }
}

#[derive(Clone, Debug, PartialOrd, PartialEq, Deserialize, Serialize)]
pub struct Event {
    /// name of the event.
    pub name: String,

    /// `timestamp` is the time the event occurred.
    pub timestamp: i64,

    pub attributes: EvictedHashMap,
}

impl ByteSizeOf for Event {
    fn allocated_bytes(&self) -> usize {
        self.name.allocated_bytes() + self.attributes.allocated_bytes()
    }
}

/// A 16-type value which identifies a given trace.
///
/// The id is valid if it contains at least one non-zero byte.
#[derive(Clone, Copy, Deserialize, Hash, PartialEq, PartialOrd, Serialize)]
pub struct TraceId(pub u128);

impl Debug for TraceId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:032x}", self.0))
    }
}

impl TraceId {
    /// Invalid trace id
    pub const INVALID: TraceId = TraceId(0);

    /// Create a trace id from its representation as a byte array.
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        TraceId(u128::from_be_bytes(bytes))
    }

    /// Return the representation of this trace id as a byte array
    pub const fn to_bytes(self) -> [u8; 16] {
        self.0.to_be_bytes()
    }
}

/// An 8-byte value which identifies a given span.
///
/// The id is valid if it contains at least one non-zero byte.
#[derive(Clone, Copy, Deserialize, Hash, PartialEq, PartialOrd, Serialize)]
pub struct SpanId(pub u64);

impl Debug for SpanId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:016x}", self.0))
    }
}

impl SpanId {
    /// Invalid span id
    pub const INVALID: SpanId = SpanId(0);

    /// Create a span id from its representation as a byte array.
    pub const fn from_bytes(bytes: [u8; 8]) -> Self {
        SpanId(u64::from_be_bytes(bytes))
    }

    /// Return the representation of this span id as a byte array.
    pub const fn to_bytes(self) -> [u8; 8] {
        self.0.to_be_bytes()
    }

    pub fn into_i64(self) -> i64 {
        i64::from_be_bytes(self.to_bytes())
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
}

impl Display for SpanKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SpanKind::Client => write!(f, "client"),
            SpanKind::Server => write!(f, "server"),
            SpanKind::Producer => write!(f, "producer"),
            SpanKind::Consumer => write!(f, "consumer"),
            SpanKind::Internal => write!(f, "internal"),
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
    /// A unique identifier of a trace that this linked span is part of.
    /// The ID is a 16-byte array.
    pub trace_id: TraceId,

    /// A unique identifier for the linked span. The ID is an 8-byte array.
    pub span_id: SpanId,

    /// The trace_state associated with the link.
    pub trace_state: String,

    /// `attributes` is a collection of key/value pairs on the link.
    pub attributes: Vec<KeyValue>,
}

/// For the semantics of status codes see
/// https://github.com/open-telemetry/opentelemetry-specification/blob/main/specification/trace/api.md#set-status
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

#[derive(Clone, Debug, Deserialize, PartialEq, PartialOrd, Serialize)]
pub struct Span {
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
    pub attributes: EvictedHashMap,

    /// `events` is a collection of event items.
    pub events: Vec<Event>,

    /// links is a collection of Links, which are references from this span to a span
    /// in the same or different trace.
    pub links: EvictedQueue<Link>,

    /// An optional final status for this span. Semantically when Status isn't set,
    /// it span's status code is unset, i.e. assume STATUS_CODE_UNSET (code = 0)
    pub status: Status,
}

pub type Spans = Vec<Span>;

impl ByteSizeOf for Span {
    fn allocated_bytes(&self) -> usize {
        self.name.allocated_bytes()
            + self.attributes.allocated_bytes()
            + self.events.allocated_bytes()
    }
}

#[derive(Clone, Debug, PartialOrd, PartialEq, Deserialize, Serialize)]
pub struct Trace {
    pub service: String,

    pub tags: BTreeMap<String, String>,

    pub spans: Vec<Span>,

    #[serde(skip)]
    metadata: EventMetadata,
}

pub type Traces = Vec<Trace>;

impl ByteSizeOf for Trace {
    fn allocated_bytes(&self) -> usize {
        self.service.allocated_bytes() + self.tags.allocated_bytes() + self.spans.allocated_bytes()
    }
}

impl Finalizable for Trace {
    fn take_finalizers(&mut self) -> EventFinalizers {
        self.metadata.take_finalizers()
    }
}

impl Trace {
    #[inline]
    pub fn metadata(&self) -> &EventMetadata {
        &self.metadata
    }

    #[inline]
    pub fn metadata_mut(&mut self) -> &mut EventMetadata {
        &mut self.metadata
    }

    pub fn add_finalizer(&mut self, finalizer: EventFinalizer) {
        self.metadata.add_finalizer(finalizer);
    }

    pub fn with_batch_notifier(mut self, batch: &Arc<BatchNotifier>) -> Self {
        self.metadata = self.metadata.with_batch_notifier(batch);
        self
    }

    pub fn with_batch_notifier_option(mut self, batch: &Option<Arc<BatchNotifier>>) -> Self {
        self.metadata = self.metadata.with_batch_notifier_option(batch);
        self
    }
}
