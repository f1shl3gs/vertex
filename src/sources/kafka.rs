use std::collections::{BTreeMap, HashMap};
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

use async_stream::stream;
use bytes::Bytes;
use chrono::{TimeZone, Utc};
use event::{log::Value, BatchNotifier, Event};
use framework::codecs::decoding::{DecodingConfig, DeserializerConfig};
use framework::codecs::StreamDecodingError;
use framework::codecs::{BytesDecoderConfig, BytesDeserializerConfig};
use framework::config::{
    deserialize_duration, serialize_duration, DataType, GenerateConfig, Output, SourceConfig,
    SourceContext, SourceDescription,
};
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::source::util::OrderedFinalizer;
use framework::{codecs, Error, Source};
use futures::{FutureExt, StreamExt};
use log_schema::log_schema;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::{BorrowedMessage, Headers};
use rdkafka::{ClientConfig, Message, TopicPartitionList};
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use tokio_util::codec::FramedRead;

use crate::common::kafka::{
    KafkaAuthConfig, KafkaEventReceived, KafkaOffsetUpdateFailed, KafkaStatisticsContext,
};

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

fn default_decoding() -> DeserializerConfig {
    BytesDeserializerConfig::new().into()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
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
    #[serde(default = "default_decoding")]
    decoding: DeserializerConfig,
    #[serde(default)]
    acknowledgement: bool,

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
        let mut client_config = ClientConfig::new();

        client_config
            .set("group.id", &self.group)
            .set("bootstrap.servers", &self.bootstrap_servers)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true")
            .set("auto.offset.reset", &self.auto_offset_reset)
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
            .set(
                "auto.commit.interval.ms",
                self.commit_interval.as_millis().to_string(),
            )
            .set("enable.auto.offset.store", "false")
            .set("statistics.interval.ms", "1000")
            .set("client.id", "vertex");

        self.auth.apply(&mut client_config)?;

        if let Some(ref options) = self.librdkafka_options {
            for (key, value) in options {
                client_config.set(key, value);
            }
        }

        let consumer = client_config
            .create_with_context::<_, StreamConsumer<_>>(KafkaStatisticsContext)
            .context(KafkaCreateSnafu)?;

        let topics: Vec<&str> = self.topics.iter().map(|s| s.as_str()).collect();
        consumer.subscribe(&topics).context(KafkaSubscribeSnafu)?;

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
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let framing = BytesDecoderConfig::new();
        let decoder = DecodingConfig::new(framing, self.decoding.clone()).build();
        let acknowledgements = cx.globals.acknowledgements || self.acknowledgement;
        let consumer = self.create_consumer()?;

        Ok(Box::pin(kafka_source(
            consumer,
            self.key_field.clone(),
            self.topic_key.clone(),
            self.partition_key.clone(),
            self.offset_key.clone(),
            self.headers_key.clone(),
            decoder,
            cx.output,
            cx.shutdown,
            acknowledgements,
        )))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }

    fn source_type(&self) -> &'static str {
        "kafka"
    }
}

async fn kafka_source(
    consumer: StreamConsumer<KafkaStatisticsContext>,
    key_field: String,
    topic_key: String,
    partition_key: String,
    offset_key: String,
    headers_key: String,
    decoder: codecs::Decoder,
    mut output: Pipeline,
    shutdown: ShutdownSignal,
    acknowledgements: bool,
) -> Result<(), ()> {
    let consumer = Arc::new(consumer);
    let shutdown = shutdown.shared();
    let mut finalizer = acknowledgements
        .then(|| OrderedFinalizer::new(shutdown.clone(), mark_done(Arc::clone(&consumer))));
    let mut stream = consumer.stream().take_until(shutdown);
    let schema = log_schema();

    while let Some(message) = stream.next().await {
        match message {
            Err(err) => {
                // TODO: metric
                warn!(
                    message = "Failed to read message",
                    ?err,
                    internal_log_rate_secs = 10
                );
            }

            Ok(msg) => {
                emit!(&KafkaEventReceived {
                    byte_size: msg.payload_len()
                });

                let payload = match msg.payload() {
                    None => continue, // skip messages with empty payload,
                    Some(payload) => payload,
                };

                // Extract timestamp from kafka message
                let timestamp = msg
                    .timestamp()
                    .to_millis()
                    .and_then(|millis| Utc.timestamp_millis_opt(millis).latest())
                    .unwrap_or_else(Utc::now);

                let msg_key = msg
                    .key()
                    .map(|key| Value::from(String::from_utf8_lossy(key).to_string()))
                    .unwrap_or(Value::Null);

                let mut headers_map = BTreeMap::new();
                if let Some(headers) = msg.headers() {
                    // Using index-based for loop because rdkafka's `Headers` trait
                    // does not provide Iterator-based API
                    for i in 0..headers.count() {
                        if let Some(header) = headers.get(i) {
                            headers_map.insert(
                                header.0.to_string(),
                                Bytes::from(header.1.to_owned()).into(),
                            );
                        }
                    }
                }

                let msg_topic = Bytes::copy_from_slice(msg.topic().as_bytes());
                let msg_partition = msg.partition();
                let msg_offset = msg.offset();
                let key_field = &key_field;
                let topic_key = &topic_key;
                let partition_key = &partition_key;
                let offset_key = &offset_key;
                let headers_key = &headers_key;

                let payload = Cursor::new(Bytes::copy_from_slice(payload));
                let mut stream = FramedRead::new(payload, decoder.clone());

                let mut stream = stream! {
                    loop {
                        match stream.next().await {
                            Some(Ok((events, _))) => {
                                for mut event in events {
                                    if let Event::Log(ref mut log) = event {
                                        log.insert_tag(schema.source_type_key(), "kafka");

                                        log.insert_field(schema.timestamp_key(), timestamp);
                                        log.insert_field(key_field, msg_key.clone());
                                        log.insert_field(topic_key, msg_topic.clone());
                                        log.insert_field(partition_key, msg_partition);
                                        log.insert_field(offset_key, msg_offset);
                                        log.insert_field(headers_key, headers_map.clone());
                                    }

                                    yield event;
                                }
                            },

                            Some(Err(err)) => {
                                // Error is logged by `codecs::Decoder`, no further handling
                                // is needed here.
                                if !err.can_continue() {
                                    break;
                                }
                            },

                            None => break,
                        }
                    }
                }
                .boxed();

                match &mut finalizer {
                    Some(finalizer) => {
                        let (batch, receiver) = BatchNotifier::new_with_receiver();
                        let mut stream = stream.map(|event| event.with_batch_notifier(&batch));
                        match output.send_all(&mut stream).await {
                            Ok(_) => {
                                // Drop stream to avoid borrowing `msg`: [...] borrow might be used
                                // here, when `stream` is dropped and runs the destructor [...].
                                drop(stream);
                                finalizer.add(msg.into(), receiver);
                            }

                            Err(err) => {
                                error!(
                                    message = "Error sending to sink",
                                    %err
                                );
                            }
                        }
                    }

                    None => match output.send_all(&mut stream).await {
                        Ok(_) => {
                            if let Err(err) =
                                consumer.store_offset(msg.topic(), msg.partition(), msg.offset())
                            {
                                emit!(&KafkaOffsetUpdateFailed { error: err })
                            }
                        }
                        Err(err) => error!(message = "Error sending to sink", %err),
                    },
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

    pub(super) fn make_config(
        bootstrap_servers: &str,
        topic: &str,
        group: &str,
    ) -> KafkaSourceConfig {
        KafkaSourceConfig {
            bootstrap_servers: bootstrap_servers.into(),
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
            acknowledgement: false,
            decoding: default_decoding(),
        }
    }

    #[tokio::test]
    async fn consumer_create_ok() {
        let config = make_config(BOOTSTRAP_SERVER, "topic", "group");
        assert!(config.create_consumer().is_ok())
    }

    #[tokio::test]
    async fn consumer_create_incorrect_auto_offset_reset() {
        let conf = KafkaSourceConfig {
            auto_offset_reset: "incorrect-auto-offset-reset".to_string(),
            ..make_config(BOOTSTRAP_SERVER, "topic", "group")
        };
        assert!(conf.create_consumer().is_err())
    }
}

#[cfg(all(test, feature = "integration-tests-kafka"))]
mod integration_tests {
    use crate::sources::kafka::kafka_source;
    use chrono::{SubsecRound, Utc};
    use event::{log::Value, EventStatus};
    use framework::{codecs, Pipeline, ShutdownSignal};
    use log_schema::log_schema;
    use rdkafka::config::FromClientConfig;
    use rdkafka::consumer::{BaseConsumer, Consumer};
    use rdkafka::message::OwnedHeaders;
    use rdkafka::producer::{FutureProducer, FutureRecord};
    use rdkafka::{ClientConfig, Offset, TopicPartitionList};
    use std::time::Duration;
    use testcontainers::images::generic::{GenericImage, Stream, WaitFor};
    use testcontainers::{clients, Docker, RunArgs};
    use testify::collect_n;
    use testify::random::random_string;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn consume_with_ack() {
        let network = format!("test-source-kafka-{}", random_string(10));
        let cli = clients::Cli::default();

        let image = GenericImage::new("bitnami/zookeeper:3.7")
            .with_env_var("ALLOW_ANONYMOUS_LOGIN", "yes")
            .with_wait_for(WaitFor::LogMessage {
                message: "Started AdminServer on address".to_string(),
                stream: Stream::StdOut,
            });
        let zk_container_name = format!("zookeeper-{}", random_string(10));
        let args = RunArgs::default()
            .with_network(network.clone())
            .with_name(&zk_container_name);
        let _zk = cli.run_with_args(image, args);

        let image = GenericImage::new("wurstmeister/kafka:2.13-2.7.0")
            .with_env_var(
                "KAFKA_ZOOKEEPER_CONNECT",
                format!("{}:2181", &zk_container_name),
            )
            .with_env_var("KAFKA_ADVERTISED_HOST_NAME", "127.0.0.1")
            .with_wait_for(WaitFor::LogMessage {
                message: "started (kafka.server.KafkaServer)".to_string(),
                stream: Stream::StdOut,
            });

        let random_port = testify::pick_unused_local_port();
        let args = RunArgs::default()
            .with_network(network)
            .with_mapped_port((random_port, 9092));
        let _kafka = cli.run_with_args(image, args);

        consume_event(format!("localhost:{}", random_port), true).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn consume_without_ack() {
        // NOTE: The code bellow is duplicated, but we just can't be moved into an
        // individual function(e.g. setup).
        let network = format!("test-source-kafka-{}", random_string(10));
        let cli = clients::Cli::default();

        let image = GenericImage::new("bitnami/zookeeper:3.7")
            .with_env_var("ALLOW_ANONYMOUS_LOGIN", "yes")
            .with_wait_for(WaitFor::LogMessage {
                message: "Started AdminServer on address".to_string(),
                stream: Stream::StdOut,
            });
        let zk_container_name = format!("zookeeper-{}", random_string(10));
        let args = RunArgs::default()
            .with_network(network.clone())
            .with_name(&zk_container_name);
        let _zk = cli.run_with_args(image, args);

        let image = GenericImage::new("wurstmeister/kafka:2.13-2.7.0")
            .with_env_var(
                "KAFKA_ZOOKEEPER_CONNECT",
                format!("{}:2181", &zk_container_name),
            )
            .with_env_var("KAFKA_ADVERTISED_HOST_NAME", "127.0.0.1")
            .with_wait_for(WaitFor::LogMessage {
                message: "started (kafka.server.KafkaServer)".to_string(),
                stream: Stream::StdOut,
            });

        let random_port = testify::pick_unused_local_port();
        let args = RunArgs::default()
            .with_network(network)
            .with_mapped_port((random_port, 9092));
        let _kafka = cli.run_with_args(image, args);

        consume_event(format!("localhost:{}", random_port), false).await;
    }

    async fn consume_event(servers: String, ack: bool) {
        let count = 10;
        let topic = format!("test-topic-{}", random_string(10));
        let group = format!("test-group-{}", random_string(10));

        let now = Utc::now();
        let config = super::tests::make_config(&servers, &topic, &group);

        send_events(
            &servers,
            topic.clone(),
            count,
            "key",
            "payload",
            now.timestamp_millis(),
            "header_key",
            "header_value",
        )
        .await;

        let (trigger_shutdown, shutdown, shutdown_done) = ShutdownSignal::new_wired();
        let (tx, rx) = Pipeline::new_test_finalize(EventStatus::Delivered);
        let consumer = config.create_consumer().unwrap();
        tokio::spawn(kafka_source(
            consumer,
            config.key_field,
            config.topic_key,
            config.partition_key,
            config.offset_key,
            config.headers_key,
            codecs::Decoder::default(),
            tx,
            shutdown,
            ack,
        ));

        let events = collect_n(rx, 10).await;

        // wait mark_done complete!?
        tokio::time::sleep(Duration::from_secs(1)).await;
        drop(trigger_shutdown);
        shutdown_done.await;

        // 1. Make sure the test did consume `count` message
        let client: BaseConsumer = client_config(Some(&group), &servers);
        client.subscribe(&[&topic]).expect("Subscribing failed");

        let mut tpl = TopicPartitionList::new();
        tpl.add_partition(&topic, 0);
        let tpl = client
            .committed_offsets(tpl, Duration::from_secs(1))
            .expect("Getting committed offsets failed");

        assert_eq!(
            tpl.find_partition(&topic, 0)
                .expect("TPL is missing topic")
                .offset(),
            Offset::from_raw(count as i64)
        );

        // 2. assert every message's timestamp and content
        assert_eq!(events.len(), count);
        for (i, event) in events.into_iter().enumerate() {
            let log = event.as_log();
            let message = log.get_field(log_schema().message_key()).unwrap();
            let timestamp = log.get_field(log_schema().timestamp_key()).unwrap();

            assert_eq!(*message, format!("{} payload", i).into());
            assert_eq!(*timestamp, Value::from(now.trunc_subsecs(3)));
            assert_eq!(*log.get_field("topic").unwrap(), Value::from(topic.clone()));
        }
    }

    fn client_config<T: FromClientConfig>(group: Option<&str>, bootstrap_servers: &str) -> T {
        let mut client = ClientConfig::new();
        client.set("bootstrap.servers", bootstrap_servers);
        client.set("produce.offset.report", "true");
        client.set("message.timeout.ms", "5000");
        client.set("auto.commit.interval.ms", "1");
        if let Some(group) = group {
            client.set("group.id", group);
        }
        client.create().expect("Producer creation error")
    }

    async fn send_events(
        servers: &str,
        topic: String,
        count: usize,
        key: &str,
        text: &str,
        timestamp: i64,
        header_key: &str,
        header_value: &str,
    ) {
        let producer: FutureProducer = client_config(None, servers);

        for i in 0..count {
            let payload = format!("{} {}", i, text);
            let record = FutureRecord::to(&topic)
                .payload(&payload)
                .key(key)
                .timestamp(timestamp)
                .headers(OwnedHeaders::new().add(header_key, header_value));

            match producer.send(record, Duration::from_secs(3)).await {
                Ok((_partition, _offset)) => {
                    // dbg!("partition: {}, offset: {}", partition, offset);
                }
                Err(err) => {
                    panic!(
                        "Cannot send event to Kafka, server: {}, err: {:?}",
                        servers, err
                    )
                }
            }
        }
    }
}
