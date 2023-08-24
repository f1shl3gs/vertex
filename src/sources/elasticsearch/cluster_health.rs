use event::{tags, Metric};
use serde::Deserialize;

use super::Elasticsearch;

#[allow(dead_code)]
#[derive(Deserialize)]
struct ClusterHealth {
    cluster_name: String,
    status: String,
    timed_out: bool,
    number_of_nodes: i32,
    number_of_data_nodes: i32,
    active_primary_shards: i32,
    active_shards: i32,
    relocating_shards: i32,
    initializing_shards: i32,
    unassigned_shards: i32,
    delayed_unassigned_shards: i32,
    number_of_pending_tasks: i32,
    number_of_in_flight_fetch: i32,
    #[serde(default)]
    task_max_waiting_in_queue_millis: i32,
    #[serde(default)]
    active_shards_percent_as_number: f64,
}

impl Elasticsearch {
    pub async fn cluster_health(&self) -> Vec<Metric> {
        let result = self.fetch::<ClusterHealth>("/_cluster/health").await;
        let up = result.is_ok();

        let mut metrics = match result {
            Ok(ch) => {
                let tags = tags!(
                    "cluster" => ch.cluster_name.clone()
                );
                vec![
                    Metric::gauge_with_tags(
                        "elasticsearch_cluster_health_active_primary_shards",
                        "The number of primary shards in your cluster, this is an aggregate total across all indices",
                        ch.active_primary_shards,
                        tags.clone()
                    ),
                    Metric::gauge_with_tags(
                        "elasticsearch_cluster_health_active_shards",
                        "Aggregate total of all shards across all indices, which includes replica shards",
                        ch.active_shards,
                        tags.clone()
                    ),
                    Metric::gauge_with_tags(
                        "elasticsearch_cluster_health_delayed_unassigned_shards",
                        "Shards delayed to reduce reallocation overhead",
                        ch.delayed_unassigned_shards,
                        tags.clone()
                    ),
                    Metric::gauge_with_tags(
                        "elasticsearch_cluster_health_initializing_shards",
                        "Count of shards that are being freshly created",
                        ch.initializing_shards,
                        tags.clone()
                    ),
                    Metric::gauge_with_tags(
                        "elasticsearch_cluster_health_number_of_data_nodes",
                        "Number of data nodes in the cluster",
                        ch.number_of_data_nodes,
                        tags.clone()
                    ),
                    Metric::gauge_with_tags(
                        "elasticsearch_cluster_health_number_of_in_flight_fetch",
                        "The number of ongoing shard info requests",
                        ch.number_of_in_flight_fetch,
                        tags.clone()
                    ),
                    Metric::gauge_with_tags(
                        "elasticsearch_cluster_health_task_max_waiting_in_queue_millis",
                        "Tasks max time waiting in queue",
                        ch.task_max_waiting_in_queue_millis,
                        tags.clone()
                    ),
                    Metric::gauge_with_tags(
                        "elasticsearch_cluster_health_number_of_nodes",
                        "Number of nodes in the cluster",
                        ch.number_of_nodes,
                        tags.clone()
                    ),
                    Metric::gauge_with_tags(
                        "elasticsearch_cluster_health_number_of_pending_tasks",
                        "Cluster level changes which have not yet been executed",
                        ch.number_of_pending_tasks,
                        tags.clone()
                    ),
                    Metric::gauge_with_tags(
                        "elasticsearch_cluster_health_relocating_shards",
                        "The number of shards that are currently moving from one node to another node",
                        ch.relocating_shards,
                        tags.clone()
                    ),
                    Metric::gauge_with_tags(
                        "elasticsearch_cluster_health_unassigned_shards",
                        "The number of shards that exist in the cluster state, but cannot be found in the cluster itself",
                        ch.unassigned_shards,
                        tags
                    ),
                    Metric::gauge_with_tags(
                        "elasticsearch_cluster_health_status",
                        "Whether all primary and replica shards are allocated",
                        ch.status == "green",
                        tags!(
                    "cluster" => ch.cluster_name.clone(),
                    "color" => "green",
                ),
                    ),
                    Metric::gauge_with_tags(
                        "elasticsearch_cluster_health_status",
                        "Whether all primary and replica shards are allocated",
                        ch.status == "yellow",
                        tags!(
                    "cluster" => ch.cluster_name.clone(),
                    "color" => "yellow",
                ),
                    ),
                    Metric::gauge_with_tags(
                        "elasticsearch_cluster_health_status",
                        "Whether all primary and replica shards are allocated",
                        ch.status == "red",
                        tags!(
                    "cluster" => ch.cluster_name,
                    "color" => "red",
                ),
                    )
                ]
            }
            Err(err) => {
                warn!(message = "Fetch cluster health failed", ?err);

                vec![]
            }
        };

        metrics.push(Metric::gauge(
            "elasticsearch_cluster_health_up",
            "Was the last scrape of the Elasticsearch cluster health endpoint successful",
            up,
        ));

        metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode() {
        let tests = [
            (
                "1.7.6",
                r#"{"cluster_name":"elasticsearch","status":"yellow","timed_out":false,"number_of_nodes":1,"number_of_data_nodes":1,"active_primary_shards":5,"active_shards":5,"relocating_shards":0,"initializing_shards":0,"unassigned_shards":5,"delayed_unassigned_shards":0,"number_of_pending_tasks":0,"number_of_in_flight_fetch":0}"#,
            ),
            (
                "2.4.5",
                r#"{"cluster_name":"elasticsearch","status":"yellow","timed_out":false,"number_of_nodes":1,"number_of_data_nodes":1,"active_primary_shards":5,"active_shards":5,"relocating_shards":0,"initializing_shards":0,"unassigned_shards":5,"delayed_unassigned_shards":0,"number_of_pending_tasks":0,"number_of_in_flight_fetch":0,"task_max_waiting_in_queue_millis":12,"active_shards_percent_as_number":50.0}"#,
            ),
            (
                "5.4.2",
                r#"{"cluster_name":"elasticsearch","status":"yellow","timed_out":false,"number_of_nodes":1,"number_of_data_nodes":1,"active_primary_shards":5,"active_shards":5,"relocating_shards":0,"initializing_shards":0,"unassigned_shards":5,"delayed_unassigned_shards":0,"number_of_pending_tasks":0,"number_of_in_flight_fetch":0,"task_max_waiting_in_queue_millis":12,"active_shards_percent_as_number":50.0}"#,
            ),
        ];

        for (version, input) in tests {
            let ch = serde_json::from_str::<ClusterHealth>(input).unwrap();

            assert_eq!(ch.cluster_name, "elasticsearch");
            assert_eq!(ch.status, "yellow");
            assert!(!ch.timed_out);
            assert_eq!(ch.number_of_nodes, 1);
            assert_eq!(ch.number_of_data_nodes, 1);
            if version != "1.7.6" {
                assert_eq!(ch.task_max_waiting_in_queue_millis, 12)
            }
        }
    }
}
