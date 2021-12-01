use rdkafka::message::OwnedHeaders;
use event::{Event, encoding::{EncodingConfig, StandardEncodings}, Finalizable, Value};
use event::encoding::Encoder;
use internal::emit;
use log_schema::LogSchema;
use crate::common::kafka::KafkaHeaderExtractionFailed;
use crate::sinks::kafka::service::KafkaRequestMetadata;

use super::service::KafkaRequest;
use crate::template::Template;


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
        let meta = KafkaRequestMetadata {
            finalizers: event.take_finalizers(),
            key: get_key(&event, &self.key_field),
            timestamp_millis: get_timestamp_millis(&event),
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

fn get_timestamp_millis(event: &Event, log_schema: &'static LogSchema) -> Option<i64> {
    match &event {
        Event::Log(log) => log.get_field(log_schema.timestamp_key())
            .and_then(|v| v.as_timestamp())
            .copied(),
        Event::Metric(metric) => metric.timestamp
    }.map(|ts| ts.timestamp_millis())
}

fn get_headers(ev: &Event, headers_field: &Option<String>) -> Option<OwnedHeaders> {
    headers_field.as_ref()
        .and_then(|field| {
            if let Event::Log(log) = ev {
                if let Some(headers) = log.get_field(headers_field) {
                    match headers {
                        Value::Map(map) => {
                            let mut owned_headers = OwnedHeaders::new_with_capacity(map.len());
                            for (key, value) in map {
                                if let Value::Bytes(b) => value {
                                    owned_headers = owned_headers.add(key,
                                    b.as_ref());
                                } else {
                                    emit!(&KafkaHeaderExtractionFailed {
                                        headers_field: headers_field
                                    });
                                }
                            }

                            return Some(owned_headers);
                        }

                        _ => {
                            emit!(&KafkaHeaderExtractionFailed {
                                headers_field: headers_field
                            });
                        }
                    }
                }
            }

            None
        })
}