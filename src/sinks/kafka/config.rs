use futures_util::FutureExt;
use std::collections::HashMap;
use std::time::Duration;

use event::encoding::{EncodingConfig, StandardEncodings};
use rdkafka::ClientConfig;
use serde::{Deserialize, Serialize};

use crate::batch::{BatchConfig, NoDefaultBatchSettings};
use crate::common::kafka::{KafkaAuthConfig, KafkaCompression};
use crate::config::{
    deserialize_duration, serialize_duration, DataType, GenerateConfig, HealthCheck, SinkConfig,
    SinkContext,
};
use crate::sinks::kafka::sink::health_check;
use crate::sinks::Sink;

pub const QUEUE_MIN_MESSAGES: u64 = 100000;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KafkaSinkConfig {
    pub bootstrap_servers: String,
    pub topic: String,
    pub key_field: Option<String>,
    pub encoding: EncodingConfig<StandardEncodings>,

    /// These batching options will `not` override librdkafka_options values
    #[serde(default)]
    pub batch: BatchConfig<NoDefaultBatchSettings>,
    #[serde(default)]
    pub compression: KafkaCompression,

    pub auth: KafkaAuthConfig,
    #[serde(default = "default_socket_timeout")]
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
    pub socket_timeout: Duration,
    #[serde(default = "default_message_timeout")]
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
    pub message_timeout: Duration,
    #[serde(default)]
    pub librdkafka_options: HashMap<String, String>,
    pub headers_field: Option<String>,
}

const fn default_socket_timeout() -> Duration {
    // default in librdkafka
    Duration::from_millis(60000)
}

const fn default_message_timeout() -> Duration {
    // default in libkafka
    Duration::from_millis(300000)
}

/// Used to determine the options to set in configs, since both kafka consumers and providers have
/// unique options, they use the same struct, and the error if given the wrong options.
#[derive(Debug, PartialOrd, PartialEq)]
pub enum KafkaRole {
    Consumer,
    Producer,
}

impl KafkaSinkConfig {
    pub fn to_rdkafka(&self, role: KafkaRole) -> crate::Result<ClientConfig> {
        let mut config = ClientConfig::new();
        config
            .set("bootstrap.servers", &self.bootstrap_servers)
            .set("compression.codec", &to_string(self.compression))
            .set(
                "socket.timeout.ms",
                &self.socket_timeout.as_millis().to_string(),
            )
            .set(
                "message.timeout.ms",
                &self.message_timeout.as_millis().to_string(),
            )
            .set("statistics.inerval.ms", "1000")
            .set("queue.min.messages", QUEUE_MIN_MESSAGES.to_string());

        self.auth.apply(&mut config)?;

        // All batch options are producer only.
        if role == KafkaRole::Producer {
            if let Some(timeout) = self.batch.timeout {
                // Delay in milliseconds to wait for messages in the producer queue to
                // accumulate before constructing message batches(MessageSets) to transmit
                // to brokers. A higher value allows larger and more effective(less overhead,
                // improved compression) batches of messages to accumulate at the expense of
                // increased message delivery latency.
                let key = "queue.buffering.max.ms";
                if let Some(val) = self.librdkafka_options.get(key) {
                    return Err(format!(
                        "Batching setting `batch.timeout` sets `librdkafka_options.{}={}`.\
                        The config already sets this as `librdkafka_options.queue.buffering.max.ms={}`.\
                        Please delete one", key, timeout.as_millis(), val
                    ).into());
                }

                debug!(
                    message = "Applying batch option as librdkafka option",
                    librdkafka_option = key,
                    batch_option = "timeout",
                    value = timeout.as_millis() as u64
                );

                config.set(key, &(timeout.as_millis()).to_string());
            }

            if let Some(value) = self.batch.max_events {
                // Maximum number of messages batched in one MessageSet. The total MessageSet size
                // is also limited by batch.size and message.max.bytes.
                let key = "batch.num.messages";
                if let Some(val) = self.librdkafka_options.get(key) {
                    return Err(format!(
                        "Batching setting `batch.max_events` sets `librdkafka_options.{}={}`.\
                        The config already sets this as `librdkafka_options.batch.num.messages={}`.\
                        Please delete one.",
                        key, value, val
                    )
                    .into());
                }

                debug!(
                    message = "Applying batch option as librdkafka option",
                    librdkafka_option = key,
                    batch_option = "max_events",
                    value
                );
                config.set(key, &value.to_string());
            }

            if let Some(value) = self.batch.max_bytes {
                // Maximum size(in bytes) of all messages batched in one MessageSet, including
                // protocol framing overhead. This limit is applied after the first message has
                // been added to the batch, regardless of the first message's size, this is to
                // ensure that messages that exceed batch.size are produced. The total MessageSet
                // size is also limited by batch.num.messages and message.max.bytes.
                let key = "batch.size";
                if let Some(val) = self.librdkafka_options.get(key) {
                    return Err(format!(
                        "Batching setting `batch.max_bytes` sets `librdkafka_options.{}={}`.\
                        The config already sets this as `librdkafka_options.batch.size={}`.\
                        Please delete one",
                        key, value, val
                    )
                    .into());
                }

                debug!(
                    message = "Applying batch option as librdkafka option",
                    librdkafka_option = key,
                    batch_option = "max_bytes",
                    value
                );
                config.set(key, &value.to_string());
            }
        }

        for (key, value) in self.librdkafka_options.iter() {
            debug!(
                message = "Setting librdkafka option",
                option = %key,
                %value
            );

            config.set(key.as_str(), value.as_str());
        }

        Ok(config)
    }
}

fn to_string(value: impl serde::Serialize) -> String {
    let value = serde_json::to_value(value).unwrap();
    value.as_str().unwrap().into()
}

impl GenerateConfig for KafkaSinkConfig {
    fn generate_config() -> String {
        format!(r#"
# A comma-separated list of host and port pairs that are the addresses of
# the Kafka brokers in a "bootstrap" Kafka cluster that a Kafka client
# connects to initially ot bootstrap itself.
#
bootstrap_servers: 127.0.0.1:9092,127.0.0.2:9092

# The Kafka topic name to write events to
#
topic: some-topic

# The log field name or tags key to use for the topic key. If the field
# does not exist in the log or in tags, a blank value will be used. If
# unspecified, the key is not sent. Kafka uses a hash of the key to choose
# the partition or uses round-robin if the record has no key.
#
# key_field: user_id

# Configures the encoding specific sink behavior.
#
# encoding: TODO

# Configures the sink batching behavior
#
# batch:
{}

# Configures the sink specific buffer behavior.
#
# buffer:
#  # The buffer's type and storage mechanism.
#  #
#  # Availabels:
#  # disk:     Stores the sink's buffer on disk. This is less performant,
#  #           but durable. Data will not be lost between restarts. Will
#  #           also hold data in memory to enhance performance.
#  #           WARNING: This may stall the sink if disk performance isn't on
#  #           par with the throughput.
#  # memory:   Stores the sinks buffer in memory. this is more performant, but
#  #           less durable. Data will be lost if Vertex restarted forcefully.
#  type: memory
#
#  # The maximum number of events allowed in the buffer.
#  #
#  # max_events: 512
#
#  # The maximum size of the buffer on the disk.
#  #
#  # Used only when buffer type is disk
#  # max_size:  128M

#

"#, BatchConfig::<NoDefaultBatchSettings>::generate_commented_with_indent(2))

        /*serde_yaml::to_value(Self {
            bootstrap_servers: "10.14.22.123.9092,10.14.22.123.9092".to_string(),
            topic: "some-topic".to_string(),
            key_field: Some("uid".to_string()),
            encoding: StandardEncodings::Json.into(),
            batch: Default::default(),
            compression: KafkaCompression::None,
            auth: Default::default(),
            socket_timeout: default_socket_timeout(),
            message_timeout: default_message_timeout(),
            librdkafka_options: Default::default(),
            headers_field: None,
        })
        .unwrap()*/
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "kafka")]
impl SinkConfig for KafkaSinkConfig {
    async fn build(&self, ctx: SinkContext) -> crate::Result<(Sink, HealthCheck)> {
        let sink = super::sink::KafkaSink::new(self.clone(), ctx.acker())?;
        let hc = health_check(self.clone()).boxed();
        Ok((Sink::Stream(Box::new(sink)), hc))
    }

    fn input_type(&self) -> DataType {
        DataType::Any
    }

    fn sink_type(&self) -> &'static str {
        "kafka"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::test_generate_config;

    #[test]
    fn generate_config() {
        test_generate_config::<KafkaSinkConfig>();
    }
}
