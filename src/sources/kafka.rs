#![allow(warnings)]

use bytes::Bytes;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use codecs::decoding::{DeserializerConfig, FramingConfig, StreamDecodingError};
use codecs::{Decoder, DecodingConfig};
use configurable::schema::{
    generate_const_string_schema, generate_one_of_schema, SchemaGenerator, SchemaObject,
};
use configurable::{configurable_component, Configurable, GenerateError};
use event::{log::Value, BatchNotifier, BatchStatus, Event, LogRecord};
use framework::config::{DataType, Output, SourceConfig, SourceContext};
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::source::util::OrderedFinalizer;
use framework::{Error, Source};
use futures::{Stream, StreamExt};
use futures_util::TryFutureExt;
use log_schema::{log_schema, LogSchema};
use rskafka::client::consumer::{StartOffset, StreamConsumerBuilder};
use rskafka::client::consumer_group::ConsumerGroup;
use rskafka::client::partition::{OffsetAt, UnknownTopicHandling};
use rskafka::client::{Client, ClientBuilder};
use rskafka::protocol::error::Error as ProtocolError;
use rskafka::protocol::messages::PartitionAssignment;
use rskafka::record::{Record, RecordAndOffset};
use rskafka::topic::Topic;
use serde::{Deserialize, Serialize};
use tokio_util::codec::FramedRead;
use tripwire::Tripwire;

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

const fn default_decoding() -> DeserializerConfig {
    DeserializerConfig::Bytes
}

const fn default_framing_message_based() -> FramingConfig {
    FramingConfig::Bytes
}

#[derive(Debug, Default, Deserialize, Serialize, Configurable)]
pub enum AutoOffsetReset {
    /// At the earlist known offset.
    ///
    /// This might be larger than 0 if some records were already deleted due to a retention policy.
    Earliest,

    /// At the latest known offset.
    ///
    /// This is helpful if you only want ot process new data.
    #[default]
    Latest,
}

impl AutoOffsetReset {
    const fn start_offset(&self) -> StartOffset {
        match self {
            AutoOffsetReset::Earliest => StartOffset::Earliest,
            AutoOffsetReset::Latest => StartOffset::Latest,
        }
    }
}

#[configurable_component(source, name = "kafka")]
#[derive(Debug)]
#[serde(deny_unknown_fields)]
struct KafkaSourceConfig {
    /// A comma-separated list of host and port pairs that are the address
    /// of the Kafka brokers in a "bootstrap" Kafka cluster that a Kafka
    /// client connects to initially to bootstrap itself.
    #[configurable(required, format = "ip-address", example = "10.14.22.123:9092")]
    bootstrap_brokers: Vec<String>,

    /// The Kafka topics names to read events from. Regex is supported if
    /// the topic begins with `^`.
    #[configurable(required)]
    topics: Vec<String>,

    /// The consumer group name to be used to consume events from Kafka.
    #[configurable(required)]
    group: String,

    /// If offsets for consumer group do not exist, set them using this
    /// strategy.
    #[serde(default)]
    auto_offset_reset: AutoOffsetReset,

    /// The Kafka session timeout.
    #[serde(default = "default_session_timeout")]
    #[serde(with = "humanize::duration::serde")]
    session_timeout: Duration,

    /// The frequency that the consumer offsets are committed(written) to
    /// offset storage.
    #[serde(default = "default_commit_interval")]
    #[serde(with = "humanize::duration::serde")]
    commit_interval: Duration,

    /// The log field name to use for the Kafka message key.
    #[serde(default = "default_key_field")]
    key_field: String,

    /// The log field name to use for the Kafka topic.
    #[serde(default = "default_topic_key")]
    topic_key: String,

    /// The log field name to use for the Kafka partition name.
    #[serde(default = "default_partition_key")]
    partition_key: String,

    /// The log field name to use for the Kafka offset
    #[serde(default = "default_offset_key")]
    offset_key: String,

    /// The log field name to use for the Kafka headers.
    #[serde(default = "default_headers_key")]
    headers_key: String,

    #[serde(default = "default_framing_message_based")]
    framing: FramingConfig,

    #[serde(default = "default_decoding")]
    decoding: DeserializerConfig,

    #[serde(default)]
    acknowledgement: bool,
}

/*

#[derive(Debug, thiserror::Error)]
enum BuildError {
    #[error("Could not create Kafka consumer: {0}")]
    KafkaCreateError(rdkafka::error::KafkaError),

    #[error("Could not subscribe to Kafka topics: {0}")]
    KafkaSubscribeError(rdkafka::error::KafkaError),
}

impl KafkaSourceConfig {
    fn create_consumer(&self) -> Result<StreamConsumer<KafkaStatisticsContext>, Error> {
        let mut client_config = ClientConfig::new();

        client_config
            .set("group.id", &self.group)
            .set("bootstrap.servers", &self.bootstrap_brokers.join(","))
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
            .create_with_context::<_, StreamConsumer<_>>(KafkaStatisticsContext::new())
            .map_err(BuildError::KafkaCreateError)?;

        let topics: Vec<&str> = self.topics.iter().map(|s| s.as_str()).collect();
        consumer
            .subscribe(&topics)
            .map_err(BuildError::KafkaSubscribeError)?;

        Ok(consumer)
    }
}
*/

/*
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
*/

#[async_trait::async_trait]
#[typetag::serde(name = "kafka")]
impl SourceConfig for KafkaSourceConfig {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let client = ClientBuilder::new(self.bootstrap_brokers.clone())
            .build()
            .await?;

        let acknowledgements = cx.globals.acknowledgements || self.acknowledgement;
        let decoder = DecodingConfig::new(self.framing.clone(), self.decoding.clone()).build();

        Ok(Box::pin(
            run(
                client,
                self.group.clone(),
                self.topics.clone(),
                self.auto_offset_reset.start_offset(),
                decoder,
                cx.output,
                cx.shutdown,
                acknowledgements,
            )
            .map_err(|err| {
                error!(message = "kafka source exit", ?err);
                ()
            }),
        ))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }
}

async fn run(
    client: Client,
    group: String,
    want_topics: Vec<String>,
    default_offset: StartOffset,
    decoder: Decoder,
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
    acknowledgements: bool,
) -> Result<(), crate::Error> {
    let client = Arc::new(client);
    let max_wait_ms = 500;

    loop {
        // list topics
        let topics = client
            .list_topics()
            .await?
            .into_iter()
            .filter(|t| want_topics.contains(&t.name))
            .collect::<Vec<_>>();

        let consumer = client.consumer_group(group.clone(), &topics).await?;
        let consumer = Arc::new(consumer);
        let notify = Arc::new(tokio::sync::Notify::new());
        let start_offsets = consumer.offsets().await?;
        let mut offsets = Vec::new();

        // consume topics
        for PartitionAssignment { topic, partitions } in consumer.assignment() {
            let starts = start_offsets.iter().find(|t| &t.name == topic);

            for partition in partitions {
                let topic = topic.to_string();
                let partition = *partition;
                let signal = Arc::clone(&notify);
                let current_offset = Arc::new(AtomicI64::new(0));
                let cli = Arc::clone(&client);
                let dec = decoder.clone();
                let mut out = output.clone();
                offsets.push(Arc::clone(&current_offset));

                let start = match starts {
                    Some(topic) => topic
                        .partitions
                        .iter()
                        .find(|p| p.partition_index == partition)
                        .map(|p| StartOffset::At(p.committed_offset))
                        .unwrap_or_else(|| default_offset),
                    None => default_offset,
                };

                tokio::spawn(async move {
                    let pc = match cli
                        .partition_client(&topic, partition, UnknownTopicHandling::Error)
                        .await
                    {
                        Ok(pc) => pc,
                        Err(err) => {
                            error!(
                                message = "create partition client failed",
                                ?err,
                                topic,
                                partition,
                            );

                            return;
                        }
                    };

                    let start = match pc.get_offset(OffsetAt::Earliest).await {
                        Ok(current) => match start {
                            StartOffset::At(committed) => {
                                if committed < current {
                                    StartOffset::At(current)
                                } else {
                                    start
                                }
                            }
                            _ => start,
                        },
                        Err(err) => {
                            error!(message = "get offset failed", ?err, topic, partition,);
                            return;
                        }
                    };

                    info!(
                        message = "start consume partition",
                        topic,
                        partition,
                        ?start
                    );

                    let mut current = 0i64;
                    let records = match pc.fetch_records(current, 1..52428800, 500).await {
                        Ok((records, watermark)) => {
                            for record in records {
                                current = record.offset;
                                match convert_message(record, &topic, partition, &dec).await {
                                    Some(logs) => {
                                        if let Err(err) = out.send(logs).await {
                                            error!(message = "send logs failed", ?err);
                                            return
                                        }
                                    }
                                    None => {}
                                }
                            }
                        },
                        Err(err) => {
                            error!(
                                message = "fetch records failed",
                                ?err,
                                topic,
                                partition,
                                current_offset=current,
                            );

                            break
                        }
                    };

                    loop {
                        tokio::select! {
                            result = pc.fetch_records(current, 1..52428800, 500) => {
                                match result {
                                    Ok((records, _water_mark)) => {

                                    },
                                    Err(err) => {
                                        error!(
                                            message = "fetch records failed",
                                            ?err,
                                            topic,
                                            partition,
                                            current_offset=current,
                                        );

                                        break
                                    }
                                }
                            }

                            _ = signal.notified() => {
                                break
                            }
                        }
                    }

                    // consumer exit
                });
            }
        }

        // heartbeat loop
        let signal = Arc::clone(&notify);
        let hc = Arc::clone(&consumer);
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(3));
            let mut retries = 3;

            loop {
                tokio::select! {
                    _ = ticker.tick() => {},
                    _ = signal.notified() => return
                }

                if let Err(err) = hc.heartbeat().await {
                    match err {
                        rskafka::client::error::Error::ServerError { protocol_error, .. }
                            if protocol_error == ProtocolError::RebalanceInProgress =>
                        {
                            info!("rebalancing triggered");
                            break;
                        }
                        _ => {
                            warn!("unexpected error when heartbeat, {}", err);
                            retries -= 1;
                            if retries <= 0 {
                                break;
                            }
                        }
                    }
                } else {
                    retries = 3;
                }
            }

            // topic check loop might call this too, so send might failed,
            // but it's ok;
            signal.notify_waiters();

            debug!(message = "heartbeat loop exit");
        });

        // commit loop
        let signal = Arc::clone(&notify);
        let cc = Arc::clone(&consumer);
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(5));

            loop {
                tokio::select! {
                    _ = signal.notified() => break,
                    _ = ticker.tick() => {}
                }

                let mut i = 0;
                for PartitionAssignment { topic, partitions } in cc.assignment() {
                    for partition in partitions {
                        let offset = offsets[i].load(Ordering::Relaxed);
                        i += 1;

                        if offset == 0 {
                            // consume nothing till now
                            continue;
                        }

                        let topics = cc.commit(topic, *partition, offset).await.unwrap();
                        for topic in topics {
                            for p in topic.partitions {
                                info!(
                                    "commit offset of {}/{}/{} err:{:?}",
                                    topic.name, p.partition_index, offset, p.error_code
                                );
                            }
                        }
                    }
                }
            }
        });

        // topic check loop
        let mut ticker = tokio::time::interval(Duration::from_secs(60 * 10));
        loop {
            tokio::select! {
                _ = notify.notified() => break,
                _ = ticker.tick() => {},
                _ = &mut shutdown => {
                    notify.notify_waiters();

                    return Ok(());
                }
            }

            let mut new_topics = client
                .list_topics()
                .await
                .unwrap()
                .into_iter()
                .filter(|t| t.name.starts_with("test_"))
                .collect::<Vec<_>>();
            new_topics.sort_by(|a, b| a.name.cmp(&b.name));

            if !compare_topics(&topics, &new_topics) {
                notify.notify_waiters();
                break;
            }
        }
    }
}

// compare_topics compare topic count, name and partitions
//
// eq_by is not stable yet.
fn compare_topics(old: &[Topic], new: &[Topic]) -> bool {
    if old.len() != new.len() {
        return false;
    }

    for i in 0..old.len() {
        let o = &old[i];
        let n = &new[i];

        if o.name != n.name {
            return false;
        }

        if o.partitions.len() != n.partitions.len() {
            return false;
        }
    }

    return true;
}

/*
fn handle_ack(
    topics: &mut Topics,
    status: BatchStatus,
    entry: FinalizerEntry,
    consumer: &StreamConsumer<KafkaStatisticsContext>,
) {
    if !topics.failed.contains(&entry.topic) {
        if status == BatchStatus::Delivered {
            if let Err(err) = consumer.store_offset(&entry.topic, entry.partition, entry.offset) {
                error!(
                    message = "Unable to update consumer offset",
                    %err,
                    internal_log_rate_limit = true,
                );
            }
        } else {
            error!(
                message = "Event received a negative acknowledgment, topic has been stopped",
                topic = entry.topic,
                partition = entry.partition,
                offset = entry.offset,
            );

            // Try to unsubscribe from the named topic. Note that the subscribed topics list
            // could be missing the named topic for two reasons:
            // 1. Multiple batches of events from the same topic could be flight and
            //    all receive acknowledgement, in which case it will only be present for the
            //    first response.
            // 2. The topic list may contain wildcards, in which case there may not be an
            //    exact match for the topic name.
            if topics.subscribed.remove(&entry.topic) {
                let topics: Vec<&str> = topics
                    .subscribed
                    .iter()
                    .map(|topic| topic.as_str())
                    .collect();
                // There is no direct way to unsubscribe from a named topic, as the
                // unsubscribe library function drops all topics. The subscribe function,
                // however, replaces the list of subscriptions, from which we have
                // removed the topic above. Ignore any errors, as we drop output from
                // the topic below anyways.
                let _ = consumer.subscribe(&topics);
            }

            // Don't update the offset after a failed ack
            topics.failed.insert(entry.topic);
        }
    }
}
*/

#[derive(Clone, Copy)]
struct Keys<'a> {
    timestamp: &'a str,
    key_field: &'a str,
    topic: &'a str,
    partition: &'a str,
    offset: &'a str,
    headers: &'a str,
}

/*
struct ReceivedMessage {
    timestamp: DateTime<Utc>,
    key: Value,
    headers: BTreeMap<String, Value>,
    topic: String,
    partition: i32,
    offset: i64,
}

impl ReceivedMessage {
    fn from(msg: &BorrowedMessage<'_>) -> Self {
        // Extract timestamp from kafka message
        let timestamp = msg
            .timestamp()
            .to_millis()
            .and_then(|millis| Utc.timestamp_millis_opt(millis).latest())
            .unwrap_or_else(Utc::now);
        let key = msg
            .key()
            .map(|key| Value::from(Bytes::from(key.to_owned())))
            .unwrap_or(Value::Null);

        let mut headers = BTreeMap::new();
        if let Some(borrowed) = msg.headers() {
            // Using index-based for loop because rdkafka's `Headers` trait does not provide
            // Interator-based API
            for i in 0..borrowed.count() {
                let header = borrowed.get(i);
                let value = header
                    .value
                    .map(Bytes::copy_from_slice)
                    .unwrap_or_else(Bytes::new);
                headers.insert(header.key.to_string(), value.into());
            }
        }

        Self {
            timestamp,
            key,
            headers,
            topic: msg.topic().to_string(),
            partition: msg.partition(),
            offset: msg.offset(),
        }
    }

    fn apply(&self, keys: &Keys<'_>, event: &mut Event) {
        if let Event::Log(log) = event {
            log.insert_tag(log_schema().source_type_key(), "kafka");

            log.insert_field(keys.timestamp, self.timestamp);
            log.insert_field(keys.key_field, self.key.clone());
            log.insert_field(keys.topic, self.topic.clone());
            log.insert_field(keys.partition, self.partition);
            log.insert_field(keys.offset, self.offset);
            log.insert_field(keys.headers, self.headers.clone());
        }
    }
}
*/

async fn convert_message(
    record: RecordAndOffset,
    topic: &str,
    partition: i32,
    decoder: &Decoder,
) -> Option<Vec<LogRecord>> {
    let RecordAndOffset { record, offset } = record;
    let payload = record.value?;
    let timestamp = record.timestamp;
    let key = record
        .key
        .map(|key| Value::from(Bytes::from(key)))
        .unwrap_or(Value::Null);
    let mut headers = record
        .headers
        .into_iter()
        .map(|(key, value)| (key, Value::from(value)))
        .collect::<BTreeMap<_, _>>();

    let mut stream = FramedRead::new(payload.as_slice(), decoder.clone());
    let (count, _) = stream.size_hint();

    let mut logs = Vec::with_capacity(count);
    while let Some(result) = stream.next().await {
        match result {
            Ok((events, _byte_size)) => {
                for mut event in events {
                    let mut log = event.into_log();

                    log.insert_tag(log_schema().source_type_key(), "kafka");
                    log.insert_field("timestamp", timestamp);
                    log.insert_field("key", key.clone());
                    log.insert_field("topic", topic.to_string());
                    log.insert_field("partition", partition);
                    log.insert_field("offset", offset);
                    log.insert_field("headers", headers.clone());

                    logs.push(log);
                }
            }
            Err(err) => {
                if !err.can_continue() {
                    break;
                }
            }
        }
    }

    Some(logs)
}

/*
async fn parse_message(
    msg: BorrowedMessage<'_>,
    decoder: &Decoder,
    keys: Keys<'_>,
    finalizer: &Option<OrderedFinalizer<FinalizerEntry>>,
    output: &mut Pipeline,
    consumer: &Arc<StreamConsumer<KafkaStatisticsContext>>,
    topics: &Topics,
) {
    if let Some(logs) = parse_stream(&msg, decoder, keys, topics).await {
        let count = logs.len();

        match finalizer {
            Some(finalizer) => {
                let (batch, receiver) = BatchNotifier::new_with_receiver();
                let logs = logs
                    .into_iter()
                    .map(|log| log.with_batch_notifier(&batch))
                    .collect::<Vec<_>>();

                match output.send(logs).await {
                    Ok(_) => {
                        finalizer.add(msg.into(), receiver);
                    }
                    Err(err) => {
                        error!(
                            message = "Failed to forward event(s), downstream is closed",
                            count,
                            %err
                        );
                    }
                }
            }
            None => match output.send(logs).await {
                Ok(_) => {
                    if let Err(err) =
                        consumer.store_offset(msg.topic(), msg.partition(), msg.offset())
                    {
                        error!(
                            message = "Unable to update consumer offset",
                            %err,
                            internal_log_rate_limit = true
                        )
                    }
                }
                Err(err) => {
                    error!(
                        message = "Failed to forward event(s), downstream is closed",
                        count,
                        ?err
                    );
                }
            },
        }
    }
}

// Turn the received message into a stream of parsed events.
async fn parse_stream<'a>(
    msg: &BorrowedMessage<'a>,
    decoder: &Decoder,
    keys: Keys<'a>,
    topics: &Topics,
) -> Option<Vec<LogRecord>> {
    if topics.failed.contains(msg.topic()) {
        return None;
    }

    let payload = msg.payload()?; // skip messages with empty payload

    let rmsg = ReceivedMessage::from(msg);
    let payload = Cursor::new(Bytes::copy_from_slice(payload));
    let mut stream = FramedRead::new(payload, decoder.clone());
    let (count, _) = stream.size_hint();

    let mut logs = Vec::with_capacity(count);
    while let Some(result) = stream.next().await {
        match result {
            Ok((events, _byte_size)) => {
                for mut event in events {
                    rmsg.apply(&keys, &mut event);
                    logs.push(event.into_log());
                }
            }
            Err(err) => {
                if !err.can_continue() {
                    break;
                }
            }
        }
    }

    Some(logs)
}
*/

/*
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<KafkaSourceConfig>()
    }

    const BOOTSTRAP_SERVER: &str = "localhost:9091";

    pub(super) fn make_config(
        bootstrap_servers: &str,
        topic: &str,
        group: &str,
    ) -> KafkaSourceConfig {
        KafkaSourceConfig {
            bootstrap_brokers: vec![bootstrap_servers.to_string()],
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
            framing: default_framing_message_based(),
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
*/

/*
#[cfg(all(test, feature = "integration-tests-kafka"))]
mod integration_tests {
    use chrono::{SubsecRound, Utc};
    use event::{log::Value, EventStatus};
    use framework::{Pipeline, ShutdownSignal};
    use log_schema::log_schema;
    use std::time::Duration;
    use testcontainers::images::zookeeper::Zookeeper;
    use testcontainers::{clients, RunnableImage};
    use testify::collect_n;
    use testify::random::random_string;

    use super::run;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn consume_with_ack() {
        let cli = clients::Cli::default();
        let image = RunnableImage::from(Zookeeper::default());
        let container = cli.run(image);
        let port = container.get_host_port_ipv4(9092);

        consume_event(format!("localhost:{}", port), true).await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn consume_without_ack() {
        let cli = clients::Cli::default();
        let image = RunnableImage::from(Zookeeper::default());
        let container = cli.run(image);
        let port = container.get_host_port_ipv4(9092);

        consume_event(format!("localhost:{}", port), false).await;
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
        tokio::spawn(run(
            config.clone(),
            consumer,
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
                .headers(OwnedHeaders::new().insert(Header {
                    key: header_key,
                    value: Some(header_value),
                }));

            match producer.send(record, Duration::from_secs(3)).await {
                Ok((_partition, _offset)) => {
                    // dbg!("partition: {}, offset: {}", partition, offset);
                }
                Err(err) => {
                    panic!(
                        "Cannot send event to Kafka, server: {:?}, err: {:?}",
                        servers, err
                    )
                }
            }
        }
    }
}
*/
