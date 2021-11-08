use std::collections::BTreeMap;
use std::time::Duration;
use futures::prelude::stream::BoxStream;
use nom::combinator::value;
use rdkafka::{ClientConfig, ClientContext, Statistics};
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::producer::FutureProducer;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use buffers::Acker;
use event::Event;

use crate::batch::BatchConfig;
use crate::common::kafka::{KafkaAuthConfig, KafkaCompression, KafkaRole};
use crate::config::{DataType, deserialize_duration, GenerateConfig, HealthCheck, serialize_duration, SinkConfig, SinkContext};
use crate::sinks::{Sink, StreamSink};
use crate::template::Template;


#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KafkaSinkConfig {
    pub bootstrap_servers: String,
    pub topic: String,
    pub key_field: Option<String>,
    pub compression: KafkaCompression,
    #[serde(default = "default_socket_timeout")]
    #[serde(deserialize_with = "deserialize_duration", serialization_with = "serialize_duration")]
    pub socket_timeout: chrono::Duration,
    #[serde(defalut = "default_message_timeout")]
    #[serde(deserialize_with = "deserialize_duration", serialization_with = "serialize_duration")]
    pub message_timeout: chrono::Duration,
    pub auth: KafkaAuthConfig,
    pub batch: BatchConfig,
    pub librdkafka_options: BTreeMap<String, String>,
}

const fn default_socket_timeout() -> chrono::Duration {
    // default in librdkafka
    chrono::Duration::milliseconds(60000)
}

const fn default_message_timeout() -> chrono::Duration {
    // default in librdkafka
    chrono::Duration::milliseconds(300000)
}

impl GenerateConfig for KafkaSinkConfig {
    fn generate_config() -> serde_yaml::Value {
        serde_yaml::Value::try_from(Self {
            bootstrap_servers: "127.0.0.1:9092".to_string(),
            topic: "some_topic".to_string(),
            key_field: Some("uid".to_string()),
            compression: KafkaCompression::None,
            socket_timeout: default_socket_timeout(),
            message_timeout: default_message_timeout(),
        }).unwrap()
    }
}

inventory::submit! {
    SourceDescription::new::<KafkaSinkConfig>("kafka")
}

#[async_trait::async_trait]
#[typetag::serde(name = "kafka")]
impl SinkConfig for KafkaSinkConfig {
    async fn build(&self, ctx: SinkContext) -> crate::Result<(Sink, HealthCheck)> {
        let sink = KafkaSink::new(self.clone(), ctx.acker)?;
        let health_check = healthcheck(self.clone());

        Ok((Sink::Stream(Box::new(sink)), health_check))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn sink_type(&self) -> &'static str {
        "kafka"
    }
}

impl KafkaSinkConfig {
    fn to_rdkafka(&self, role: KafkaRole) -> crate::Result<ClientConfig> {
        let mut client_config = ClientConfig::new();
        let codec = serde_json::to_value(self.compression)?;

        client_config
            .set("bootstrap.servers", &self.bootstrap_servers)
            .set("compression.codec", codec.as_str().unwrap().into())
            .set("socket.timeout.ms", &self.socket_timeout.to_string())
            .set("message.timeout.ms", &self.message_timeout.to_string())
            .set("statistics.interval.ms", "1000")
            .set("queued.min.messages", QUEUE_MIN_MESSAGE.to_string());

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
                    value,
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
                    ).into_boxed_str());
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

pub struct KafkaStatisticsContext;

impl ClientContext for KafkaStatisticsContext {
    fn stats(&self, statistics: Statistics) {

    }
}

struct KafkaSink {
    acker: Acker,
    topic: Template,
    key_field: Option<String>,
    headers_field: Option<String>,

    producer: FutureProducer<KafkaStatisticsContext>
}

impl KafkaSink {
    fn new(config: KafkaSinkConfig, acker: Acker) -> crate::Result<Self> {}
}

#[async_trait::async_trait]
impl StreamSink for KafkaSink {
    async fn run(&mut self, input: BoxStream<'_, Event>) -> Result<(), ()> {

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
