use std::collections::BTreeMap;
use std::time::Duration;
use bytes::Bytes;

use serde::{Deserialize, Serialize};
use snafu::{Snafu, ResultExt};
use buffers::Acker;
use futures::{stream::BoxStream, StreamExt, FutureExt};
use event::{encoding::{Encoder, EncodingConfig, StandardEncodings}, Event, Value};
use rdkafka::{
    ClientConfig, ClientContext, Statistics,
    consumer::{BaseConsumer, Consumer},
    producer::{FutureProducer, FutureRecord},
    util::Timeout,
    message::OwnedHeaders,
};

use crate::batch::BatchConfig;
use crate::common::kafka::{KafkaAuthConfig, KafkaCompression, KafkaHeaderExtractionFailed, KafkaRole, KafkaStatisticsContext};
use crate::sinks::{Sink, StreamSink};
use crate::template::{Template, TemplateParseError};
use crate::config::{
    DataType, deserialize_duration, GenerateConfig, HealthCheck,
    serialize_duration, SinkConfig, SinkContext, SinkDescription,
};


const QUEUED_MIN_MESSAGES: u64 = 100000;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KafkaSinkConfig {
    pub bootstrap_servers: String,
    pub topic: String,
    pub key_field: Option<String>,
    pub compression: KafkaCompression,
    #[serde(default = "default_socket_timeout")]
    #[serde(deserialize_with = "deserialize_duration", serialize_with = "serialize_duration")]
    pub socket_timeout: chrono::Duration,
    #[serde(default = "default_message_timeout")]
    #[serde(deserialize_with = "deserialize_duration", serialize_with = "serialize_duration")]
    pub message_timeout: chrono::Duration,
    pub auth: KafkaAuthConfig,
    pub batch: BatchConfig,
    pub librdkafka_options: BTreeMap<String, String>,
}

fn default_socket_timeout() -> chrono::Duration {
    // default in librdkafka
    chrono::Duration::milliseconds(60000)
}

fn default_message_timeout() -> chrono::Duration {
    // default in librdkafka
    chrono::Duration::milliseconds(300000)
}

inventory::submit! {
    SinkDescription::new::<KafkaSinkConfig>("kafka")
}

impl GenerateConfig for KafkaSinkConfig {
    fn generate_config() -> serde_yaml::Value {
        serde_yaml::to_value(Self {
            bootstrap_servers: "127.0.0.1:9092".to_string(),
            topic: "some_topic".to_string(),
            key_field: Some("uid".to_string()),
            compression: KafkaCompression::None,
            socket_timeout: default_socket_timeout(),
            message_timeout: default_message_timeout(),
            auth: Default::default(),
            batch: Default::default(),
            librdkafka_options: Default::default(),
        }).unwrap()
    }
}

#[derive(Debug, Snafu)]
enum BuildError {
    #[snafu(display("`message_group_id` should be defined for FIFO queue"))]
    MessageGroupIdMissing,
    #[snafu(display("`message_group_id` is not allowed with non-FIFO queue"))]
    MessageGroupIdNotAllowed,
    #[snafu(display("invalid topic template: {}", source))]
    TopicTemplate { source: TemplateParseError },
    #[snafu(display("invalid message_deduplication_id template: {}", source))]
    MessageDeduplicationIdTemplate { source: TemplateParseError },
}

#[async_trait::async_trait]
#[typetag::serde(name = "kafka")]
impl SinkConfig for KafkaSinkConfig {
    async fn build(&self, ctx: SinkContext) -> crate::Result<(Sink, HealthCheck)> {
        let sink = KafkaSink::new(self.clone(), ctx.acker)?;
        let hc = healthcheck(self.clone()).boxed();
        Ok((Sink::Stream(Box::new(sink)), hc))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn sink_type(&self) -> &'static str {
        "kafka"
    }
}

fn to_string(value: impl serde::Serialize) -> String {
    let value = serde_json::to_value(value).unwrap();
    value.as_str().unwrap().into()
}

impl KafkaSinkConfig {
    fn to_rdkafka(&self, role: KafkaRole) -> crate::Result<ClientConfig> {
        let mut client_config = ClientConfig::new();
        let codec = serde_json::to_value(self.compression)?;

        client_config
            .set("bootstrap.servers", &self.bootstrap_servers)
            .set("compression.codec", to_string(self.compression))
            .set("socket.timeout.ms", &self.socket_timeout.to_string())
            .set("message.timeout.ms", &self.message_timeout.to_string())
            .set("statistics.interval.ms", "1000")
            .set("queued.min.messages", QUEUED_MIN_MESSAGES.to_string());

        self.auth.apply(&mut client_config)?;

        // All batch options are producer only
        if role == KafkaRole::Producer {
            if let Some(value) = self.batch.timeout {
                // Delay in milliseconds to wait for messages in the producer queue  to accumulate
                // before constructing message batches(MessageSets) to transmit to brokers. A higher
                // value allows larger and more effective(less overhead, improved compression)
                // batches of messages to accumulate at the expense of increased message delivery
                // latency.
                let key = "queue.buffering.max.ms";
                if let Some(val) = self.librdkafka_options.get(key) {
                    return Err(format!(
                        "Batching setting `batch.timeout_secs` sets `librdkafka_options.{}={}`.\
                         The config already sets this as `librdkafka_options.queue.buffering.max.ms={}`. \
                        Please delete one.",
                        key, value, val
                    ).into());
                }

                debug!(
                    librdkafka_option = key,
                    batch_option = "timeout_secs",
                    value = value.num_seconds(),
                    "Applying batch option as librdkafka option"
                );

                client_config.set(key, value.num_milliseconds().to_string().as_str());
            }

            if let Some(value) = self.batch.max_events {
                // Maximum number of messages batched in one MessageSet. The total MessageSet size
                // is also limited by batch.size and message.max.bytes.
                // Type: integer
                let key = "batch.num.messages";
                if let Some(val) = self.librdkafka_options.get(key) {
                    return Err(format!(
                        "Batching setting `batch.max_events` sets `librdkafka_options.{}={}`.\
                        The config already sets this as `librdkafka_options.batch.num.messages={}`\
                        Please delete one.",
                        key, value, val
                    ).into());
                }

                debug!(
                    librdkafka_option = key,
                    batch_option = "max_events",
                    value,
                    "Applying batch option as librdkafka option."
                );
                client_config.set(key, &value.to_string());
            }

            if let Some(value) = self.batch.max_bytes {
                // Maximum size(in bytes) of all messages batched in one MessageSet, including
                // protocol framing overhead. This limit is applied after the first message has
                // been added to the batch, regardless of the first message's size, this is to
                // ensure that messages that exceed batch.size are produced. The total MessageSet
                // size is also limited by batch.num.messages and message.max.bytes
                // Type: integer
                let key = "batch.size";
                if let Some(val) = self.librdkafka_options.get(key) {
                    return Err(format!(
                        "Batching setting `batch.max_bytes` sets `librdkafka_options.{}={}`.\
                        The config already sets this as `librdkafka_options.batch.size={}`.\
                        Please delete one",
                        key, value, val
                    ).into());
                }
                debug!(
                    librdkafka_option = key,
                    batch_option = "max_bytes",
                    value,
                    "Applying batch option as librdkafka option"
                );

                client_config.set(key, &value.to_string());
            }
        }

        for (key, value) in self.librdkafka_options.iter() {
            debug!(
                option = %key,
                value = %value,
                "Setting librdkafka option"
            );
            client_config.set(key.as_str(), value.as_str());
        }

        Ok(client_config)
    }
}

struct KafkaRequestMetadata {
    key: Option<Bytes>,
    timestamp_millis: Option<i64>,
    headers: Option<OwnedHeaders>,
    topic: String,
}

struct KafkaSink {
    acker: Acker,
    topic: Template,
    key_field: Option<String>,
    headers_field: Option<String>,
    encoder: EncodingConfig<StandardEncodings>,
    producer: FutureProducer<KafkaStatisticsContext>,
}

impl KafkaSink {
    fn new(config: KafkaSinkConfig, acker: Acker) -> crate::Result<Self> {
        todo!()
    }

    fn get_metadata(&self, event: &Event) -> Option<KafkaRequestMetadata> {
        let key = get_key(event, &self.key_field);
        let headers = get_headers(event, &self.headers_field);
        let timestamp_millis = get_timestamp_millis(event);
        let topic = self.topic
            .render_string(event)
            .ok()?;

        Some(KafkaRequestMetadata {
            key,
            timestamp_millis,
            headers,
            topic
        })
    }

    async fn send(&self, event: Event) {
        let metadata = match self.get_metadata(&event) {
            Some(metadata) => metadata,
            _ => return
        };

        let mut payload = vec![];
        let event_byte_size = event.size_of();

        // TODO: We need to refactor the encoder,
        //  cause the input doesn't need to be mutable, and it shouldn't
        if let Err(err) = self.encoder.encode(event, &mut payload) {
            // TODO: handle error
            return;
        }

        let mut record = FutureRecord::to(&metadata.topic)
            .payload(&payload);
        if let Some(key) = &metadata.key {
            record.key = Some(&key[..]);
        }
        if let Some(headers) = metadata.headers {
            record.headers = Some(headers);
        }
        if let Some(timestamp) = metadata.timestamp_millis {
            record.timestamp = Some(timestamp);
        }

        self.producer.send(record, Timeout::Never).await;
    }
}

#[async_trait::async_trait]
impl StreamSink for KafkaSink {
    async fn run(&mut self, mut input: BoxStream<'_, Event>) -> Result<(), ()> {
        while let Some(event) = input.next().await {
            self.send(event).await
        }

        Ok(())
    }
}

async fn healthcheck(config: KafkaSinkConfig) -> crate::Result<()> {
    trace!("Health check started");

    let client = config.to_rdkafka(KafkaRole::Consumer)
        .unwrap();
    let topic = match Template::try_from(config.topic)
        .context(TopicTemplate)?
        .render_string(&Event::from(""))
    {
        Ok(topic) => Some(topic),
        Err(err) => {
            warn!(
                message = "Could not generate topic for healthcheck",
                %err
            );
            None
        }
    };

    tokio::task::spawn_blocking(move || {
        let consumer: BaseConsumer = client.create()
            .unwrap();
        let topic = topic.as_ref().map(|topic| &topic[..]);

        consumer
            .fetch_metadata(topic, Duration::from_secs(3))
            .map(|_| ())
    }).await??;

    trace!(message = "Health check completed");

    Ok(())
}

fn get_headers(event: &Event, headers_field: &Option<String>) -> Option<OwnedHeaders> {
    headers_field.as_ref()
        .and_then(|headers_field| {
            if let Event::Log(log) = event {
                if let Some(headers) = log.get_field(headers_field) {
                    match headers {
                        Value::Map(map) => {
                            let mut owned_headers = OwnedHeaders::new_with_capacity(map.len());
                            for (key, value) in map {
                                if let Value::Bytes(value_bytes) = value {
                                    owned_headers = owned_headers.add(key, value_bytes.as_ref());
                                } else {
                                    emit!(&KafkaHeaderExtractionFailed {
                                        headers_field
                                    });
                                }
                            }

                            return Some(owned_headers);
                        }

                        _ => {
                            emit!(&KafkaHeaderExtractionFailed {
                                headers_field
                            });
                        }
                    }
                }
            }
            None
        })
}

fn get_timestamp_millis(event: &Event) -> Option<i64> {
    match &event {
        Event::Log(log) => {
            log.get_field(log_schema::log_schema().timestamp_key())
                .and_then(|v| v.as_timestamp())
                .copied()
        }

        Event::Metric(metric) => metric.timestamp()
    }.map(|ts| ts.timestamp_millis())
}

fn get_key(event: &Event, key_field: &Option<String>) -> Option<Bytes> {
    key_field.as_ref()
        .and_then(|key_field| match event {
            Event::Log(log) => {
                log.get_field(key_field)
                    .map(|value| value.as_bytes())
            }
            Event::Metric(metric) => {
                metric.tags
                    .get(key_field)
                    .map(|value| value.clone().into())
            }
        })
}

#[cfg(test)]
mod tests {
    use crate::config::test_generate_config;
    use super::*;

    #[test]
    fn generate_config() {
        test_generate_config::<KafkaSinkConfig>()
    }
}