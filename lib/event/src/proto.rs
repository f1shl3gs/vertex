include!(concat!(env!("OUT_DIR"), "/event.rs"));

use chrono::TimeZone;
use std::collections::BTreeMap;
use tracing::error;

use crate::metadata::WithMetadata;
use crate::proto::event_wrapper::Event;
use crate::{LogRecord, MetricValue};

fn encode_array(items: Vec<crate::Value>) -> ValueArray {
    ValueArray {
        items: items.into_iter().map(encode_value).collect(),
    }
}

fn encode_map(fields: BTreeMap<String, crate::Value>) -> ValueMap {
    ValueMap {
        map: fields
            .into_iter()
            .map(|(k, v)| (k, encode_value(v)))
            .collect(),
    }
}

fn encode_value(value: crate::Value) -> Value {
    Value {
        kind: match value {
            crate::Value::Bytes(bytes) => Some(value::Kind::Bytes(bytes)),
            crate::Value::Float(f) => Some(value::Kind::Float(f)),
            crate::Value::Uint64(u) => Some(value::Kind::U64(u)),
            crate::Value::Int64(i) => Some(value::Kind::I64(i)),
            crate::Value::Boolean(b) => Some(value::Kind::Boolean(b)),
            crate::Value::Array(arr) => Some(value::Kind::Array(encode_array(arr))),
            crate::Value::Map(m) => Some(value::Kind::Map(encode_map(m))),
            crate::Value::Timestamp(ts) => Some(value::Kind::Timestamp(prost_types::Timestamp {
                seconds: ts.timestamp(),
                nanos: ts.timestamp_subsec_nanos() as i32,
            })),
            crate::Value::Null => Some(value::Kind::Null(0)),
        },
    }
}

fn decode_value(input: Value) -> Option<crate::Value> {
    match input.kind {
        Some(value::Kind::Bytes(b)) => Some(crate::Value::Bytes(b)),
        Some(value::Kind::Float(f)) => Some(crate::Value::Float(f)),
        Some(value::Kind::U64(u)) => Some(crate::Value::Uint64(u)),
        Some(value::Kind::I64(i)) => Some(crate::Value::Int64(i)),
        Some(value::Kind::Boolean(b)) => Some(crate::Value::Boolean(b)),
        Some(value::Kind::Array(a)) => decode_array(a.items),
        Some(value::Kind::Map(m)) => decode_map(m.map),
        Some(value::Kind::Timestamp(ts)) => Some(crate::Value::Timestamp(
            chrono::Utc.timestamp(ts.seconds, ts.nanos as u32),
        )),
        Some(value::Kind::Null(_)) => Some(crate::Value::Null),
        None => {
            error!(message = "Encode event contains unknown value kind.");
            None
        }
    }
}

fn decode_map(fields: BTreeMap<String, Value>) -> Option<crate::Value> {
    let mut accum: BTreeMap<String, crate::Value> = BTreeMap::new();

    for (key, value) in fields {
        match decode_value(value) {
            Some(value) => {
                accum.insert(key, value);
            }
            None => return None,
        }
    }

    Some(crate::Value::Map(accum))
}

fn decode_array(items: Vec<Value>) -> Option<crate::Value> {
    let mut accum = Vec::with_capacity(items.len());

    for value in items {
        match decode_value(value) {
            Some(value) => accum.push(value),
            None => return None,
        }
    }

    Some(crate::Value::Array(accum))
}

impl From<Log> for crate::LogRecord {
    fn from(log: Log) -> Self {
        let fields = log
            .fields
            .into_iter()
            .filter_map(|(k, v)| decode_value(v).map(|value| (k, value)))
            .collect::<BTreeMap<_, _>>();

        crate::LogRecord::new(log.tags, fields)
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
            tags: series.tags,
            description: "".to_string(),
            unit: "".to_string(),
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

        let timestamp = timestamp.map(|ts| chrono::Utc.timestamp(ts.seconds, ts.nanos as u32));

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

        crate::Metric::new(name, Some(description), tags, timestamp.unwrap(), value)
    }
}

impl From<crate::LogRecord> for Log {
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

impl From<crate::LogRecord> for WithMetadata<Log> {
    fn from(log: LogRecord) -> Self {
        let (tags, fields, metadata) = log.into_parts();
        let fields = fields
            .into_iter()
            .map(|(k, v)| (k, encode_value(v)))
            .collect::<BTreeMap<_, _>>();

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
