use std::collections::{BTreeMap, HashMap};

use event::trace::{
    AnyValue, Event, EvictedHashMap, EvictedQueue, Key, KeyValue, Link, SpanContext, SpanId,
    SpanKind, Status, StatusCode, Trace, TraceId, TraceState,
};

use super::proto;
use crate::proto::ValueType;
use crate::thrift::jaeger;
use crate::{Batch, Log, Span, SpanRef, Tag, TagType};

impl From<KeyValue> for Tag {
    fn from(kv: KeyValue) -> Self {
        let KeyValue { key, value } = kv;
        (key, value).into()
    }
}

impl From<(Key, AnyValue)> for Tag {
    fn from((key, value): (Key, AnyValue)) -> Self {
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

impl From<Tag> for KeyValue {
    fn from(tag: Tag) -> Self {
        let key = tag.key.into();
        let value: AnyValue = match tag.v_type {
            TagType::String => tag.v_str.unwrap_or_default().into(),
            TagType::Double => {
                let f = tag.v_double.unwrap_or_default().into();
                AnyValue::Float(f)
            }
            TagType::Bool => tag.v_bool.unwrap_or_default().into(),
            TagType::Long => tag.v_long.unwrap_or_default().into(),
            TagType::Binary => base64::encode(tag.v_binary.unwrap()).into(),
        };

        KeyValue { key, value }
    }
}

impl From<Span> for event::trace::Span {
    fn from(js: Span) -> Self {
        let trace_id = to_trace_id(js.trace_id_high, js.trace_id_low);
        let end_time = js.start_time + js.duration;
        let mut attributes = tags_to_attributes(js.tags);

        let status = status_from_attributes(&mut attributes);

        let kind = if let Some(value) = attributes.remove("span.kind") {
            let value = value.to_string();
            match value.as_str() {
                "client" => SpanKind::Client,
                "server" => SpanKind::Server,
                "producer" => SpanKind::Producer,
                "consumer" => SpanKind::Consumer,
                "internal" => SpanKind::Internal,
                _ => SpanKind::Unspecified,
            }
        } else {
            SpanKind::Unspecified
        };

        Self {
            span_context: SpanContext {
                trace_id,
                span_id: js.span_id.into(),
                trace_flags: Default::default(),
                is_remote: false,
                trace_state: Default::default(),
            },
            parent_span_id: js.parent_span_id.into(),
            name: js.operation_name,
            kind,
            start_time: js.start_time * 1000,
            end_time: end_time * 1000,
            attributes,
            events: jaeger_logs_to_internal_event(js.logs),
            links: references_to_links(js.references).into(),
            status,
        }
    }
}

impl From<Event> for Log {
    fn from(event: Event) -> Self {
        let timestamp = event.timestamp / 1000;

        let mut fields = event
            .attributes
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>();

        fields.push((Key::from("message"), AnyValue::from(event.name.to_string())).into());

        Log { timestamp, fields }
    }
}

impl From<Log> for Event {
    fn from(log: Log) -> Self {
        let timestamp = log.timestamp * 1000;
        let mut attributes = tags_to_attributes(Some(log.fields));
        let name = if let Some(value) = attributes.remove("message") {
            value.to_string().into()
        } else {
            "".into()
        };

        Event {
            name,
            timestamp,
            attributes,
        }
    }
}

impl From<event::trace::Span> for Span {
    fn from(span: event::trace::Span) -> Self {
        let trace_id_bytes = span.trace_id().unwrap().to_bytes();
        let (high, low) = trace_id_bytes.split_at(8);
        let trace_id_high = i64::from_be_bytes(high.try_into().unwrap());
        let trace_id_low = i64::from_be_bytes(low.try_into().unwrap());

        Span {
            trace_id_low,
            trace_id_high,
            span_id: span.span_id().into_i64(),
            parent_span_id: span.parent_span_id.into_i64(),
            operation_name: span.name,
            references: links_to_references(span.links),
            flags: 0,
            // nanosecond to microsecond
            start_time: span.start_time / 1000,
            duration: (span.end_time - span.start_time) / 1000,
            tags: Some(build_span_tags(
                span.attributes,
                span.status.status_code,
                span.status.message.into_owned(),
                span.kind,
            )),
            logs: events_to_logs(span.events),
        }
    }
}

const ERROR: &str = "error";
const SPAN_KIND: &str = "span.kind";
const OTEL_STATUS_CODE: &str = "otel.status_code";
const OTEL_STATUS_DESCRIPTION: &str = "otel.status_description";

#[derive(Default)]
struct UserOverrides {
    error: bool,
    span_kind: bool,
    status_code: bool,
    status_description: bool,
}

impl UserOverrides {
    fn record_attr(&mut self, attr: &str) {
        match attr {
            ERROR => self.error = true,
            SPAN_KIND => self.span_kind = true,
            OTEL_STATUS_CODE => self.status_code = true,
            OTEL_STATUS_DESCRIPTION => self.status_description = true,
            _ => (),
        }
    }
}

fn build_span_tags(
    attrs: EvictedHashMap,
    status_code: StatusCode,
    status_description: String,
    kind: SpanKind,
) -> Vec<jaeger::Tag> {
    let mut user_overrides = UserOverrides::default();
    // TODO: determine if namespacing is required to avoid collision with set attributes
    let mut tags = attrs
        .into_iter()
        .map(|(k, v)| {
            user_overrides.record_attr(k.as_str());
            KeyValue::new(k, v).into()
        })
        .collect::<Vec<_>>();

    if !user_overrides.span_kind && kind != SpanKind::Internal {
        tags.push(Key::new(SPAN_KIND).string(kind.to_string()).into())
    }

    if status_code != StatusCode::Unset {
        // Ensure error status is set unless user has already overrided it
        if status_code == StatusCode::Error {
            tags.push(Key::new(ERROR).bool(true).into());
        }

        if !user_overrides.status_code {
            tags.push(
                Key::new(OTEL_STATUS_CODE)
                    .string::<&'static str>(status_code.as_str())
                    .into(),
            );
        }

        // set status message if there is one
        if !status_description.is_empty() && !user_overrides.status_description {
            tags.push(
                Key::new(OTEL_STATUS_DESCRIPTION)
                    .string(status_description)
                    .into(),
            );
        }
    }

    tags
}

fn events_to_logs(events: EvictedQueue<Event>) -> Option<Vec<jaeger::Log>> {
    if events.is_empty() {
        None
    } else {
        Some(events.into_iter().map(Into::into).collect())
    }
}

fn links_to_references(links: EvictedQueue<Link>) -> Option<Vec<jaeger::SpanRef>> {
    if links.is_empty() {
        return None;
    }

    let refs = links
        .iter()
        .map(|link| {
            let trace_id_bytes = link.trace_id.to_bytes();
            let (high, low) = trace_id_bytes.split_at(8);
            let trace_id_high = i64::from_be_bytes(high.try_into().unwrap());
            let trace_id_low = i64::from_be_bytes(low.try_into().unwrap());

            jaeger::SpanRef::new(
                jaeger::SpanRefType::FollowsFrom,
                trace_id_low,
                trace_id_high,
                link.span_id.into_i64(),
            )
        })
        .collect();

    Some(refs)
}

fn to_trace_id(high: i64, low: i64) -> TraceId {
    let mut buf = [0u8; 16];
    let high: [u8; 8] = high.to_be_bytes();
    let low: [u8; 8] = low.to_be_bytes();

    buf[..8].clone_from_slice(&high);
    buf[8..].clone_from_slice(&low);

    TraceId::from_bytes(buf)
}

fn tags_to_attributes(tags: Option<Vec<Tag>>) -> EvictedHashMap {
    if let Some(tags) = tags {
        tags.into_iter()
            .map(|tag| {
                let kv: KeyValue = tag.into();
                (kv.key, kv.value)
            })
            .collect::<HashMap<Key, AnyValue>>()
            .into()
    } else {
        EvictedHashMap::new(128, 0)
    }
}

fn references_to_links(refs: Option<Vec<SpanRef>>) -> Vec<Link> {
    if let Some(refs) = refs {
        refs.into_iter()
            .map(|span_ref| {
                let trace_id = to_trace_id(span_ref.trace_id_high, span_ref.trace_id_low);

                Link {
                    trace_id,
                    span_id: span_ref.span_id.into(),
                    trace_state: "".to_string(),
                    attributes: vec![],
                }
            })
            .collect()
    } else {
        vec![]
    }
}

fn jaeger_logs_to_internal_event(logs: Option<Vec<Log>>) -> EvictedQueue<Event> {
    if let Some(logs) = logs {
        logs.into_iter()
            .map(Into::into)
            .fold(EvictedQueue::default(), |mut queue, event| {
                queue.push_back(event);
                queue
            })
    } else {
        EvictedQueue::default()
    }
}

fn internal_trace_id_to_jaeger_trace_id(trace_id: TraceId) -> (i64, i64) {
    let bytes = trace_id.to_bytes();
    let (high, low) = bytes.split_at(8);
    let high = i64::from_be_bytes(high.try_into().unwrap());
    let low = i64::from_be_bytes(low.try_into().unwrap());

    (high, low)
}

impl Tag {
    pub fn str_value(self) -> String {
        match self.v_type {
            TagType::String => self.v_str.unwrap_or_default(),
            TagType::Double => self.v_double.unwrap_or_default().to_string(),
            TagType::Bool => {
                if self.v_bool.unwrap_or(false) {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            TagType::Long => self.v_long.unwrap_or_default().to_string(),
            TagType::Binary => {
                let value = self.v_binary.unwrap_or_default();
                base64::encode(value)
            }
        }
    }
}

impl From<Batch> for Trace {
    fn from(batch: Batch) -> Self {
        let service = batch.process.service_name;

        let tags = batch
            .process
            .tags
            .unwrap_or_default()
            .into_iter()
            .map(|tag| {
                let value = match tag.v_type {
                    TagType::String => tag.v_str.unwrap_or_default(),
                    TagType::Double => tag.v_double.unwrap_or_default().to_string(),
                    TagType::Bool => {
                        if tag.v_bool.unwrap_or(false) {
                            "true".to_string()
                        } else {
                            "false".to_string()
                        }
                    }
                    TagType::Long => tag.v_long.unwrap_or_default().to_string(),
                    TagType::Binary => {
                        let value = tag.v_binary.unwrap_or_default();
                        base64::encode(value)
                    }
                };

                (tag.key, value)
            })
            .collect::<BTreeMap<String, String>>();

        Trace::new(
            service,
            tags,
            batch.spans.into_iter().map(Into::into).collect(),
        )
    }
}

impl From<Batch> for event::Event {
    fn from(batch: Batch) -> Self {
        event::Event::Trace(batch.into())
    }
}

// From proto to internal

impl From<proto::Batch> for event::Event {
    fn from(batch: proto::Batch) -> Self {
        let trace: Trace = batch.into();
        trace.into()
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
            base64::encode(kv.v_binary).into()
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

fn prost_duration_to_nano_seconds(duration: Option<prost_types::Duration>) -> i64 {
    duration
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
            attributes,
            events: span.logs.into_iter().map(Into::into).collect(),
            links: Default::default(),
            status: Default::default(),
        }
    }
}

fn status_from_attributes(attributes: &mut EvictedHashMap) -> Status {
    if let Some(value) = attributes.remove("error") {
        let msg = if matches!(value, AnyValue::Boolean(_)) {
            String::new()
        } else {
            value.to_string()
        };

        Status {
            message: msg.into(),
            status_code: StatusCode::Error,
        }
    } else {
        Status {
            message: "".into(),
            status_code: StatusCode::Ok,
        }
    }
}

impl From<proto::Batch> for Trace {
    fn from(batch: crate::proto::Batch) -> Self {
        let (service, tags) = match batch.process {
            Some(process) => {
                let tags = process
                    .tags
                    .into_iter()
                    .map(|kv| {
                        let value = if kv.v_type == ValueType::String as i32 {
                            kv.v_str
                        } else if kv.v_type == ValueType::Bool as i32 {
                            if kv.v_bool {
                                "true".to_string()
                            } else {
                                "false".to_string()
                            }
                        } else if kv.v_type == ValueType::Int64 as i32 {
                            kv.v_int64.to_string()
                        } else if kv.v_type == ValueType::Float64 as i32 {
                            kv.v_float64.to_string()
                        } else {
                            base64::encode(kv.v_binary)
                        };

                        (kv.key, value)
                    })
                    .collect::<BTreeMap<String, String>>();

                (process.service_name, tags)
            }
            None => (String::new(), BTreeMap::new()),
        };

        let spans = batch.spans.into_iter().map(Into::into).collect();

        Trace::new(service, tags, spans)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_id_converts() {
        let inputs = [123u128, u64::MAX as u128 + u32::MAX as u128];

        for want in inputs {
            let trace_id = TraceId(want);
            let (high, low) = internal_trace_id_to_jaeger_trace_id(trace_id);
            let got = to_trace_id(high, low);

            assert_eq!(got.0, want);
        }
    }
}
