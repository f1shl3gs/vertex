use crate::thrift::jaeger;
use crate::Batch;
use event::trace::{
    AnyValue, Event, EvictedHashMap, EvictedQueue, Key, KeyValue, Link, SpanKind, StatusCode,
};
use event::Trace;

use crate::thrift::jaeger::{Log, Span, SpanRef, SpanRefType, Tag, TagType};
use crate::translate::id::internal_trace_id_to_jaeger_trace_id;

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

impl From<event::trace::Span> for Span {
    fn from(span: event::trace::Span) -> Self {
        let (trace_id_high, trace_id_low) =
            internal_trace_id_to_jaeger_trace_id(span.span_context.trace_id);

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
                span.tags,
                span.status.status_code,
                span.status.message.into_owned(),
                span.kind,
            )),
            logs: events_to_logs(span.events),
        }
    }
}

fn links_to_references(links: EvictedQueue<Link>) -> Option<Vec<SpanRef>> {
    if links.is_empty() {
        return None;
    }

    let refs = links
        .iter()
        .map(|link| {
            let trace_id_bytes = link.trace_id().to_bytes();
            let (high, low) = trace_id_bytes.split_at(8);
            let trace_id_high = i64::from_be_bytes(high.try_into().unwrap());
            let trace_id_low = i64::from_be_bytes(low.try_into().unwrap());

            SpanRef::new(
                SpanRefType::FollowsFrom,
                trace_id_low,
                trace_id_high,
                link.span_id().into_i64(),
            )
        })
        .collect();

    Some(refs)
}

fn events_to_logs(events: EvictedQueue<Event>) -> Option<Vec<Log>> {
    if events.is_empty() {
        None
    } else {
        Some(events.into_iter().map(Into::into).collect())
    }
}

#[derive(Default)]
struct UserOverrides {
    error: bool,
    span_kind: bool,
    status_code: bool,
    status_description: bool,
}

const ERROR: &str = "error";
const SPAN_KIND: &str = "span.kind";
const OTEL_STATUS_CODE: &str = "otel.status_code";
const OTEL_STATUS_DESCRIPTION: &str = "otel.status_description";

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
) -> Vec<Tag> {
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

impl From<Trace> for Batch {
    fn from(trace: Trace) -> Self {
        let tags = trace
            .tags
            .into_iter()
            .map(|(k, v)| {
                jaeger::Tag::new(
                    k.to_string(),
                    jaeger::TagType::String,
                    Some(v.to_string()),
                    None,
                    None,
                    None,
                    None,
                )
            })
            .collect();
        let process = jaeger::Process::new(trace.service.to_string(), Some(tags));
        let spans = trace.spans.into_iter().map(Into::into).collect();

        jaeger::Batch::new(process, spans)
    }
}
