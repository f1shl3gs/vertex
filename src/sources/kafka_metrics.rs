use std::time::{Duration, Instant};

use async_trait::async_trait;
use configurable::configurable_component;
use event::{tags, Metric};
use framework::config::{
    default_interval, serde_regex, DataType, Output, SourceConfig, SourceContext,
};
use framework::{Pipeline, ShutdownSignal, Source};
use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;
use regex::Regex;
use rskafka::client::partition::{OffsetAt, UnknownTopicHandling};
use rskafka::client::{Client, ClientBuilder};

fn default_topic_filter() -> Regex {
    Regex::new(".*").expect("default topic filter")
}

/// Collect Kafka metrics. For other metrics from Kafka, have a look at the
/// [JMX exporter](https://github.com/prometheus/jmx_exporter).
///
/// N.B. `Consume lag` metrics is not supported yet, and this feature not
/// enabled by default in
/// [Kafka exporter](https://github.com/danielqsj/kafka_exporter)
#[configurable_component(source, name = "kafka_metrics")]
#[derive(Debug)]
#[serde(deny_unknown_fields)]
struct Config {
    /// A comma-separated list of host and port pairs that are the addresses of
    /// the Kafka brokers in a "bootstrap" Kafka cluster that a Kafka client
    /// connects to initially ot bootstrap itself.
    #[configurable(required, format = "ip-address", example = "127.0.0.1:9092")]
    bootstrap_servers: Vec<String>,

    /// This sources collects metrics on an interval.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    /// Regex that determines which topics to collect.
    #[serde(with = "serde_regex", default = "default_topic_filter")]
    topic_filter: Regex,
}

#[async_trait]
#[typetag::serde(name = "kafka_metrics")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        let client = ClientBuilder::new(self.bootstrap_servers.clone())
            .build()
            .await?;

        Ok(Box::pin(run(
            client,
            self.interval,
            self.topic_filter.clone(),
            cx.output,
            cx.shutdown,
        )))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }
}

async fn run(
    client: Client,
    interval: Duration,
    topic_filter: Regex,
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let mut ticker = tokio::time::interval(interval);

    loop {
        tokio::select! {
            biased;

            _ = &mut shutdown => break,
            _ = ticker.tick() => {}
        }

        let start = Instant::now();
        let result = scrape(&client, &topic_filter).await;
        let elapsed = start.elapsed();

        let mut metrics = vec![
            Metric::gauge(
                "kafka_up",
                "Whether collector can access kafka cluster properly or not",
                result.is_ok(),
            ),
            Metric::gauge(
                "kafak_scrape_duration_seconds",
                "Time spent on collecting metrics",
                elapsed,
            ),
        ];

        if let Ok(ms) = result {
            metrics.extend(ms);
        }

        if let Err(err) = output.send(metrics).await {
            warn!(message = "send metrics failed", ?err);
            break;
        }
    }

    Ok(())
}

async fn scrape(client: &Client, topic_filter: &Regex) -> framework::Result<Vec<Metric>> {
    let brokers = client.brokers();
    let mut metrics = vec![Metric::gauge(
        "kafka_brokers",
        "Number of brokers in the kafka cluster.",
        brokers.len(),
    )];

    metrics.extend(brokers.iter().map(|b| {
        Metric::gauge_with_tags(
            "kafka_broker_info",
            "Information about the Kafka Broker",
            1,
            tags!(
                "id" => b.id.to_string(),
                "address" => format!("{}:{}", b.host, b.port)
            ),
        )
    }));

    let topics = client.list_topics().await?;
    let topics = topics.iter().filter(|t| topic_filter.is_match(&t.name));

    let mut tasks = FuturesUnordered::from_iter(topics.map(|topic| async move {
        let mut metrics = vec![Metric::gauge_with_tags(
            "kafka_topic_partitions",
            "Number of partitions for this Topic",
            topic.partitions.len(),
            tags!(
                "topic" => topic.name.clone()
            ),
        )];

        for (index, partition) in &topic.partitions {
            let tags = tags!(
                "topic" => topic.name.clone(),
                "partition" => *index
            );

            match client
                .partition_client(&topic.name, *index, UnknownTopicHandling::Error)
                .await
            {
                Ok(pc) => {
                    let offset = pc.get_offset(OffsetAt::Latest).await?;
                    metrics.push(Metric::gauge_with_tags(
                        "kafka_topic_partition_current_offset",
                        "Current offset of a broker at topic/partition",
                        offset,
                        tags.clone(),
                    ));

                    let offset = pc.get_offset(OffsetAt::Earliest).await?;
                    metrics.push(Metric::gauge_with_tags(
                        "kafka_topic_partition_oldest_offset",
                        "Oldest offset of a broker at topic/partition",
                        offset,
                        tags.clone(),
                    ))
                }
                Err(err) => {
                    warn!(message = "create partition client failed", ?err)
                }
            }

            metrics.extend_from_slice(&[
                Metric::gauge_with_tags(
                    "kafka_topic_partition_leader",
                    "Leader Broker ID of this Topic/Partition",
                    partition.leader_id,
                    tags.clone(),
                ),
                Metric::gauge_with_tags(
                    "kafka_topic_partition_replicas",
                    "Number of replicas for this topic/partition",
                    partition.replica_nodes.len(),
                    tags.clone(),
                ),
                Metric::gauge_with_tags(
                    "kafka_topic_partition_in_sync_replica",
                    "Number of in-sync replicas for this topic/partition",
                    partition.isr_nodes.len(),
                    tags.clone(),
                ),
                Metric::gauge_with_tags(
                    "kafka_topic_partition_leader_is_preferred",
                    "1 if topic/partition is using the Preferred broker",
                    partition.leader_id
                        == partition.replica_nodes.first().copied().unwrap_or_default(),
                    tags.clone(),
                ),
                Metric::gauge_with_tags(
                    "kafka_topic_partition_under_replicated_partition",
                    "1 if topic/partition is under replicated",
                    partition.isr_nodes.len() < partition.replica_nodes.len(),
                    tags.clone(),
                ),
            ]);
        }

        Ok::<Vec<Metric>, rskafka::client::error::Error>(metrics)
    }));

    while let Some(result) = tasks.next().await {
        metrics.extend(result?);
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>();
    }
}
