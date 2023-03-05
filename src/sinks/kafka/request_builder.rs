use std::collections::BTreeMap;

use bytes::BytesMut;
use chrono::{DateTime, Utc};
use codecs::encoding::Transformer;
use codecs::Encoder;
use event::{log::Value, Event, Finalizable};
use framework::template::Template;
use log_schema::LogSchema;
use tokio_util::codec::Encoder as _;

use super::service::KafkaRequest;
use super::service::KafkaRequestMetadata;

pub struct KafkaRequestBuilder {
    pub key_field: Option<String>,
    pub headers_field: Option<String>,
    pub topic_template: Template,
    pub transformer: Transformer,
    pub encoder: Encoder<()>,
    pub log_schema: &'static LogSchema,
}

impl KafkaRequestBuilder {
    pub fn build_request(&mut self, mut event: Event) -> Option<KafkaRequest> {
        let topic = self.topic_template.render_string(&event).ok()?;
        let metadata = KafkaRequestMetadata {
            finalizers: event.take_finalizers(),
            key: get_key(&event, &self.key_field),
            timestamp: get_timestamp(&event, self.log_schema),
            headers: get_headers(&event, &self.headers_field),
            topic,
        };

        let mut body = BytesMut::new();
        let event_byte_size = event.size_of();
        self.encoder.encode(event, &mut body).ok()?;
        let body = Some(body.to_vec());

        Some(KafkaRequest {
            body,
            metadata,
            event_byte_size,
        })
    }
}

fn get_key(event: &Event, key_field: &Option<String>) -> Option<Vec<u8>> {
    key_field.as_ref().and_then(|key_field| match event {
        Event::Log(log) => log
            .get_field(key_field.as_str())
            .map(|v| v.as_bytes().to_vec()),
        Event::Metric(metric) => metric
            .tag_value(key_field)
            .map(|v| v.to_string().into_bytes()),
        Event::Trace(_span) => None,
    })
}

fn get_timestamp(event: &Event, log_schema: &'static LogSchema) -> Option<DateTime<Utc>> {
    match &event {
        Event::Log(log) => log
            .get_field(log_schema.timestamp_key())
            .and_then(|v| v.as_timestamp())
            .copied(),
        Event::Metric(metric) => metric.timestamp,
        Event::Trace(_span) => unreachable!(),
    }
}

fn get_headers(ev: &Event, headers_field: &Option<String>) -> Option<BTreeMap<String, Vec<u8>>> {
    headers_field.as_ref().and_then(|headers_field| {
        if let Event::Log(log) = ev {
            if let Some(value) = log.get_field(headers_field.as_str()) {
                match value {
                    Value::Object(map) => {
                        let mut headers = BTreeMap::new();
                        for (key, value) in map {
                            if let Value::Bytes(b) = value {
                                headers.insert(key.to_string(), b.to_vec());
                            } else {
                                // TODO: metrics
                                warn!(
                                    message = "Failed to extract header. Value should be a map of String -> Bytes",
                                    %headers_field
                                );
                            }
                        }

                        return Some(headers);
                    }

                    _ => {
                        warn!(
                            message = "Failed to extract header. Value should be a map of String -> Bytes",
                            %headers_field
                        )
                    }
                }
            }
        }

        None
    })
}
