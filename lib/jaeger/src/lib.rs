pub mod agent;
mod thrift;
mod transport;

pub use crate::thrift::jaeger::{Batch, Log, Process, Span, SpanRef, SpanRefType, Tag, TagType};
use event::trace::{AnyValue, KeyValue};

impl From<KeyValue> for Tag {
    fn from(kv: KeyValue) -> Self {
        let KeyValue { key, value } = kv;

        match value {
            AnyValue::String(s) => Tag::new(
                key.into(),
                TagType::String,
                Some(s.into()),
                None,
                None,
                None,
                None,
            ),
            AnyValue::Float(f) => Tag::new(
                key.into(),
                TagType::Double,
                None,
                Some(f.into()),
                None,
                None,
                None,
            ),
            AnyValue::Boolean(b) => {
                Tag::new(key.into(), TagType::Bool, None, None, Some(b), None, None)
            }
            AnyValue::Int64(i) => {
                Tag::new(key.into(), TagType::Long, None, None, None, Some(i), None)
            }
            // TODO: better Array handling, jaeger thrift doesn't support arrays
            // v @ Value::Array(_) => Tag::new(key.into(), TagType::String, Some(v.to_string()), None, None, None, None),
        }
    }
}
