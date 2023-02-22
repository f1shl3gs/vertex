#[allow(warnings, clippy::all, clippy::pedantic)]
mod proto_event {
    include!(concat!(env!("OUT_DIR"), "/event.rs"));
}
pub use proto_event::*;

use chrono::TimeZone;
use std::borrow::Cow;
use std::collections::BTreeMap;
use tracing::error;

use crate::metadata::WithMetadata;
use crate::proto::event_wrapper::Event;
use crate::tags::Array;
use crate::{Key, LogRecord, MetricValue, Tags};

fn encode_array(items: Vec<crate::log::Value>) -> ValueArray {
    ValueArray {
        items: items.into_iter().map(encode_value).collect(),
    }
}

fn encode_map(fields: BTreeMap<String, crate::log::Value>) -> ValueMap {
    ValueMap {
        map: fields
            .into_iter()
            .map(|(k, v)| (k, encode_value(v)))
            .collect(),
    }
}

fn encode_value(value: crate::log::Value) -> Value {
    Value {
        kind: match value {
            crate::log::Value::Bytes(bytes) => Some(value::Kind::Bytes(bytes)),
            crate::log::Value::Float(f) => Some(value::Kind::Float(f)),
            crate::log::Value::Int64(i) => Some(value::Kind::I64(i)),
            crate::log::Value::Boolean(b) => Some(value::Kind::Boolean(b)),
            crate::log::Value::Array(arr) => Some(value::Kind::Array(encode_array(arr))),
            crate::log::Value::Object(m) => Some(value::Kind::Map(encode_map(m))),
            crate::log::Value::Timestamp(ts) => {
                Some(value::Kind::Timestamp(prost_types::Timestamp {
                    seconds: ts.timestamp(),
                    nanos: ts.timestamp_subsec_nanos() as i32,
                }))
            }
            crate::log::Value::Null => Some(value::Kind::Null(0)),
        },
    }
}

fn decode_value(input: Value) -> Option<crate::log::Value> {
    match input.kind {
        Some(value::Kind::Bytes(b)) => Some(crate::log::Value::Bytes(b)),
        Some(value::Kind::Float(f)) => Some(crate::log::Value::Float(f)),
        Some(value::Kind::I64(i)) => Some(crate::log::Value::Int64(i)),
        Some(value::Kind::Boolean(b)) => Some(crate::log::Value::Boolean(b)),
        Some(value::Kind::Array(a)) => decode_array(a.items),
        Some(value::Kind::Map(m)) => decode_map(m.map),
        Some(value::Kind::Timestamp(ts)) => Some(crate::log::Value::Timestamp(
            chrono::Utc.timestamp_nanos(ts.seconds * 1_000_000_000 + ts.nanos as i64),
        )),
        Some(value::Kind::Null(_)) => Some(crate::log::Value::Null),
        None => {
            error!(message = "Encode event contains unknown value kind.");
            None
        }
    }
}

fn decode_map(fields: BTreeMap<String, Value>) -> Option<crate::log::Value> {
    let mut accum: BTreeMap<String, crate::log::Value> = BTreeMap::new();

    for (key, value) in fields {
        match decode_value(value) {
            Some(value) => {
                accum.insert(key, value);
            }
            None => return None,
        }
    }

    Some(crate::log::Value::Object(accum))
}

fn decode_array(items: Vec<Value>) -> Option<crate::log::Value> {
    let mut accum = Vec::with_capacity(items.len());

    for value in items {
        match decode_value(value) {
            Some(value) => accum.push(value),
            None => return None,
        }
    }

    Some(crate::log::Value::Array(accum))
}

impl From<TagValueArray> for crate::tags::Array {
    fn from(array: TagValueArray) -> Self {
        match array.kind {
            0 => crate::tags::Array::Bool(array.bool),
            1 => crate::tags::Array::I64(array.i64),
            2 => crate::tags::Array::F64(array.f64),
            3 => crate::tags::Array::String(array.string.into_iter().map(Cow::from).collect()),
            _ => unreachable!(), // TryFrom is what we need
        }
    }
}

impl From<crate::tags::Array> for TagValueArray {
    fn from(array: Array) -> Self {
        match array {
            Array::Bool(b) => TagValueArray {
                kind: 0,
                bool: b,
                ..Default::default()
            },
            Array::I64(i) => TagValueArray {
                kind: 1,
                i64: i,
                ..Default::default()
            },
            Array::F64(f) => TagValueArray {
                kind: 2,
                f64: f,
                ..Default::default()
            },
            Array::String(s) => TagValueArray {
                kind: 3,
                string: s.into_iter().map(|s| s.to_string()).collect(),
                ..Default::default()
            },
        }
    }
}

impl From<TagValue> for crate::tags::Value {
    fn from(value: TagValue) -> Self {
        match value.value.unwrap() {
            tag_value::Value::Bool(b) => crate::tags::Value::Bool(b),
            tag_value::Value::I64(i) => crate::tags::Value::I64(i),
            tag_value::Value::F64(f) => crate::tags::Value::F64(f),
            tag_value::Value::String(s) => crate::tags::Value::String(Cow::from(s)),
            tag_value::Value::Array(a) => crate::tags::Value::Array(a.into()),
        }
    }
}

impl From<crate::tags::Value> for TagValue {
    fn from(value: crate::tags::Value) -> Self {
        let tv = match value {
            crate::tags::Value::Bool(b) => tag_value::Value::Bool(b),
            crate::tags::Value::I64(i) => tag_value::Value::I64(i),
            crate::tags::Value::F64(f) => tag_value::Value::F64(f),
            crate::tags::Value::String(s) => tag_value::Value::String(s.to_string()),
            crate::tags::Value::Array(a) => tag_value::Value::Array(a.into()),
        };

        TagValue { value: Some(tv) }
    }
}

impl From<BTreeMap<String, TagValue>> for Tags {
    fn from(m: BTreeMap<String, TagValue>) -> Self {
        m.into_iter()
            .map(|(k, v)| (Key::from(k), crate::tags::Value::from(v)))
            .collect()
    }
}

impl From<Log> for crate::LogRecord {
    fn from(log: Log) -> Self {
        let fields = log
            .fields
            .into_iter()
            .filter_map(|(k, v)| decode_value(v).map(|value| (k, value)))
            .collect::<BTreeMap<_, _>>();

        crate::LogRecord::new(log.tags.into(), fields)
    }
}

impl From<crate::Metric> for Metric {
    fn from(metric: crate::Metric) -> Self {
        WithMetadata::<Self>::from(metric).data
    }
}

impl From<crate::Bucket> for Bucket {
    fn from(b: crate::Bucket) -> Self {
        Self {
            count: b.count,
            upper: b.upper,
        }
    }
}

impl From<crate::Quantile> for Quantile {
    fn from(q: crate::Quantile) -> Self {
        Self {
            quantile: q.quantile,
            value: q.value,
        }
    }
}

impl From<crate::Metric> for WithMetadata<Metric> {
    fn from(metric: crate::Metric) -> Self {
        let (series, value, timestamp, metadata) = metric.into_parts();
        let timestamp = timestamp.map(|ts| prost_types::Timestamp {
            seconds: ts.timestamp(),
            nanos: ts.timestamp_subsec_nanos() as i32,
        });

        let value = match value {
            MetricValue::Sum(value) => metric::Value::Counter(Counter { value }),
            MetricValue::Gauge(value) => metric::Value::Gauge(Gauge { value }),
            MetricValue::Histogram {
                count,
                sum,
                buckets,
            } => metric::Value::Histogram(Histogram {
                count,
                sum,
                buckets: buckets.into_iter().map(Into::into).collect(),
            }),
            MetricValue::Summary {
                count,
                sum,
                quantiles,
            } => metric::Value::Summary(Summary {
                count,
                sum,
                quantiles: quantiles.into_iter().map(Into::into).collect(),
            }),
        };

        let data = Metric {
            name: series.name,
            tags: series
                .tags
                .into_iter()
                .map(|(k, v)| (k.to_string(), TagValue::from(v)))
                .collect(),
            description: String::new(),
            unit: String::new(),
            timestamp,
            value: Some(value),
        };

        Self { data, metadata }
    }
}

impl From<Metric> for crate::Metric {
    fn from(metric: Metric) -> Self {
        let Metric {
            name,
            tags,
            description,
            timestamp,
            value,
            ..
        } = metric;

        let timestamp = timestamp
            .map(|ts| chrono::Utc.timestamp_nanos(ts.seconds * 1_000_000_000 + ts.nanos as i64));

        let value = match value.unwrap() {
            metric::Value::Counter(counter) => MetricValue::Sum(counter.value),
            metric::Value::Gauge(gauge) => MetricValue::Gauge(gauge.value),
            metric::Value::Histogram(Histogram {
                count,
                sum,
                buckets,
            }) => MetricValue::Histogram {
                count,
                sum,
                buckets: buckets
                    .into_iter()
                    .map(|b| crate::Bucket {
                        count: b.count,
                        upper: b.upper,
                    })
                    .collect(),
            },
            metric::Value::Summary(Summary {
                count,
                sum,
                quantiles,
            }) => MetricValue::Summary {
                count,
                sum,
                quantiles: quantiles
                    .into_iter()
                    .map(|q| crate::Quantile {
                        quantile: q.quantile,
                        value: q.value,
                    })
                    .collect(),
            },
        };

        let tags = tags
            .into_iter()
            .map(|(k, v)| (Key::from(k), crate::tags::Value::from(v)))
            .collect();

        crate::Metric::new(name, Some(description), tags, timestamp.unwrap(), value)
    }
}

impl From<LogRecord> for Log {
    fn from(log: LogRecord) -> Self {
        WithMetadata::<Self>::from(log).data
    }
}

impl From<Log> for event_wrapper::Event {
    fn from(log: Log) -> Self {
        Self::Log(log)
    }
}

impl From<Metric> for event_wrapper::Event {
    fn from(metric: Metric) -> Self {
        Self::Metric(metric)
    }
}

impl From<LogRecord> for WithMetadata<Log> {
    fn from(log: LogRecord) -> Self {
        let (tags, fields, metadata) = log.into_parts();
        let tags = tags
            .into_iter()
            .map(|(k, v)| (k.to_string(), TagValue::from(v)))
            .collect();

        let fields = if let crate::log::Value::Object(fields) = fields {
            fields
                .into_iter()
                .map(|(k, v)| (k, encode_value(v)))
                .collect::<BTreeMap<_, _>>()
        } else {
            // dummy
            BTreeMap::new()
        };

        Self {
            data: Log { tags, fields },
            metadata,
        }
    }
}

impl From<crate::Event> for WithMetadata<Event> {
    fn from(event: crate::Event) -> Self {
        match event {
            crate::Event::Log(log) => WithMetadata::<Log>::from(log).into(),
            crate::Event::Metric(metric) => WithMetadata::<Metric>::from(metric).into(),
            crate::Event::Trace(_span) => todo!(),
        }
    }
}

impl From<crate::Event> for EventWrapper {
    fn from(event: crate::Event) -> Self {
        WithMetadata::<EventWrapper>::from(event).data
    }
}

impl From<Event> for EventWrapper {
    fn from(event: Event) -> Self {
        Self { event: Some(event) }
    }
}

impl From<crate::Event> for WithMetadata<EventWrapper> {
    fn from(event: crate::Event) -> Self {
        WithMetadata::<Event>::from(event).into()
    }
}

impl From<EventWrapper> for crate::Event {
    fn from(wrapper: EventWrapper) -> Self {
        let event = wrapper.event.unwrap();

        match event {
            Event::Log(log) => Self::Log(log.into()),
            Event::Metric(metric) => Self::Metric(metric.into()),
        }
    }
}

impl events::Events {
    fn from_logs(logs: crate::Logs) -> Self {
        let logs = logs.into_iter().map(Into::into).collect();
        Self::Logs(events::Logs { logs })
    }

    fn from_metrics(metrics: crate::Metrics) -> Self {
        let metrics = metrics.into_iter().map(Into::into).collect();
        Self::Metrics(events::Metrics { metrics })
    }
}

impl From<crate::Events> for Events {
    fn from(events: crate::Events) -> Self {
        let events = Some(match events {
            crate::Events::Logs(logs) => events::Events::from_logs(logs),
            crate::Events::Metrics(metrics) => events::Events::from_metrics(metrics),
            crate::Events::Traces(_) => unimplemented!(),
        });

        Self { events }
    }
}

impl From<Events> for crate::Events {
    fn from(events: Events) -> Self {
        let events = events.events.unwrap();

        match events {
            events::Events::Logs(logs) => {
                crate::Events::Logs(logs.logs.into_iter().map(Into::into).collect())
            }
            events::Events::Metrics(metrics) => {
                crate::Events::Metrics(metrics.metrics.into_iter().map(Into::into).collect())
            }
        }
    }
}
