use bytes::Bytes;
use event::encoding::Encoder;
use event::{
    encoding::{EncodingConfig, StandardEncodings},
    Event, Finalizable, Value,
};
use framework::template::Template;
use internal::emit;
use log_schema::LogSchema;
use rdkafka::message::OwnedHeaders;

use super::service::KafkaRequest;
use super::service::KafkaRequestMetadata;
use crate::common::kafka::KafkaHeaderExtractionFailed;

pub struct KafkaRequestBuilder {
    pub key_field: Option<String>,
    pub headers_field: Option<String>,
    pub topic_template: Template,
    pub encoder: EncodingConfig<StandardEncodings>,
    pub log_schema: &'static LogSchema,
}

impl KafkaRequestBuilder {
    pub fn build_request(&self, mut event: Event) -> Option<KafkaRequest> {
        let topic = self.topic_template.render_string(&event).ok()?;
        let metadata = KafkaRequestMetadata {
            finalizers: event.take_finalizers(),
            key: get_key(&event, &self.key_field),
            timestamp_millis: get_timestamp_millis(&event, self.log_schema),
            headers: get_headers(&event, &self.headers_field),
            topic,
        };

        let mut body = vec![];
        let event_byte_size = event.size_of();
        self.encoder.encode(event, &mut body).ok()?;

        Some(KafkaRequest {
            body,
            metadata,
            event_byte_size,
        })
    }
}

fn get_key(event: &Event, key_field: &Option<String>) -> Option<Bytes> {
    key_field.as_ref().and_then(|key_field| match event {
        Event::Log(log) => log.get_field(key_field).map(|v| v.as_bytes()),
        Event::Metric(metric) => metric.tags.get(key_field).map(|v| v.clone().into()),
    })
}

fn get_timestamp_millis(event: &Event, log_schema: &'static LogSchema) -> Option<i64> {
    match &event {
        Event::Log(log) => log
            .get_field(log_schema.timestamp_key())
            .and_then(|v| v.as_timestamp())
            .copied(),
        Event::Metric(metric) => metric.timestamp,
    }
    .map(|ts| ts.timestamp_millis())
}

fn get_headers(ev: &Event, headers_field: &Option<String>) -> Option<OwnedHeaders> {
    headers_field.as_ref().and_then(|headers_field| {
        if let Event::Log(log) = ev {
            if let Some(headers) = log.get_field(headers_field) {
                match headers {
                    Value::Map(map) => {
                        let mut owned_headers = OwnedHeaders::new_with_capacity(map.len());
                        for (key, value) in map {
                            if let Value::Bytes(b) = value {
                                owned_headers = owned_headers.add(key, b.as_ref());
                            } else {
                                emit!(&KafkaHeaderExtractionFailed { headers_field });
                            }
                        }

                        return Some(owned_headers);
                    }

                    _ => {
                        emit!(&KafkaHeaderExtractionFailed { headers_field });
                    }
                }
            }
        }

        None
    })
}
