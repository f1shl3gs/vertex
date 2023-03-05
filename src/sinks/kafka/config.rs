use std::collections::HashMap;
use std::time::Duration;

use codecs::encoding::EncodingConfig;
use configurable::configurable_component;
use framework::batch::{BatchConfig, NoDefaultBatchSettings};
use framework::config::{DataType, SinkConfig, SinkContext};
use framework::{Healthcheck, Sink};
use futures_util::FutureExt;
use rdkafka::ClientConfig;

use super::sink::health_check;
use crate::common::kafka::{KafkaAuthConfig, KafkaCompression};

pub const QUEUE_MIN_MESSAGES: u64 = 100000;

#[configurable_component(sink, name = "kafka")]
#[derive(Clone, Debug)]
pub struct KafkaSinkConfig {
    /// A comma-separated list of host and port pairs that are the addresses of
    /// the Kafka brokers in a "bootstrap" Kafka cluster that a Kafka client
    /// connects to initially ot bootstrap itself.
    #[configurable(required, format = "ip-address", example = "127.0.0.1:9092")]
    pub bootstrap_servers: Vec<String>,

    /// The Kafka topic name to write events to
    #[configurable(required)]
    pub topic: String,

    /// The log field name or tags key to use for the topic key. If the field
    /// does not exist in the log or in tags, a blank value will be used. If
    /// unspecified, the key is not sent. Kafka uses a hash of the key to choose
    /// the partition or uses round-robin if the record has no key.
    pub key_field: Option<String>,
    /// Configures the encoding specific sink behavior.
    pub encoding: EncodingConfig,

    /// These batching options will `not` override librdkafka_options values
    #[serde(default)]
    pub batch: BatchConfig<NoDefaultBatchSettings>,

    /// The compression strategy used to compress the encoded event
    /// data before transmission.
    #[serde(default)]
    pub compression: KafkaCompression,

    #[serde(default = "default_auth")]
    pub auth: KafkaAuthConfig,

    /// Default timeout for network requests
    #[serde(default = "default_socket_timeout")]
    #[serde(with = "humanize::duration::serde")]
    pub socket_timeout: Duration,

    /// Local message timeout
    #[serde(default = "default_message_timeout")]
    #[serde(with = "humanize::duration::serde")]
    pub message_timeout: Duration,

    /// Advanced options. See librdkafka decumentation for details.
    /// https://github.com/edenhill/librdkafka/blob/master/CONFIGURATION.md
    #[serde(default)]
    pub librdkafka_options: HashMap<String, String>,

    /// The log field name to use for the Kafka headers. If omitted,
    /// no headers will be written.
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

const fn default_auth() -> KafkaAuthConfig {
    KafkaAuthConfig {
        sasl: None,
        tls: None,
    }
}

/// Used to determine the options to set in configs, since both kafka consumers and providers have
/// unique options, they use the same struct, and the error if given the wrong options.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, PartialOrd, PartialEq)]
pub enum KafkaRole {
    Consumer,
    Producer,
}

impl KafkaSinkConfig {
    pub fn to_rdkafka(&self, role: KafkaRole) -> crate::Result<ClientConfig> {
        let mut config = ClientConfig::new();
        config
            .set("bootstrap.servers", &self.bootstrap_servers.join(","))
            .set("compression.codec", &to_string(self.compression))
            .set(
                "socket.timeout.ms",
                &self.socket_timeout.as_millis().to_string(),
            )
            .set("statistics.interval.ms", "1000");

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

#[async_trait::async_trait]
#[typetag::serde(name = "kafka")]
impl SinkConfig for KafkaSinkConfig {
    async fn build(&self, _cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        let sink = super::sink::KafkaSink::new(self.clone())?;
        let hc = health_check(self.clone()).boxed();
        Ok((Sink::Stream(Box::new(sink)), hc))
    }

    fn input_type(&self) -> DataType {
        DataType::All
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<KafkaSinkConfig>();
    }
}
