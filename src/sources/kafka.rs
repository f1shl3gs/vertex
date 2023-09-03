use std::collections::BTreeMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use chrono::Utc;
use codecs::decoding::{DeserializerConfig, FramingConfig, StreamDecodingError};
use codecs::{Decoder, DecodingConfig};
use configurable::{configurable_component, Configurable};
use event::{log::Value, LogRecord};
use framework::config::{DataType, Output, SourceConfig, SourceContext};
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::{Error, Source};
use futures::{Stream, StreamExt};
use futures_util::TryFutureExt;
use log_schema::log_schema;
use lookup::OwnedPath;
use rskafka::client::partition::{OffsetAt, UnknownTopicHandling};
use rskafka::client::{Client, ClientBuilder};
use rskafka::protocol::error::Error as ProtocolError;
use rskafka::protocol::messages::PartitionAssignment;
use rskafka::record::RecordAndOffset;
use rskafka::topic::Topic;
use serde::{Deserialize, Serialize};
use tokio_util::codec::FramedRead;

const fn default_session_timeout() -> Duration {
    Duration::from_secs(10)
}

const fn default_fetch_wait_max() -> Duration {
    Duration::from_millis(200)
}

const fn default_commit_interval() -> Duration {
    Duration::from_secs(5)
}

fn default_key_field() -> OwnedPath {
    "message_key".into()
}

fn default_topic_key() -> OwnedPath {
    "topic".into()
}

fn default_partition_key() -> OwnedPath {
    "partition".into()
}

fn default_offset_key() -> OwnedPath {
    "offset".into()
}

fn default_headers_key() -> OwnedPath {
    "headers".into()
}

const fn default_decoding() -> DeserializerConfig {
    DeserializerConfig::Bytes
}

const fn default_framing_message_based() -> FramingConfig {
    FramingConfig::Bytes
}

#[derive(Debug, Default, Deserialize, Serialize, Configurable)]
#[serde(rename = "lowercase")]
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
    const fn start_offset(&self) -> OffsetAt {
        match self {
            AutoOffsetReset::Earliest => OffsetAt::Earliest,
            AutoOffsetReset::Latest => OffsetAt::Latest,
        }
    }
}

/// Collect logs from Apache Kafka topics.
#[configurable_component(source, name = "kafka")]
#[serde(deny_unknown_fields)]
struct Config {
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

    /// Tell Kafka to wait until it has enough data to send before responding to the consumer.
    #[serde(default = "default_fetch_wait_max")]
    fetch_wait_max: Duration,

    /// The log field name to use for the Kafka message key.
    #[serde(default = "default_key_field")]
    key_field: OwnedPath,

    /// The log field name to use for the Kafka topic.
    #[serde(default = "default_topic_key")]
    topic_key: OwnedPath,

    /// The log field name to use for the Kafka partition name.
    #[serde(default = "default_partition_key")]
    partition_key: OwnedPath,

    /// The log field name to use for the Kafka offset
    #[serde(default = "default_offset_key")]
    offset_key: OwnedPath,

    /// The log field name to use for the Kafka headers.
    #[serde(default = "default_headers_key")]
    headers_key: OwnedPath,

    #[serde(default = "default_framing_message_based")]
    framing: FramingConfig,

    #[serde(default = "default_decoding")]
    decoding: DeserializerConfig,

    #[serde(default)]
    acknowledgement: bool,
}

#[async_trait::async_trait]
#[typetag::serde(name = "kafka")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let client = ClientBuilder::new(self.bootstrap_brokers.clone())
            .build()
            .await?;

        let keys = Arc::new(Keys::from(self));
        let decoder = DecodingConfig::new(self.framing.clone(), self.decoding.clone()).build();

        Ok(Box::pin(
            run(
                client,
                self.group.clone(),
                self.topics.clone(),
                self.auto_offset_reset.start_offset(),
                self.fetch_wait_max.as_millis() as i32,
                keys,
                decoder,
                cx.output,
                cx.shutdown,
            )
            .map_err(|err| {
                error!(message = "kafka source exit", ?err);
            }),
        ))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }
}

#[derive(Debug)]
struct Keys {
    timestamp: OwnedPath,
    key: OwnedPath,
    topic: OwnedPath,
    partition: OwnedPath,
    offset: OwnedPath,
    headers: OwnedPath,
}

impl Keys {
    fn from(config: &Config) -> Self {
        Self {
            timestamp: OwnedPath::from(log_schema().timestamp_key()),
            key: config.key_field.clone(),
            topic: config.topic_key.clone(),
            partition: config.partition_key.clone(),
            offset: config.offset_key.clone(),
            headers: config.headers_key.clone(),
        }
    }
}

async fn run(
    client: Client,
    group: String,
    want_topics: Vec<String>,
    offset_at: OffsetAt,
    fetch_max_wait_ms: i32,
    keys: Arc<Keys>,
    decoder: Decoder,
    output: Pipeline,
    mut shutdown: ShutdownSignal,
) -> Result<(), Error> {
    let client = Arc::new(client);

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
        let committed_offsets = consumer.offsets().await?;
        let mut offsets = Vec::new();

        // consume topics
        for PartitionAssignment { topic, partitions } in consumer.assignment() {
            let topic_offsets = committed_offsets.iter().find(|t| &t.name == topic);

            for partition in partitions {
                let topic = topic.to_string();
                let partition = *partition;
                let signal = Arc::clone(&notify);
                let current_offset = Arc::new(AtomicI64::new(0));
                let cli = Arc::clone(&client);
                let dec = decoder.clone();
                let keys = Arc::clone(&keys);
                let mut out = output.clone();
                offsets.push(Arc::clone(&current_offset));

                let committed_offset = match topic_offsets {
                    Some(topic) => topic
                        .partitions
                        .iter()
                        .find(|p| p.partition_index == partition)
                        .map(|p| p.committed_offset),
                    None => None,
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

                    // get correct start offset.
                    let start = match committed_offset {
                        Some(committed) => {
                            // the committed offset might be small than earliest offset.
                            // if this happened, OutOfRange error will be returned.
                            let earliest = match pc.get_offset(OffsetAt::Earliest).await {
                                Ok(o) => o,
                                Err(err) => {
                                    error!(message = "get earliest offset failed", ?err);
                                    return;
                                }
                            };

                            committed.max(earliest)
                        }
                        None => match pc.get_offset(offset_at).await {
                            Ok(o) => o,
                            Err(err) => {
                                error!(message = "get start offset failed", ?err);
                                return;
                            }
                        },
                    };

                    current_offset.store(start, Ordering::Relaxed);
                    info!(
                        message = "start consume partition",
                        topic,
                        partition,
                        start,
                        ?committed_offset,
                    );

                    loop {
                        let start = current_offset.load(Ordering::Relaxed);
                        let result = tokio::select! {
                            result = pc.fetch_records(start, 1..52428800, fetch_max_wait_ms) => result,
                            _ = signal.notified() => break,
                        };

                        match result {
                            Ok((records, _watermark)) => {
                                if records.is_empty() {
                                    // no new message
                                    continue;
                                }

                                let mut current = start;
                                for record in records {
                                    current = record.offset;
                                    if let Some(logs) =
                                        convert_message(record, &topic, partition, &dec, &keys)
                                            .await
                                    {
                                        if let Err(err) = out.send(logs).await {
                                            error!(message = "send logs failed", ?err);
                                            return;
                                        }
                                    }
                                }

                                current_offset.store(current + 1, Ordering::Relaxed);
                            }
                            Err(err) => {
                                error!(
                                    message = "fetch records failed",
                                    ?err,
                                    topic,
                                    partition,
                                    current_offset = start,
                                );

                                break;
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
                            info!(message = "rebalancing triggered");
                            break;
                        }
                        _ => {
                            warn!(message = "unexpected error when heartbeat", ?err);

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

                        if let Err(err) = cc.commit(topic, *partition, offset).await {
                            error!(message = "commit offset failed", ?err);
                        }
                    }
                }
            }
        });

        // topic check loop
        let mut ticker = tokio::time::interval(Duration::from_secs(60 * 10));
        loop {
            tokio::select! {
                _ = notify.notified() => {
                    // rebalance or heartbeat error
                    break
                },
                _ = ticker.tick() => {},
                _ = &mut shutdown => {
                    notify.notify_waiters();

                    if let Err(err) = consumer.leave().await {
                        warn!(message = "consumer leave failed", ?err);
                    }

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

    true
}

async fn convert_message(
    record: RecordAndOffset,
    topic: &str,
    partition: i32,
    decoder: &Decoder,
    keys: &Keys,
) -> Option<Vec<LogRecord>> {
    let RecordAndOffset { record, offset } = record;
    let payload = record.value?;
    let timestamp = if record.timestamp.timestamp() == 0 {
        Utc::now()
    } else {
        record.timestamp
    };
    let key = record
        .key
        .map(|key| Value::from(Bytes::from(key)))
        .unwrap_or(Value::Null);
    let headers = record
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
                for event in events {
                    let mut log = event.into_log();

                    log.insert_tag(log_schema().source_type_key(), "kafka");
                    log.insert_field(&keys.timestamp, timestamp);
                    log.insert_field(&keys.key, key.clone());
                    log.insert_field(&keys.topic, topic.to_string());
                    log.insert_field(&keys.partition, partition);
                    log.insert_field(&keys.offset, offset);
                    log.insert_field(&keys.headers, headers.clone());

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }
}
