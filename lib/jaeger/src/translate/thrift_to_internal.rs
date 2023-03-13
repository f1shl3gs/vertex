use base64::Engine;
use std::collections::HashMap;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use event::trace::{
    AnyValue, Event, EvictedHashMap, EvictedQueue, Key, KeyValue, Link, SpanContext, SpanKind,
    Status, StatusCode,
};
use event::{tags::Value, Trace};

use crate::thrift::jaeger::{Batch, Log, Span, SpanRef, Tag, TagType};
use crate::translate::id::to_trace_id;

impl From<Batch> for event::Event {
    fn from(batch: Batch) -> Self {
        event::Event::Trace(batch.into())
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
                    TagType::String => Value::from(tag.v_str.unwrap_or_default()),
                    TagType::Double => Value::from(tag.v_double.unwrap_or_default().0),
                    TagType::Bool => Value::from(tag.v_bool.unwrap_or_default()),
                    TagType::Long => Value::from(tag.v_long.unwrap_or_default()),
                    TagType::Binary => {
                        let value = tag.v_binary.unwrap_or_default();
                        Value::from(BASE64_STANDARD.encode(value))
                    }
                };

                (event::tags::Key::from(tag.key), value)
            })
            .collect();

        Trace::new(
            service,
            tags,
            batch.spans.into_iter().map(Into::into).collect(),
        )
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
            tags: attributes,
            events: jaeger_logs_to_internal_event(js.logs),
            links: references_to_links(js.references).into(),
            status,
        }
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
            TagType::Binary => BASE64_STANDARD.encode(tag.v_binary.unwrap()).into(),
        };

        KeyValue { key, value }
    }
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

fn references_to_links(refs: Option<Vec<SpanRef>>) -> Vec<Link> {
    if let Some(refs) = refs {
        refs.into_iter()
            .map(|span_ref| {
                let trace_id = to_trace_id(span_ref.trace_id_high, span_ref.trace_id_low);

                Link {
                    span_context: SpanContext {
                        trace_id,
                        span_id: span_ref.span_id.into(),
                        trace_flags: Default::default(),
                        is_remote: false,
                        trace_state: Default::default(),
                    },
                    attributes: vec![],
                }
            })
            .collect()
    } else {
        vec![]
    }
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
                BASE64_STANDARD.encode(value)
            }
        }
    }
}
