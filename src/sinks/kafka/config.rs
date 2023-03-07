use codecs::encoding::EncodingConfig;
use configurable::configurable_component;
use framework::batch::{BatchConfig, SinkBatchSettings};
use framework::config::{DataType, SinkConfig, SinkContext};
use framework::{Healthcheck, Sink};
use futures_util::FutureExt;
use rskafka::client::partition::Compression;
use std::time::Duration;

use super::sink::health_check;

mod compression_serde {
    use std::borrow::Cow;
    use std::ops::Deref;

    use rskafka::client::partition::Compression;
    use serde::{Deserializer, Serializer};

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Compression, D::Error> {
        let s: Cow<str> = serde::__private::de::borrow_cow_str(deserializer)?;
        let compression = match s.deref() {
            "no" | "none" => Compression::NoCompression,
            "gzip" => Compression::Gzip,
            "lz4" => Compression::Lz4,
            "snappy" => Compression::Snappy,
            "zstd" => Compression::Zstd,

            _ => return Err(serde::de::Error::custom("unknown compression")),
        };

        Ok(compression)
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn serialize<S: Serializer>(compression: &Compression, s: S) -> Result<S::Ok, S::Error> {
        let cs = match compression {
            Compression::NoCompression => "none",
            Compression::Gzip => "gzip",
            Compression::Lz4 => "lz4",
            Compression::Snappy => "snappy",
            Compression::Zstd => "zstd",
        };

        s.serialize_str(cs)
    }
}

/// Default batch settings when the sink handles batch settings entirely on its own.
#[derive(Clone, Debug, Default)]
pub struct KafkaDefaultsBatchSettings;

impl SinkBatchSettings for KafkaDefaultsBatchSettings {
    const MAX_EVENTS: Option<usize> = Some(100);
    const MAX_BYTES: Option<usize> = Some(1024 * 1024); // 1M
    const TIMEOUT: Duration = Duration::from_millis(20);
}

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

    #[serde(default)]
    pub batch: BatchConfig<KafkaDefaultsBatchSettings>,

    /// The compression strategy used to compress the encoded event
    /// data before transmission.
    #[serde(default, with = "compression_serde")]
    #[configurable(skip)]
    pub compression: Compression,

    /// The log field name to use for the Kafka headers. If omitted,
    /// no headers will be written.
    pub headers_field: Option<String>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "kafka")]
impl SinkConfig for KafkaSinkConfig {
    async fn build(&self, _cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        let sink = super::sink::KafkaSink::new(self.clone()).await?;
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
