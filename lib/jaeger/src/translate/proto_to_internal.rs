use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use event::tags::{Tags, Value};
use event::{
    trace::{AnyValue, EvictedHashMap, Key, SpanContext, SpanId, SpanKind, TraceId, TraceState},
    Events, Trace,
};

use crate::proto::{self, Batch, ValueType};

impl From<Batch> for Events {
    fn from(batch: Batch) -> Self {
        Events::Traces(vec![batch.into()])
    }
}

/// Translate `proto::Span` into internal `event::trace::Span`
impl From<proto::Span> for event::trace::Span {
    fn from(span: proto::Span) -> Self {
        let mut trace_id_bytes = [0u8; 16];
        trace_id_bytes.clone_from_slice(span.trace_id.as_slice());
        let mut span_id_bytes = [0u8; 8];
        span_id_bytes.clone_from_slice(span.trace_id.as_slice());

        let trace_id = TraceId::from_bytes(trace_id_bytes);
        let span_id = SpanId::from_bytes(span_id_bytes);
        let parent_span_id = span.parent_span_id();
        let name = span.operation_name;
        let mut attributes = span.tags.into();
        let start_time = prost_timestamp_to_nano_seconds(span.start_time);
        let end_time = start_time + prost_duration_to_nano_seconds(span.duration);
        let trace_state = trace_state_from_attributes(&mut attributes);
        let kind = span_kind_from_attributes(&mut attributes);

        event::trace::Span {
            span_context: SpanContext {
                trace_id,
                span_id,
                trace_flags: Default::default(),
                is_remote: false,
                trace_state,
            },
            parent_span_id,
            name,
            kind,
            start_time,
            end_time,
            tags: attributes,
            events: span.logs.into_iter().map(Into::into).collect(),
            links: Default::default(),
            status: Default::default(),
        }
    }
}

impl From<Batch> for Trace {
    fn from(batch: Batch) -> Self {
        let (service, tags) = match batch.process {
            Some(process) => {
                let attrs = process
                    .tags
                    .into_iter()
                    .map(|kv| {
                        let value = if kv.v_type == ValueType::String as i32 {
                            Value::from(kv.v_str)
                        } else if kv.v_type == ValueType::Bool as i32 {
                            Value::from(kv.v_bool)
                        } else if kv.v_type == ValueType::Int64 as i32 {
                            Value::from(kv.v_int64)
                        } else if kv.v_type == ValueType::Float64 as i32 {
                            Value::from(kv.v_float64)
                        } else {
                            Value::from(BASE64_STANDARD.encode(kv.v_binary))
                        };

                        (Key::new(kv.key), value)
                    })
                    .collect();

                (process.service_name, attrs)
            }
            None => (String::new(), Tags::default()),
        };

        let spans = batch.spans.into_iter().map(Into::into).collect();

        Trace::new(service, tags, spans)
    }
}

impl From<proto::KeyValue> for (Key, AnyValue) {
    fn from(kv: proto::KeyValue) -> Self {
        let value = if kv.v_type == ValueType::String as i32 {
            kv.v_str.into()
        } else if kv.v_type == ValueType::Bool as i32 {
            kv.v_bool.into()
        } else if kv.v_type == ValueType::Int64 as i32 {
            kv.v_int64.into()
        } else if kv.v_type == ValueType::Float64 as i32 {
            kv.v_float64.into()
        } else {
            BASE64_STANDARD.encode(kv.v_binary).into()
        };

        (kv.key.into(), value)
    }
}

impl From<proto::Log> for event::trace::Event {
    fn from(log: proto::Log) -> Self {
        let timestamp = log.timestamp.unwrap();
        let mut attributes: EvictedHashMap = log.fields.into();

        let name = if let Some(value) = attributes.remove("message") {
            value.to_string()
        } else {
            String::new()
        };

        Self {
            name: name.into(),
            timestamp: timestamp.seconds * 1000 * 1000 * 1000 + timestamp.nanos as i64,
            attributes,
        }
    }
}

fn prost_timestamp_to_nano_seconds(timestamp: Option<prost_types::Timestamp>) -> i64 {
    timestamp
        .map(|ts| ts.seconds * 1000 * 1000 * 1000 + ts.nanos as i64)
        .unwrap_or(0)
}

const W3C_TRACESTATE: &str = "w3c.tracestate";

fn trace_state_from_attributes(attributes: &mut EvictedHashMap) -> TraceState {
    if let Some(value) = attributes.remove(W3C_TRACESTATE) {
        let value = value.to_string();
        // TODO: log errors
        value.parse().unwrap_or_default()
    } else {
        TraceState::default()
    }
}

#[inline]
fn prost_duration_to_nano_seconds(duration: Option<prost_types::Duration>) -> i64 {
    duration
        .map(|ts| ts.seconds * 1000 * 1000 * 1000 + ts.nanos as i64)
        .unwrap_or(0)
}

const SPAN_KIND: &str = "span.kind";

fn span_kind_from_attributes(attributes: &mut EvictedHashMap) -> SpanKind {
    if let Some(value) = attributes.remove(SPAN_KIND) {
        match value {
            AnyValue::String(s) => match s.as_ref() {
                "client" => SpanKind::Client,
                "server" => SpanKind::Server,
                "producer" => SpanKind::Producer,
                "consumer" => SpanKind::Consumer,
                "internal" => SpanKind::Internal,
                _ => SpanKind::Unspecified,
            },
            _ => SpanKind::Unspecified,
        }
    } else {
        SpanKind::Unspecified
    }
}
