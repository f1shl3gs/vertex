use serde::{Deserialize, Serialize};

use crate::config::{DataType, deserialize_duration, GenerateConfig, HealthCheck, serialize_duration, SinkConfig, SinkContext};
use crate::sinks::Sink;

#[derive(Clone, Copy, Debug, Derivative, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum KafkaCompression {
    None,
    Gzip,
    Snappy,
    Lz4,
    Zstd,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KafkaConfig {
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
}

const fn default_socket_timeout() -> chrono::Duration {
    // default in librdkafka
    chrono::Duration::milliseconds(60000)
}

const fn default_message_timeout() -> chrono::Duration {
    // default in librdkafka
    chrono::Duration::milliseconds(300000)
}

#[async_trait::async_trait]
#[typetag::serde(name = "kafka")]
impl SinkConfig for KafkaConfig {
    async fn build(&self, ctx: SinkContext) -> crate::Result<(Sink, HealthCheck)> {
        todo!()
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn sink_type(&self) -> &'static str {
        "kafka"
    }
}

impl GenerateConfig for KafkaConfig {
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