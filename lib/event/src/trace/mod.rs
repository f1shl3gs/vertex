// mod context;
mod evicted_hash_map;
mod evicted_queue;
pub mod generator;
mod span;
// mod tracer;

use std::borrow::Cow;
use std::collections::VecDeque;
use std::fmt::{self, Debug};
use std::num::ParseIntError;
use std::ops::{BitAnd, BitOr, Not};
use std::str::FromStr;

use measurable::ByteSizeOf;
use serde::{Deserialize, Serialize};

use crate::tags::Tags;
use crate::{
    BatchNotifier, EventDataEq, EventFinalizer, EventFinalizers, EventMetadata, Finalizable,
};
pub use evicted_hash_map::EvictedHashMap;
pub use evicted_queue::EvictedQueue;
pub use generator::RngGenerator;
pub use span::*;

/// Key used for metric `AttributeSet`s and trace `Span` attributes
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
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

impl From<&'static str> for Key {
    /// Convert a `&str` to a `Key`.
    fn from(key_str: &'static str) -> Self {
        Key(Cow::from(key_str))
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

#[derive(Clone, Debug, PartialOrd, PartialEq, Serialize)]
pub enum AnyValue {
    String(Cow<'static, str>),
    Float(f64),
    Boolean(bool),
    Int64(i64),
}

impl From<i64> for AnyValue {
    fn from(i: i64) -> Self {
        Self::Int64(i)
    }
}

impl From<u32> for AnyValue {
    fn from(u: u32) -> Self {
        Self::Int64(u as i64)
    }
}

impl From<f64> for AnyValue {
    fn from(f: f64) -> Self {
        Self::Float(f)
    }
}

impl From<bool> for AnyValue {
    fn from(b: bool) -> Self {
        Self::Boolean(b)
    }
}

impl From<&str> for AnyValue {
    fn from(s: &str) -> Self {
        Self::String(s.to_string().into())
    }
}

impl From<String> for AnyValue {
    fn from(s: String) -> Self {
        Self::String(s.into())
    }
}

impl ToString for AnyValue {
    fn to_string(&self) -> String {
        match self {
            AnyValue::String(s) => s.to_string(),
            AnyValue::Int64(i) => i.to_string(),
            AnyValue::Float(f) => f.to_string(),
            AnyValue::Boolean(b) => b.to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialOrd, PartialEq, Serialize)]
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

/// Error returned by `TraceState` operations.
#[derive(Debug)]
#[non_exhaustive]
pub enum TraceStateError {
    /// The key is invalid. See <https://www.w3.org/TR/trace-context/#key> for requirement for keys.
    // #[error("{0} is not a valid key in TraceState, see https://www.w3.org/TR/trace-context/#key for more details")]
    InvalidKey(String),

    /// The value is invalid. See <https://www.w3.org/TR/trace-context/#value> for requirement for values.
    // #[error("{0} is not a valid value in TraceState, see https://www.w3.org/TR/trace-context/#value for more details")]
    InvalidValue(String),

    /// The value is invalid. See <https://www.w3.org/TR/trace-context/#list> for requirement for list members.
    // #[error("{0} is not a valid list member in TraceState, see https://www.w3.org/TR/trace-context/#list for more details")]
    InvalidList(String),
}

/// `TraceState` carries system-specific configuration data, represented as a list
/// of key-value pairs. `TraceState` allows multiple tracing systems to
/// participate in the same trace.
///
/// Please review the [W3C specification] for details on this field.
///
/// [W3C specification]: https://www.w3.org/TR/trace-context/#tracestate-header
#[derive(Clone, Debug, Default, Eq, PartialEq, PartialOrd, Hash, Deserialize, Serialize)]
pub struct TraceState(Option<VecDeque<(String, String)>>);

impl TraceState {
    /// Validates that the given `TraceState` list-member key is valid per the [W3 Spec].
    ///
    /// [W3 Spec]: https://www.w3.org/TR/trace-context/#key
    fn valid_key(key: &str) -> bool {
        if key.len() > 256 {
            return false;
        }

        let allowed_special = |b: u8| (b == b'_' || b == b'-' || b == b'*' || b == b'/');
        let mut vendor_start = None;
        for (i, &b) in key.as_bytes().iter().enumerate() {
            if !(b.is_ascii_lowercase() || b.is_ascii_digit() || allowed_special(b) || b == b'@') {
                return false;
            }

            if i == 0 && (!b.is_ascii_lowercase() && !b.is_ascii_digit()) {
                return false;
            } else if b == b'@' {
                if vendor_start.is_some() || i + 14 < key.len() {
                    return false;
                }
                vendor_start = Some(i);
            } else if let Some(start) = vendor_start {
                if i == start + 1 && !(b.is_ascii_lowercase() || b.is_ascii_digit()) {
                    return false;
                }
            }
        }

        true
    }

    /// Validates that the given `TraceState` list-member value is valid per the [W3 Spec].
    ///
    /// [W3 Spec]: https://www.w3.org/TR/trace-context/#value
    fn valid_value(value: &str) -> bool {
        if value.len() > 256 {
            return false;
        }

        !(value.contains(',') || value.contains('='))
    }

    /// Creates a new `TraceState` from the given key-value collection.
    ///
    /// # Errors
    ///
    /// This function returns error if the "key" or "value" is not valid.
    pub fn from_key_value<T, K, V>(trace_state: T) -> Result<Self, TraceStateError>
    where
        T: IntoIterator<Item = (K, V)>,
        K: ToString,
        V: ToString,
    {
        let ordered_data = trace_state
            .into_iter()
            .map(|(key, value)| {
                let (key, value) = (key.to_string(), value.to_string());
                if !TraceState::valid_key(key.as_str()) {
                    return Err(TraceStateError::InvalidKey(key));
                }
                if !TraceState::valid_value(value.as_str()) {
                    return Err(TraceStateError::InvalidValue(value));
                }

                Ok((key, value))
            })
            .collect::<Result<VecDeque<_>, TraceStateError>>()?;

        if ordered_data.is_empty() {
            Ok(TraceState(None))
        } else {
            Ok(TraceState(Some(ordered_data)))
        }
    }

    /// Retrieves a value for a given key from the `TraceState` if it exists.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.as_ref().and_then(|kvs| {
            kvs.iter().find_map(|item| {
                if item.0.as_str() == key {
                    Some(item.1.as_str())
                } else {
                    None
                }
            })
        })
    }

    /// Inserts the given key-value pair into the `TraceState`. If a value already exists for the
    /// given key, this updates the value and updates the value's position. If the key or value are
    /// invalid per the [W3 Spec] an `Err` is returned, else a new `TraceState` with the
    /// updated key/value is returned.
    ///
    /// # Errors
    ///
    /// This function returns error if the "key" or "value" is not valid.
    /// [W3 Spec]: <https://www.w3.org/TR/trace-context/#mutating-the-tracestate-field>
    pub fn insert<K, V>(&self, key: K, value: V) -> Result<TraceState, TraceStateError>
    where
        K: Into<String>,
        V: Into<String>,
    {
        let (key, value) = (key.into(), value.into());
        if !TraceState::valid_key(key.as_str()) {
            return Err(TraceStateError::InvalidKey(key));
        }
        if !TraceState::valid_value(value.as_str()) {
            return Err(TraceStateError::InvalidValue(value));
        }

        let mut trace_state = self.delete_from_deque(&key);
        let kvs = trace_state.0.get_or_insert(VecDeque::with_capacity(1));

        kvs.push_front((key, value));

        Ok(trace_state)
    }

    /// Removes the given key-value pair from the `TraceState`. If the key is invalid per the
    /// [W3 Spec] an `Err` is returned. Else, a new `TraceState`
    /// with the removed entry is returned.
    ///
    /// If the key is not in `TraceState`. The original `TraceState` will be cloned and returned.
    ///
    /// # Errors
    ///
    /// This function returns error if the "key" or "value" is not valid.
    /// [W3 Spec]: <https://www.w3.org/TR/trace-context/#mutating-the-tracestate-field>
    pub fn delete<K: Into<String>>(&self, key: K) -> Result<TraceState, TraceStateError> {
        let key = key.into();
        if !TraceState::valid_key(key.as_str()) {
            return Err(TraceStateError::InvalidKey(key));
        }

        Ok(self.delete_from_deque(&key))
    }

    /// Delete key from trace state's deque. The key MUST be valid
    fn delete_from_deque(&self, key: &str) -> TraceState {
        let mut owned = self.clone();
        if let Some(kvs) = owned.0.as_mut() {
            if let Some(index) = kvs.iter().position(|x| *x.0 == *key) {
                kvs.remove(index);
            }
        }
        owned
    }

    /// Creates a new `TraceState` header string, delimiting each key and value with a `=` and each
    /// entry with a `,`.
    pub fn header(&self) -> String {
        self.header_delimited("=", ",")
    }

    /// Creates a new `TraceState` header string, with the given key/value delimiter and entry delimiter.
    pub fn header_delimited(&self, entry_delimiter: &str, list_delimiter: &str) -> String {
        self.0
            .as_ref()
            .map(|kvs| {
                kvs.iter()
                    .map(|(key, value)| format!("{}{}{}", key, entry_delimiter, value))
                    .collect::<Vec<String>>()
                    .join(list_delimiter)
            })
            .unwrap_or_default()
    }
}

impl FromStr for TraceState {
    type Err = TraceStateError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let list_members: Vec<&str> = s.split_terminator(',').collect();
        let mut key_value_pairs: Vec<(String, String)> = Vec::with_capacity(list_members.len());

        for list_member in list_members {
            match list_member.find('=') {
                None => return Err(TraceStateError::InvalidList(list_member.to_string())),
                Some(separator_index) => {
                    let (key, value) = list_member.split_at(separator_index);
                    key_value_pairs
                        .push((key.to_string(), value.trim_start_matches('=').to_string()));
                }
            }
        }

        TraceState::from_key_value(key_value_pairs)
    }
}

/// A 16-type value which identifies a given trace.
///
/// The id is valid if it contains at least one non-zero byte.
#[derive(Clone, Copy, Hash, PartialEq, PartialOrd, Serialize, Eq)]
pub struct TraceId(pub u128);

impl Debug for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:032x}", self.0))
    }
}

impl From<[u8; 16]> for TraceId {
    fn from(b: [u8; 16]) -> Self {
        TraceId::from_bytes(b)
    }
}

impl fmt::LowerHex for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

impl TraceId {
    /// Invalid trace id
    pub const INVALID: TraceId = TraceId(0);

    /// Create a trace id from its representation as a byte array.
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        TraceId(u128::from_be_bytes(bytes))
    }

    /// Converts a string in base 15 to a trace id.
    ///
    /// # Errors
    ///
    /// `ParseIntError` will returned if hex is not a valid hex string.
    #[inline]
    pub fn from_hex(hex: &str) -> Result<Self, ParseIntError> {
        u128::from_str_radix(hex, 16).map(TraceId)
    }

    /// Return the representation of this trace id as a byte array
    pub const fn to_bytes(self) -> [u8; 16] {
        self.0.to_be_bytes()
    }
}

/// An 8-byte value which identifies a given span.
///
/// The id is valid if it contains at least one non-zero byte.
#[derive(Clone, Copy, Deserialize, Hash, PartialEq, PartialOrd, Serialize, Eq)]
pub struct SpanId(pub u64);

impl Debug for SpanId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:016x}", self.0))
    }
}

impl fmt::LowerHex for SpanId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

impl From<u64> for SpanId {
    fn from(u: u64) -> Self {
        Self(u)
    }
}

impl From<i64> for SpanId {
    fn from(i: i64) -> Self {
        SpanId::from_bytes(i64::to_be_bytes(i))
    }
}

impl SpanId {
    /// Invalid span id
    pub const INVALID: SpanId = SpanId(0);

    /// Create a span id from its representation as a byte array.
    pub const fn from_bytes(bytes: [u8; 8]) -> Self {
        SpanId(u64::from_be_bytes(bytes))
    }

    /// Converts a string in base 16 to a span id.
    ///
    /// # Errors
    ///
    /// `ParseIntError` will returned if hex is not a valid hex string.
    pub fn from_hex(hex: &str) -> Result<Self, ParseIntError> {
        u64::from_str_radix(hex, 16).map(SpanId)
    }

    /// Return the representation of this span id as a byte array.
    pub const fn to_bytes(self) -> [u8; 8] {
        self.0.to_be_bytes()
    }

    /// Return true is the `span_id` is valid.
    #[inline]
    pub fn valid(&self) -> bool {
        *self != SpanId::INVALID
    }

    pub fn into_i64(self) -> i64 {
        i64::from_be_bytes(self.to_bytes())
    }
}

#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Eq, Copy, Hash, Deserialize, Serialize)]
pub struct TraceFlags(u8);

impl TraceFlags {
    /// Trace flags with the `sampled` flag set to `1`.
    ///
    /// Spans that are not sampled will be ignored by most tracing tools.
    /// See the `sampled` section of the
    /// [W3C `TraceContext` specification](https://www.w3.org/TR/trace-context/#sampled-flag)
    /// for details.
    pub const SAMPLED: TraceFlags = TraceFlags(0x01);

    /// Construct new trace flags
    pub const fn new(flag: u8) -> Self {
        TraceFlags(flag)
    }

    /// Returns `true` if the `sampled` flag is set
    pub fn is_sampled(&self) -> bool {
        (*self & TraceFlags::SAMPLED) == TraceFlags::SAMPLED
    }

    /// Returns copy  of the current flags with the `sampled` flag set.
    #[must_use]
    pub fn with_sampled(&self, sampled: bool) -> Self {
        if sampled {
            *self | TraceFlags::SAMPLED
        } else {
            *self & !TraceFlags::SAMPLED
        }
    }

    /// Returns the flags as a `u8`
    pub fn to_u8(self) -> u8 {
        self.0
    }
}

impl BitAnd for TraceFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitOr for TraceFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl Not for TraceFlags {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl fmt::LowerHex for TraceFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize)]
pub struct Trace {
    pub service: Cow<'static, str>,

    pub tags: Tags,

    pub spans: Vec<Span>,

    #[serde(skip)]
    metadata: EventMetadata,
}

pub type Traces = Vec<Trace>;

impl ByteSizeOf for Trace {
    fn allocated_bytes(&self) -> usize {
        self.tags.allocated_bytes() + self.spans.allocated_bytes()
    }
}

impl Finalizable for Trace {
    fn take_finalizers(&mut self) -> EventFinalizers {
        self.metadata.take_finalizers()
    }
}

impl EventDataEq for Trace {
    fn event_data_eq(&self, other: &Self) -> bool {
        self.service == other.service && self.tags == other.tags && self.spans == other.spans
    }
}

impl Trace {
    pub fn new(service: impl Into<Cow<'static, str>>, tags: Tags, spans: Vec<Span>) -> Trace {
        Self {
            service: service.into(),
            tags,
            spans,
            metadata: EventMetadata::default(),
        }
    }

    pub fn insert_tag(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.tags.insert(key.into(), value.into());
    }

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

    #[must_use]
    pub fn with_batch_notifier(mut self, batch: &BatchNotifier) -> Self {
        self.metadata = self.metadata.with_batch_notifier(batch);
        self
    }

    #[must_use]
    pub fn with_batch_notifier_option(mut self, batch: &Option<BatchNotifier>) -> Self {
        self.metadata = self.metadata.with_batch_notifier_option(batch);
        self
    }
}
