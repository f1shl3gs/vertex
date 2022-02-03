use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use chrono::{TimeZone, Utc};
use event::{LogRecord, Value};
use futures::{FutureExt, StreamExt};
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::{BorrowedMessage, Headers};
use rdkafka::{ClientConfig, Message, TopicPartitionList};
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};

use crate::common::kafka::{KafkaAuthConfig, KafkaEventReceived, KafkaStatisticsContext};
use crate::config::{
    deserialize_duration, serialize_duration, DataType, GenerateConfig, Output, SourceConfig,
    SourceContext, SourceDescription,
};
use crate::pipeline::Pipeline;
use crate::shutdown::ShutdownSignal;
use crate::sources::Source;
use crate::Error;

fn default_auto_offset_reset() -> String {
    "largest".to_string()
}

const fn default_session_timeout() -> Duration {
    Duration::from_secs(10)
}

const fn default_socket_timeout() -> Duration {
    Duration::from_secs(60)
}

const fn default_fetch_wait_max() -> Duration {
    Duration::from_millis(100)
}

const fn default_commit_interval() -> Duration {
    Duration::from_secs(5)
}

fn default_key_field() -> String {
    "message_key".to_string()
}

fn default_topic_key() -> String {
    "topic".to_string()
}

fn default_partition_key() -> String {
    "partition".to_string()
}

fn default_offset_key() -> String {
    "offset".to_string()
}

fn default_headers_key() -> String {
    "headers".to_string()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct KafkaSourceConfig {
    bootstrap_servers: String,
    topics: Vec<String>,
    group: String,
    #[serde(default = "default_auto_offset_reset")]
    auto_offset_reset: String,
    #[serde(default = "default_session_timeout")]
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
    session_timeout: Duration,
    #[serde(default = "default_socket_timeout")]
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
    socket_timeout: Duration,
    #[serde(default = "default_fetch_wait_max")]
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
    fetch_wait_max: Duration,
    #[serde(default = "default_commit_interval")]
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
    commit_interval: Duration,
    #[serde(default = "default_key_field")]
    key_field: String,
    #[serde(default = "default_topic_key")]
    topic_key: String,
    #[serde(default = "default_partition_key")]
    partition_key: String,
    #[serde(default = "default_offset_key")]
    offset_key: String,
    #[serde(default = "default_headers_key")]
    headers_key: String,
    #[serde(flatten)]
    auth: KafkaAuthConfig,

    librdkafka_options: Option<HashMap<String, String>>,
}

impl GenerateConfig for KafkaSourceConfig {
    fn generate_config() -> String {
        format!(
            r#"
# A comma-separated list of host and port pairs that are the address
# of the Kafka brokers in a "bootstrap" Kafka cluster that a Kafka
# client connects to initially to bootstrap itself.
bootstrap_servers: 10.14.22.123:9092,10.14.23.332:9092

# The Kafka topics names to read eevnts from. Regex is supported if
# the topic begins with `^`.
topics:
- ^(prefix1|prefix2)-.*
- topic1
- topic2

# The consumer group name to be used to consume events from Kafka.
group: foo

# If offsets for consumer group do not exist, set them using this
# strategy. See the librdkafka documentation for the
# "auto.offset.reset" option for further clarification.
#
# auto_offset_reset: "largest"

# The Kafka session timeout.
# session_timeout: {}s

# Default timeout for network requests.
socket_timeout: {}s

# Maximum time the broker may wait to fill the response.
fetch_wait_max: {}s

# The frequency that the consumer offsets are committed(written) to
# offset storage.
# commit_interval: {}s

# The log field name to use for the Kafka message key.
# key_field: {}

# The log field name to use for the Kafka topic.
# topic_key: {}

# The log field name to use for the Kafka partition name.
# partition_key: {}

# The log field name to use for the Kafka offset
# offset_key: {}

# The log field name to use for the Kafka headers.
# headers_key: {}

# auth:

# Advanced options. See librdkafka documentation for more details.
# https://github.com/edenhill/librdkafka/blob/master/CONFIGURATION.md
# librdkafka_options:
#   foo: bar

"#,
            default_session_timeout().as_secs(),
            default_socket_timeout().as_secs(),
            default_fetch_wait_max().as_secs(),
            default_commit_interval().as_secs(),
            default_key_field(),
            default_topic_key(),
            default_partition_key(),
            default_offset_key(),
            default_headers_key(),
        )
    }
}

inventory::submit! {
    SourceDescription::new::<KafkaSourceConfig>("kafka")
}

#[derive(Debug, Snafu)]
enum BuildError {
    #[snafu(display("Could not create Kafka consumer: {}", source))]
    KafkaCreateError { source: rdkafka::error::KafkaError },
    #[snafu(display("Could not subscribe to Kafka topics: {}", source))]
    KafkaSubscribeError { source: rdkafka::error::KafkaError },
}

impl KafkaSourceConfig {
    fn create_consumer(&self) -> Result<StreamConsumer<KafkaStatisticsContext>, Error> {
        let mut conf = ClientConfig::new();
        conf.set("group.id", self.group.to_string())
            .set("bootstrap.servers", self.bootstrap_servers.to_string())
            .set("auto.offset.reset", self.auto_offset_reset.to_string())
            .set(
                "session.timeout.ms",
                self.session_timeout.as_millis().to_string(),
            )
            .set(
                "socket.timeout.ms",
                self.socket_timeout.as_millis().to_string(),
            )
            .set(
                "fetch.wait.max.ms",
                self.fetch_wait_max.as_millis().to_string(),
            )
            .set("enable.partition.eof", "false")
            .set("enable.auto.commit", "true")
            .set(
                "auto.commit.interval.ms",
                self.commit_interval.as_millis().to_string(),
            )
            .set("enable.auto.offset.store", "false")
            .set("statistics.interval.ms", "1000")
            .set("client.id", "vertex");

        self.auth.apply(&mut conf)?;

        if let Some(ref options) = self.librdkafka_options {
            for (key, value) in options {
                conf.set(key, value);
            }
        }

        let consumer = conf
            .create_with_context::<_, StreamConsumer<_>>(KafkaStatisticsContext)
            .context(KafkaCreateError)?;
        let topics: Vec<&str> = self.topics.iter().map(|s| s.as_str()).collect();
        consumer.subscribe(&topics).context(KafkaSubscribeError)?;

        Ok(consumer)
    }
}

#[derive(Debug)]
struct FinalizerEntry {
    topic: String,
    partition: i32,
    offset: i64,
}

impl<'a> From<BorrowedMessage<'a>> for FinalizerEntry {
    fn from(msg: BorrowedMessage<'a>) -> Self {
        Self {
            topic: msg.topic().into(),
            partition: msg.partition(),
            offset: msg.offset(),
        }
    }
}

fn mark_done(consumer: Arc<StreamConsumer<KafkaStatisticsContext>>) -> impl Fn(FinalizerEntry) {
    move |entry| {
        // Would like to use `consumer.store_offset` here, but types don't allow it
        let mut tpl = TopicPartitionList::new();
        tpl.add_partition(&entry.topic, entry.partition)
            .set_offset(rdkafka::Offset::from_raw(entry.offset + 1))
            .expect("Setting offset failed");

        if let Err(err) = consumer.store_offsets(&tpl) {
            warn!(message = "Unable to update consumer offset", ?err);
        }
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "kafka")]
impl SourceConfig for KafkaSourceConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        let consumer = self.create_consumer()?;

        Ok(Box::pin(drain(
            consumer,
            self.key_field.clone(),
            self.topic_key.clone(),
            self.partition_key.clone(),
            self.offset_key.clone(),
            self.headers_key.clone(),
            ctx.output,
            ctx.shutdown,
        )))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }

    fn source_type(&self) -> &'static str {
        "kafka"
    }
}

async fn drain(
    consumer: StreamConsumer<KafkaStatisticsContext>,
    key_field: String,
    topic_key: String,
    partition_key: String,
    offset_key: String,
    headers_key: String,
    mut output: Pipeline,
    shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let consumer = Arc::new(consumer);
    let shutdown = shutdown.shared();
    // let mut finalizer = acknoledgements
    //            .then(|| OrderedFinalizer::new(shutdown.clone(), mark_done(Arc::clone(&consumer))));
    let mut stream = consumer.stream().take_until(shutdown);

    while let Some(msg) = stream.next().await {
        match msg {
            Err(err) => {
                warn!(message = "Failed to read message", ?err);
            }

            Ok(msg) => {
                emit!(&KafkaEventReceived {
                    byte_size: msg.payload_len()
                });

                let payload = match msg.payload() {
                    Some(payload) => payload,
                    None => continue,
                };
                let mut log = LogRecord::default();
                log.fields.insert("message".to_string(), payload.into());
                let timestamp = msg
                    .timestamp()
                    .to_millis()
                    .and_then(|millis| Utc.timestamp_millis_opt(millis).latest())
                    .unwrap_or_else(Utc::now);
                log.fields.insert("timestamp".to_string(), timestamp.into());
                // Add source type
                log.fields.insert("source_type".to_string(), "kafka".into());
                let msg_key = msg
                    .key()
                    .map(|key| Value::from(String::from_utf8_lossy(key).to_string()))
                    .unwrap_or(Value::Null);
                log.fields.insert(key_field.to_owned(), msg_key);
                log.fields.insert(topic_key.to_owned(), msg.topic().into());
                log.fields
                    .insert(partition_key.to_owned(), msg.partition().into());
                log.fields
                    .insert(offset_key.to_owned(), msg.offset().into());

                let mut headers = BTreeMap::new();
                if let Some(msg_headers) = msg.headers() {
                    // Using index-based for loop because rdkafka's `Headers` trait
                    // does not provide Iterator-based API
                    for i in 0..msg_headers.count() {
                        if let Some(header) = msg_headers.get(i) {
                            headers.insert(
                                header.0.to_string(),
                                Bytes::from(header.1.to_owned()).into(),
                            );
                        }
                    }
                }
                log.fields.insert(headers_key.to_owned(), headers.into());

                match output.send(log.into()).await {
                    Ok(_) => {
                        // if let Err(err) = consumer.store_offset(&msg) {
                        //     warn!(
                        //         message = "Unable to update consumer offset",
                        //         ?err
                        //     );
                        // }
                    }
                    Err(err) => {
                        warn!(message = "Error sending to sink", ?err);
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOOTSTRAP_SERVER: &str = "localhost:9091";

    fn make_config(topic: &str, group: &str) -> KafkaSourceConfig {
        KafkaSourceConfig {
            bootstrap_servers: BOOTSTRAP_SERVER.into(),
            topics: vec![topic.into()],
            group: group.into(),
            auto_offset_reset: "beginning".to_string(),
            session_timeout: Duration::from_secs(6),
            socket_timeout: Duration::from_secs(60),
            fetch_wait_max: Duration::from_millis(100),
            commit_interval: Duration::from_secs(5),
            key_field: "message_key".to_string(),
            topic_key: "topic".to_string(),
            partition_key: "partition".to_string(),
            offset_key: "offset".to_string(),
            headers_key: "headers".to_string(),
            auth: Default::default(),
            librdkafka_options: None,
        }
    }

    #[tokio::test]
    async fn consumer_create_ok() {
        let config = make_config("topic", "group");
        assert!(config.create_consumer().is_ok())
    }

    #[tokio::test]
    async fn consumer_create_incorrect_auto_offset_reset() {
        let conf = KafkaSourceConfig {
            auto_offset_reset: "incorrect-auto-offset-reset".to_string(),
            ..make_config("topic", "group")
        };
        assert!(conf.create_consumer().is_err())
    }
}

#[cfg(all(test, feature = "integration-tests"))]
mod integration_tests {}
