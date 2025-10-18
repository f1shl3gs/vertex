use configurable::Configurable;
use event::{Metric, tags};
use framework::http::HttpClient;
use serde::{Deserialize, Serialize};

use super::{Error, fetch};

#[derive(Clone, Configurable, Debug, Default, Deserialize, Serialize)]
pub struct Config {
    accounts: bool,
    consumers: bool,
    streams: bool,
}

impl Config {
    pub fn enabled(&self) -> bool {
        self.accounts && self.streams && self.consumers
    }
}

#[derive(Deserialize)]
struct JetStreamConfig {
    #[serde(default)]
    domain: Option<String>,

    max_memory: i64,
    max_storage: i64,
}

#[derive(Deserialize)]
struct JetStreamStats {
    memory: u64,
    storage: u64,
    reserved_memory: u64,
    reserved_storage: u64,
}

#[derive(Deserialize)]
struct MetaCluster {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    leader: Option<String>,
}

#[derive(Deserialize)]
struct ConsumerConfig {
    #[serde(default)]
    description: Option<String>,
}

#[derive(Deserialize)]
struct SequenceInfo {
    consumer_seq: u64,
    stream_seq: u64,
}

#[derive(Deserialize)]
struct ConsumerInfo {
    name: String,

    #[serde(default)]
    config: Option<ConsumerConfig>,
    #[serde(default)]
    cluster: Option<ClusterInfo>,

    delivered: SequenceInfo,
    ack_floor: SequenceInfo,

    num_ack_pending: i64,
    num_redelivered: i64,
    num_waiting: i64,
    num_pending: i64,
}

#[derive(Deserialize)]
struct ExternalStream {
    api: String,
    deliver: String,
}

#[derive(Deserialize)]
struct StreamSourceInfo {
    name: String,
    lag: u64,
    active: i64,

    #[serde(default)]
    external: Option<ExternalStream>,
}

#[derive(Deserialize)]
struct StreamConfig {
    max_bytes: i64,
    max_msgs: i64,
}

/// Information about the underlying set of servers that make up the stream or consumer.
#[derive(Deserialize)]
struct ClusterInfo {
    #[serde(default)]
    leader: Option<String>,
}

#[derive(Deserialize)]
struct StreamState {
    messages: u64,
    bytes: u64,
    first_seq: u64,
    last_seq: u64,
    consumer_count: i32,
    #[serde(default)]
    num_subjects: Option<i32>,
}

#[derive(Deserialize)]
struct StreamDetail {
    name: String,

    #[serde(default)]
    state: Option<StreamState>,
    #[serde(default)]
    config: Option<StreamConfig>,
    #[serde(default)]
    cluster: Option<ClusterInfo>,
    #[serde(default, rename = "stream_raft_group")]
    raft_group: Option<String>,

    #[serde(default, rename = "consumer_detail")]
    consumers: Vec<ConsumerInfo>,
    #[serde(default)]
    sources: Vec<StreamSourceInfo>,
}

#[derive(Deserialize)]
struct AccountDetail {
    #[serde(flatten)]
    stats: JetStreamStats,

    id: String,
    name: String,

    #[serde(default, rename = "stream_detail")]
    streams: Vec<StreamDetail>,
}

#[derive(Deserialize)]
struct JetStreamInfo {
    #[serde(default)]
    disabled: bool,
    streams: i64,
    consumers: i64,
    messages: u64,
    bytes: u64,

    config: Option<JetStreamConfig>,
    meta_cluster: Option<MetaCluster>,
    #[serde(default)]
    account_details: Vec<AccountDetail>,
}

pub async fn collect(
    client: &HttpClient,
    endpoint: &str,
    config: &Config,
    server_name: &str,
) -> Result<Vec<Metric>, Error> {
    let mut metrics = if config.accounts {
        collect_inner(client, format!("{endpoint}/jsz?accounts=true"), server_name).await?
    } else {
        Vec::new()
    };

    if config.consumers {
        let partial = collect_inner(
            client,
            format!("{endpoint}/jsz?consumers=true&config=true&raft=true"),
            server_name,
        )
        .await?;
        metrics.extend(partial);
    }

    if config.streams {
        let partial =
            collect_inner(client, format!("{endpoint}/jsz?accounts=true"), server_name).await?;
        metrics.extend(partial);
    }

    Ok(metrics)
}

async fn collect_inner(
    client: &HttpClient,
    uri: String,
    server_name: &str,
) -> Result<Vec<Metric>, Error> {
    let resp = fetch::<JetStreamInfo>(client, &uri).await?;

    let mut metrics = Vec::new();

    let domain = match &resp.config {
        Some(config) => config.domain.clone().unwrap_or_default(),
        None => String::new(),
    };
    let (cluster, is_leader, leader) = match resp.meta_cluster {
        Some(meta) => {
            let cluster = meta.name.unwrap_or_default();
            let leader = meta.leader.unwrap_or_default();
            (cluster, leader == server_name, leader)
        }
        None => (String::new(), true, String::new()),
    };

    let tags = tags!(
        "cluster" => cluster,
        "domain" => domain,
        "meta_leader" => leader,
        "is_meta_leader" => is_leader,
    );

    metrics.extend([
        Metric::gauge_with_tags(
            "jetstream_server_disabled",
            "JetStream disabled or not",
            resp.disabled,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "jetstream_server_streams",
            "Total number of streams in JetStream",
            resp.streams,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "jetstream_server_consumers",
            "Total number of consumers in JetStream",
            resp.consumers,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "jetstream_server_messages_total",
            "Total number of stored messages in JetStream",
            resp.messages,
            tags.clone(),
        ),
        Metric::gauge_with_tags(
            "jetstream_server_messages_bytes",
            "Total number of bytes stored in JetStream",
            resp.bytes,
            tags.clone(),
        ),
    ]);

    if let Some(config) = resp.config {
        metrics.extend([
            Metric::gauge_with_tags(
                "jetstream_server_max_memory",
                "JetStream max memory",
                config.max_memory,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "jetstream_server_max_storage",
                "JetStream Storage",
                config.max_storage,
                tags.clone(),
            ),
        ]);
    }

    for account in resp.account_details {
        let mut account_tags = tags.clone();
        account_tags.insert("account_id", account.id);
        account_tags.insert("account", account.name.clone());
        account_tags.insert("account_name", account.name);

        metrics.extend([
            Metric::gauge_with_tags(
                "jetstream_account_max_storage",
                "JetStream max storage in bytes",
                account.stats.reserved_storage,
                account_tags.clone(),
            ),
            Metric::gauge_with_tags(
                "jetstream_account_storage_used",
                "Total number of bytes used by JetStream storage",
                account.stats.storage,
                account_tags.clone(),
            ),
            Metric::gauge_with_tags(
                "jetstream_account_max_memory",
                "JetStream max memory in bytes",
                account.stats.reserved_memory,
                account_tags.clone(),
            ),
            Metric::gauge_with_tags(
                "jetstream_account_memory_used",
                "Total number of bytes used by JetStream memory",
                account.stats.memory,
                account_tags.clone(),
            ),
        ]);

        for stream in account.streams {
            let (is_leader, stream_leader) = match stream.cluster {
                Some(cluster) => {
                    let leader = cluster.leader.unwrap_or_default();

                    (leader == server_name, leader)
                }
                None => (false, String::new()),
            };

            let mut stream_tags = account_tags.clone();
            stream_tags.insert("stream_name", stream.name);
            stream_tags.insert("stream_leader", stream_leader);
            stream_tags.insert("is_stream_leader", is_leader);
            stream_tags.insert("stream_raft_group", stream.raft_group.unwrap_or_default());

            if let Some(state) = stream.state {
                metrics.extend([
                    Metric::gauge_with_tags(
                        "jetstream_stream_messages_total",
                        "Total number of messages from a stream",
                        state.messages,
                        stream_tags.clone(),
                    ),
                    Metric::gauge_with_tags(
                        "jetstream_stream_bytes",
                        "Total stored bytes from a stream",
                        state.bytes,
                        stream_tags.clone(),
                    ),
                    Metric::gauge_with_tags(
                        "jetstream_stream_first_seq",
                        "First sequence from a stream",
                        state.first_seq,
                        stream_tags.clone(),
                    ),
                    Metric::gauge_with_tags(
                        "jetstream_stream_last_seq",
                        "Last sequence from a stream",
                        state.last_seq,
                        stream_tags.clone(),
                    ),
                    Metric::gauge_with_tags(
                        "jetstream_stream_consumer_count",
                        "Total number of consumers from a stream",
                        state.consumer_count,
                        stream_tags.clone(),
                    ),
                ]);

                if let Some(num_subjects) = state.num_subjects {
                    metrics.push(Metric::gauge_with_tags(
                        "jetstream_stream_subject_count",
                        "Total number of subjects in a stream",
                        num_subjects,
                        stream_tags.clone(),
                    ));
                }
            }

            if let Some(config) = stream.config {
                metrics.extend([
                    Metric::gauge_with_tags(
                        "jetstream_stream_limit_bytes",
                        "The maximum configured storage limit (in bytes) for a JetStream stream. A value of -1 indicates no limit",
                        config.max_bytes,
                        stream_tags.clone(),
                    ),
                    Metric::gauge_with_tags(
                        "jetstream_stream_limit_messages",
                        "The maximum number of messages allowed in a JetStream stream as per its configuration. A value of -1 indicates no limit",
                        config.max_msgs,
                        stream_tags.clone(),
                    )
                ]);
            }

            for source in stream.sources {
                let mut source_tags = stream_tags.clone();
                source_tags.insert("source", source.name);
                let (api, deliver) = match source.external {
                    None => (String::new(), String::new()),
                    Some(external) => (external.api, external.deliver),
                };
                source_tags.insert("source_api", api);
                source_tags.insert("source_deliver", deliver);

                metrics.extend([
                    Metric::gauge_with_tags(
                        "jetstream_stream_source_lag",
                        "Number of messages a stream source is behind",
                        source.lag,
                        source_tags.clone(),
                    ),
                    Metric::gauge_with_tags(
                        "jetstream_stream_source_active_duration_seconds",
                        "Stream source active duration in nanoseconds (-1 indicates inactive)",
                        source.active / 1000 / 1000 / 1000,
                        source_tags,
                    ),
                ]);
            }

            for consumer in stream.consumers {
                let mut consumer_tags = stream_tags.clone();
                consumer_tags.insert("consumer_name", consumer.name);
                let consumer_desc = if let Some(config) = consumer.config {
                    config.description.unwrap_or_default()
                } else {
                    String::new()
                };
                consumer_tags.insert("consumer_desc", consumer_desc);

                if let Some(cluster) = consumer.cluster {
                    consumer_tags.insert(
                        "is_consumer_leader",
                        cluster
                            .leader
                            .as_ref()
                            .map(|leader| leader == server_name)
                            .unwrap_or_default(),
                    );
                    consumer_tags.insert("consumer_leader", cluster.leader.unwrap_or_default());
                } else {
                    consumer_tags.insert("is_consumer_leader", true);
                    consumer_tags.insert("consumer_leader", String::new());
                }

                metrics.extend([
                    Metric::gauge_with_tags(
                        "jetstream_consumer_delivered_consumer_seq",
                        "Latest sequence number of a stream consumer",
                        consumer.delivered.consumer_seq,
                        consumer_tags.clone(),
                    ),
                    Metric::gauge_with_tags(
                        "jetstream_consumer_delivered_stream_seq",
                        "Latest sequence number of a stream",
                        consumer.delivered.stream_seq,
                        consumer_tags.clone(),
                    ),
                    Metric::gauge_with_tags(
                        "jetstream_consumer_num_ack_pending",
                        "Number of pending acks from a consumer",
                        consumer.num_ack_pending,
                        consumer_tags.clone(),
                    ),
                    Metric::gauge_with_tags(
                        "jetstream_consumer_redelivered",
                        "Number of redelivered messages from a consumer",
                        consumer.num_redelivered,
                        consumer_tags.clone(),
                    ),
                    Metric::gauge_with_tags(
                        "jetstream_consumer_num_waiting",
                        "Number of inflight fetch requests from a pull consumer",
                        consumer.num_waiting,
                        consumer_tags.clone(),
                    ),
                    Metric::gauge_with_tags(
                        "jetstream_consumer_num_pending",
                        "Number of pending messages from a consumer",
                        consumer.num_pending,
                        consumer_tags.clone(),
                    ),
                    Metric::gauge_with_tags(
                        "jetstream_consumer_ack_floor_stream_seq",
                        "Number of ack floor stream seq from a consumer",
                        consumer.ack_floor.stream_seq,
                        consumer_tags.clone(),
                    ),
                    Metric::gauge_with_tags(
                        "jetstream_consumer_ack_floor_consumer_seq",
                        "Number of ack floor consumer seq from a consumer",
                        consumer.ack_floor.consumer_seq,
                        consumer_tags,
                    ),
                ]);
            }
        }
    }

    Ok(metrics)
}
