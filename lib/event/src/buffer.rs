use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Formatter};

use buffer::Encodable;
use bytes::{Buf, BufMut};
use chrono::{DateTime, TimeZone, Utc};
use typesize::TypeSize;
use value::Value;

use super::metric::{Bucket, Metric, MetricValue};
use crate::tags::Tags;
use crate::{EventMetadata, Events, LogRecord, Quantile, Trace};

#[derive(Debug)]
pub enum Error {
    UnknownType(&'static str, u32),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::UnknownType(typ, id) => write!(f, "Unknown type {} when decode {}", id, typ),
        }
    }
}

impl std::error::Error for Error {}

impl Encodable for Events {
    type Error = Error;

    fn encode<B: BufMut>(&self, buf: &mut B) -> Result<(), Self::Error> {
        match self {
            Events::Logs(logs) => {
                let head = encode_type_and_length(1, logs.len() as u32);
                buf.put_u32(head);

                for log in logs {
                    encode_log(log, buf)?;
                }
            }
            Events::Metrics(metrics) => {
                let head = encode_type_and_length(2, metrics.len() as u32);
                buf.put_u32(head);

                for metric in metrics {
                    encode_metric(metric, buf)?;
                }
            }
            Events::Traces(traces) => {
                let head = encode_type_and_length(3, traces.len() as u32);
                buf.put_u32(head);

                for trace in traces {
                    encode_trace(trace, buf)?;
                }
            }
        }

        Ok(())
    }

    fn decode<B: Buf>(mut buf: B) -> Result<Self, Self::Error> {
        let head = buf.get_u32();
        let (typ, len) = decode_head(head);

        let events = match typ {
            1 => {
                let mut logs = Vec::with_capacity(len as usize);
                for _ in 0..len {
                    let log = decode_log(&mut buf)?;
                    logs.push(log);
                }

                Events::Logs(logs)
            }
            2 => {
                let mut metrics = Vec::with_capacity(len as usize);
                for _ in 0..len {
                    let metric = decode_metric(&mut buf)?;
                    metrics.push(metric);
                }

                Events::Metrics(metrics)
            }
            3 => {
                let mut traces = Vec::with_capacity(len as usize);
                for _ in 0..len {
                    let trace = decode_trace(&mut buf)?;
                    traces.push(trace);
                }

                Events::Traces(traces)
            }
            _ => return Err(Error::UnknownType("events", typ)),
        };

        Ok(events)
    }

    fn byte_size(&self) -> usize {
        self.size_of()
    }
}

fn encode_type_and_length(typ: u32, length: u32) -> u32 {
    let prefix = typ << 30;
    prefix | length
}

fn decode_head(head: u32) -> (u32, u32) {
    let len = head & 0b0011_1111_1111_1111_1111_1111_1111_1111;
    let typ = head >> 30;

    (typ, len)
}

fn decode_option_string<B: Buf>(buf: &mut B) -> Result<Option<String>, Error> {
    let len = buf.get_i32();
    if len == -1 {
        return Ok(None);
    }

    let mut s = vec![0u8; len as usize];
    buf.copy_to_slice(s.as_mut_slice());

    Ok(Some(unsafe { String::from_utf8_unchecked(s) }))
}

fn encode_option_string<B: BufMut>(s: Option<&str>, buf: &mut B) -> Result<(), Error> {
    match s {
        Some(s) => {
            buf.put_i32(s.len() as i32);
            buf.put_slice(s.as_bytes());
        }
        None => buf.put_i32(-1),
    }

    Ok(())
}

fn decode_string<B: Buf>(buf: &mut B) -> Result<String, Error> {
    let len = buf.get_u16();

    let mut s = vec![0u8; len as usize];
    buf.copy_to_slice(s.as_mut_slice());

    Ok(unsafe { String::from_utf8_unchecked(s) })
}

fn encode_string<B: BufMut>(s: &str, buf: &mut B) -> Result<(), Error> {
    buf.put_u16(s.len() as u16);
    buf.put_slice(s.as_bytes());
    Ok(())
}

const VALUE_TYPE_BYTES: u8 = 1;
const VALUE_TYPE_INTEGER: u8 = 2;
const VALUE_TYPE_FLOAT: u8 = 3;
const VALUE_TYPE_TRUE: u8 = 4;
const VALUE_TYPE_FALSE: u8 = 5;
const VALUE_TYPE_TIMESTAMP: u8 = 6;
const VALUE_TYPE_OBJECT: u8 = 7;
const VALUE_TYPE_ARRAY: u8 = 8;
const VALUE_TYPE_NULL: u8 = 9;

fn decode_value<B: Buf>(buf: &mut B) -> Result<Value, Error> {
    let value = match buf.get_u8() {
        VALUE_TYPE_BYTES => {
            let len = buf.get_u32();
            let data = buf.copy_to_bytes(len as usize);
            Value::Bytes(data)
        }
        VALUE_TYPE_INTEGER => {
            let i = buf.get_i64();
            Value::Integer(i)
        }
        VALUE_TYPE_FLOAT => {
            let f = buf.get_f64();
            Value::Float(f)
        }
        VALUE_TYPE_TRUE => Value::Boolean(true),
        VALUE_TYPE_FALSE => Value::Boolean(false),
        VALUE_TYPE_TIMESTAMP => {
            let i = buf.get_i64();
            Value::Timestamp(DateTime::from_timestamp_nanos(i))
        }
        VALUE_TYPE_OBJECT => {
            let len = buf.get_u32();

            let mut obj = BTreeMap::new();
            for _ in 0..len {
                let key = decode_string(buf)?;
                let value = decode_value(buf)?;

                obj.insert(key, value);
            }

            Value::Object(obj)
        }
        VALUE_TYPE_ARRAY => {
            let len = buf.get_u32();

            let mut arr = Vec::with_capacity(len as usize);
            for _ in 0..len {
                arr.push(decode_value(buf)?);
            }

            Value::Array(arr)
        }
        VALUE_TYPE_NULL => Value::Null,
        typ => return Err(Error::UnknownType("value", typ as u32)),
    };

    Ok(value)
}

fn encode_value<B: BufMut>(value: &Value, buf: &mut B) -> Result<(), Error> {
    match value {
        Value::Bytes(b) => {
            buf.put_u8(VALUE_TYPE_BYTES);
            buf.put_u32(b.len() as u32);
            buf.put_slice(b);
        }
        Value::Integer(i) => {
            buf.put_u8(VALUE_TYPE_INTEGER);
            buf.put_i64(*i);
        }
        Value::Float(f) => {
            buf.put_u8(VALUE_TYPE_FLOAT);
            buf.put_f64(*f);
        }
        Value::Boolean(b) => {
            if *b {
                buf.put_u8(VALUE_TYPE_TRUE);
            } else {
                buf.put_u8(VALUE_TYPE_FALSE);
            }
        }
        Value::Timestamp(ts) => {
            let ts = ts.timestamp_nanos_opt().unwrap();

            buf.put_u8(VALUE_TYPE_TIMESTAMP);
            buf.put_i64(ts);
        }
        Value::Object(obj) => {
            buf.put_u8(VALUE_TYPE_OBJECT);
            let len = obj.len();
            buf.put_u32(len as u32);

            for (k, v) in obj {
                encode_string(k, buf)?;
                encode_value(v, buf)?;
            }
        }
        Value::Array(arr) => {
            buf.put_u8(VALUE_TYPE_ARRAY);
            let len = arr.len();
            buf.put_u32(len as u32);

            for item in arr {
                encode_value(item, buf)?;
            }
        }
        Value::Null => {
            buf.put_u8(VALUE_TYPE_NULL);
        }
    }

    Ok(())
}

fn decode_metadata<B: Buf>(buf: &mut B) -> Result<EventMetadata, Error> {
    let value = decode_value(buf)?;

    let len = buf.get_i16();
    let source_id = if len == -1 {
        None
    } else {
        let mut dst = String::with_capacity(len as usize);
        buf.copy_to_slice(unsafe { dst.as_bytes_mut() });
        Some(Cow::Owned(dst))
    };

    let len = buf.get_i16();
    let source_type = if len == -1 {
        None
    } else {
        let mut dst = String::with_capacity(len as usize);
        buf.copy_to_slice(unsafe { dst.as_bytes_mut() });
        Some(Cow::Owned(dst))
    };

    Ok(EventMetadata::from_parts(value, source_id, source_type))
}

fn encode_metadata<B: BufMut>(metadata: &EventMetadata, buf: &mut B) -> Result<(), Error> {
    encode_value(metadata.value(), buf)?;

    match metadata.source_id() {
        None => {
            buf.put_i16(-1);
        }
        Some(source_id) => {
            buf.put_i16(source_id.len() as i16);
            buf.put_slice(source_id.as_ref());
        }
    }

    match metadata.source_type() {
        None => {
            buf.put_i16(-1);
        }
        Some(source_type) => {
            buf.put_i16(source_type.len() as i16);
            buf.put_slice(source_type.as_ref());
        }
    }

    Ok(())
}

fn decode_tags<B: Buf>(buf: &mut B) -> Result<Tags, Error> {
    use super::tags::Value as TagValue;

    let len = buf.get_u32();
    let mut tags = Tags::with_capacity(len as usize);
    for _ in 0..len {
        let key = decode_string(buf)?;

        let value = match buf.get_u8() {
            0 => TagValue::Bool(false),
            1 => TagValue::Bool(true),
            2 => TagValue::I64(buf.get_i64()),
            3 => TagValue::F64(buf.get_f64()),
            4 => TagValue::String(decode_string(buf)?.into()),
            typ => return Err(Error::UnknownType("tag value", typ as u32)),
        };

        tags.insert(key, value);
    }

    Ok(tags)
}

fn encode_tags<B: BufMut>(tags: &Tags, buf: &mut B) -> Result<(), Error> {
    use super::tags::Value as TagValue;

    let len = tags.len();
    buf.put_u32(len as u32);

    for (key, value) in tags {
        encode_string(key.as_str(), buf)?;

        match value {
            TagValue::Bool(b) => {
                buf.put_u8(if *b { 1 } else { 0 });
            }
            TagValue::I64(i) => {
                buf.put_u8(2);
                buf.put_i64(*i);
            }
            TagValue::F64(f) => {
                buf.put_u8(3);
                buf.put_f64(*f);
            }
            TagValue::String(s) => {
                buf.put_u8(4);
                encode_string(s, buf)?;
            }
            TagValue::Array(_) => {
                panic!("not implemented");
            }
        }
    }

    Ok(())
}

fn decode_log<B: Buf>(buf: &mut B) -> Result<LogRecord, Error> {
    let metadata = decode_metadata(buf)?;
    let value = decode_value(buf)?;

    Ok(LogRecord::from_parts(metadata, value))
}

fn encode_log<B: BufMut>(log: &LogRecord, buf: &mut B) -> Result<(), Error> {
    encode_metadata(log.metadata(), buf)?;
    encode_value(log.value(), buf)
}

fn decode_metric<B: Buf>(buf: &mut B) -> Result<Metric, Error> {
    let name = decode_string(buf)?;
    let tags = decode_tags(buf)?;
    let description = decode_option_string(buf)?.map(Cow::Owned);

    let timestamp = match buf.get_i64() {
        -1 => None,
        ts => Some(Utc.timestamp_nanos(ts)),
    };

    let value = match buf.get_u8() {
        1 => {
            let v = buf.get_f64();
            MetricValue::Sum(v)
        }
        2 => {
            let v = buf.get_f64();
            MetricValue::Gauge(v)
        }
        3 => {
            let count = buf.get_u64();
            let sum = buf.get_f64();

            let len = buf.get_u32();
            let mut buckets = Vec::with_capacity(len as usize);
            for _ in 0..len {
                let upper = buf.get_f64();
                let count = buf.get_u64();

                buckets.push(Bucket { upper, count });
            }

            MetricValue::Histogram {
                count,
                sum,
                buckets,
            }
        }
        4 => {
            let count = buf.get_u64();
            let sum = buf.get_f64();

            let len = buf.get_u32();
            let mut quantiles = Vec::with_capacity(len as usize);
            for _ in 0..len {
                let quantile = buf.get_f64();
                let value = buf.get_f64();

                quantiles.push(Quantile { quantile, value });
            }

            MetricValue::Summary {
                count,
                sum,
                quantiles,
            }
        }

        typ => return Err(Error::UnknownType("metric value", typ as u32)),
    };

    let metadata = decode_metadata(buf)?;

    Ok(Metric::new_with_metadata(
        Cow::Owned(name),
        tags,
        description,
        value,
        timestamp,
        metadata,
    ))
}

fn encode_metric<B: BufMut>(metric: &Metric, buf: &mut B) -> Result<(), Error> {
    encode_string(metric.name(), buf)?;
    encode_tags(metric.tags(), buf)?;
    encode_option_string(metric.description.as_deref(), buf)?;

    match &metric.timestamp {
        Some(ts) => {
            let ts = ts.timestamp_nanos_opt().unwrap();
            buf.put_i64(ts);
        }
        None => {
            buf.put_i64(-1);
        }
    }

    match &metric.value {
        MetricValue::Sum(f) => {
            buf.put_u8(1);
            buf.put_f64(*f);
        }
        MetricValue::Gauge(f) => {
            buf.put_u8(2);
            buf.put_f64(*f);
        }
        MetricValue::Histogram {
            count,
            sum,
            buckets,
        } => {
            buf.put_u64(*count);
            buf.put_f64(*sum);

            buf.put_u32(buckets.len() as u32);
            for bucket in buckets {
                buf.put_f64(bucket.upper);
                buf.put_u64(bucket.count);
            }
        }
        MetricValue::Summary {
            count,
            sum,
            quantiles,
        } => {
            buf.put_u64(*count);
            buf.put_f64(*sum);

            buf.put_u32(quantiles.len() as u32);
            for quantile in quantiles {
                buf.put_f64(quantile.quantile);
                buf.put_f64(quantile.value);
            }
        }
    }

    encode_metadata(metric.metadata(), buf)
}

fn encode_trace<B: BufMut>(_trace: &Trace, _buf: &mut B) -> Result<(), Error> {
    todo!()
}

fn decode_trace<B: Buf>(_buf: B) -> Result<Trace, Error> {
    todo!()
}

#[cfg(test)]
mod tests {
    use bytes::BytesMut;

    use super::*;
    use crate::tags;

    #[test]
    fn string() {
        let s = String::from("hello world");
        let mut buf = BytesMut::new();
        encode_string(&s, &mut buf).unwrap();

        let got = decode_string(&mut buf).unwrap();
        assert_eq!(s, got);
    }

    #[test]
    fn tags() {
        let mut tags = Tags::default();
        let mut buf = BytesMut::with_capacity(1024);
        encode_tags(&tags, &mut buf).unwrap();

        tags.insert("foo", "bar");
        let mut buf = BytesMut::with_capacity(1024);
        encode_tags(&tags, &mut buf).unwrap();

        let got = decode_tags(&mut buf).unwrap();
        assert_eq!(tags, got);
    }

    #[test]
    fn metrics() {
        for (case, metric) in [
            (
                "sum",
                Metric::new(
                    "foo",
                    Some("bar".into()),
                    tags!(
                        "foo" => "bar"
                    ),
                    Utc::now(),
                    MetricValue::Sum(1.2),
                ),
            ),
            (
                "sum without description",
                Metric::new(
                    "foo",
                    None,
                    tags!(
                        "foo" => "bar"
                    ),
                    Utc::now(),
                    MetricValue::Sum(1.2),
                ),
            ),
            (
                "sum without description and tags",
                Metric::new("foo", None, tags!(), Utc::now(), MetricValue::Sum(1.2)),
            ),
            (
                "sum without description, tags and timestamp",
                Metric::new_with_metadata(
                    "foo".into(),
                    tags!(),
                    None,
                    MetricValue::Sum(1.2),
                    None,
                    EventMetadata::default(),
                ),
            ),
            /* --------- GAUGE ------------ */
            (
                "gauge",
                Metric::new(
                    "foo",
                    Some("bar".into()),
                    tags!(
                        "foo" => "bar"
                    ),
                    Utc::now(),
                    MetricValue::Gauge(1.2),
                ),
            ),
            (
                "gauge without description",
                Metric::new(
                    "foo",
                    None,
                    tags!(
                        "foo" => "bar"
                    ),
                    Utc::now(),
                    MetricValue::Gauge(1.2),
                ),
            ),
            (
                "gauge without description and tags",
                Metric::new("foo", None, tags!(), Utc::now(), MetricValue::Gauge(1.2)),
            ),
            (
                "gauge without description, tags and timestamp",
                Metric::new_with_metadata(
                    "foo".into(),
                    tags!(),
                    None,
                    MetricValue::Gauge(1.2),
                    None,
                    EventMetadata::default(),
                ),
            ),
        ] {
            let mut buf = BytesMut::with_capacity(1024);
            encode_metric(&metric, &mut buf).unwrap();
            let got = decode_metric(&mut buf).unwrap();
            assert_eq!(metric, got, "{}", case);
        }
    }

    #[test]
    fn logs() {
        for (case, log) in [
            ("empty", LogRecord::default()),
            ("simple", LogRecord::from("foo")),
        ] {
            let mut buf = BytesMut::with_capacity(1024);
            encode_log(&log, &mut buf).unwrap();
            let got = decode_log(&mut buf).unwrap();
            assert_eq!(log, got, "{}", case);
        }
    }
}
