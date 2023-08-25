use crate::sources::elasticsearch::Elasticsearch;
use event::{tags, Metric};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct Shard {
    total: i64,
    failed: i64,
    successful: i64,
}

#[derive(Deserialize)]
struct Failure {}

#[allow(dead_code)]
#[derive(Deserialize)]
struct SnapshotStatData {
    snapshot: String,
    #[serde(default)]
    uuid: String,
    version_id: i64,
    version: String,
    #[serde(default)]
    indices: Vec<String>,
    state: String,
    start_time_in_millis: i64,
    end_time_in_millis: i64,
    duration_in_millis: i64,
    shards: Shard,
    #[serde(default)]
    failures: Vec<Failure>,
}

#[derive(Deserialize)]
struct SnapshotsStats {
    #[serde(default)]
    snapshots: Vec<SnapshotStatData>,
}

#[derive(Deserialize)]
struct SnapshotRepository {}

impl Elasticsearch {
    pub async fn snapshots(&self) -> Result<Vec<Metric>, crate::Error> {
        let repos = self
            .fetch::<BTreeMap<String, SnapshotRepository>>("/_snapshot")
            .await?;

        let mut metrics = vec![];
        for (name, _repo) in repos {
            let stats = self
                .fetch::<SnapshotsStats>(format!("/_snapshot/{}/_all", name).as_str())
                .await?;

            // Repositories
            let tags = tags!(
                "repository" => name.clone()
            );
            let oldest_timestamp = if stats.snapshots.is_empty() {
                0
            } else {
                stats.snapshots[0].start_time_in_millis / 1000
            };
            let latest_timestamp = stats
                .snapshots
                .iter()
                .rev()
                .find(|snap| snap.state == "SUCCESS" || snap.state == "PARTIAL")
                .map(|snap| snap.start_time_in_millis / 1000)
                .unwrap_or_default();

            metrics.extend_from_slice(&[
                Metric::gauge_with_tags(
                    "elasticsearch_snapshot_stats_number_of_snapshots",
                    "Number of snapshots in a repository",
                    stats.snapshots.len(),
                    tags.clone(),
                ),
                Metric::gauge_with_tags(
                    "elasticsearch_snapshot_stats_oldest_snapshot_timestamp",
                    "Timestamp of the oldest snapshot",
                    oldest_timestamp,
                    tags.clone(),
                ),
                Metric::gauge_with_tags(
                    "elasticsearch_snapshot_stats_latest_snapshot_timestamp_seconds",
                    "Timestamp of the latest SUCCESS or PARTIAL snapshot",
                    latest_timestamp,
                    tags,
                ),
            ]);

            if stats.snapshots.is_empty() {
                continue;
            }

            let last_snapshot = stats.snapshots.last().unwrap();
            let tags = tags!(
                "repository" => name,
                "state" => last_snapshot.state.clone(),
                "version" => last_snapshot.version.clone(),
            );
            metrics.extend_from_slice(&[
                Metric::gauge_with_tags(
                    "elasticsearch_snapshot_stats_snapshot_number_of_indices",
                    "Number of indices in the last snapshot",
                    last_snapshot.indices.len(),
                    tags.clone(),
                ),
                Metric::gauge_with_tags(
                    "elasticsearch_snapshot_stats_snapshot_start_time_timestamp",
                    "Last snapshot start timestamp",
                    last_snapshot.start_time_in_millis / 1000,
                    tags.clone(),
                ),
                Metric::gauge_with_tags(
                    "elasticsearch_snapshot_stats_snapshot_end_time_timestamp",
                    "Last snapshot end timestamp",
                    last_snapshot.end_time_in_millis / 1000,
                    tags.clone(),
                ),
                Metric::gauge_with_tags(
                    "elasticsearch_snapshot_stats_snapshot_number_of_failures",
                    "Last snapshot number of failures",
                    last_snapshot.failures.len(),
                    tags.clone(),
                ),
                Metric::gauge_with_tags(
                    "elasticsearch_snapshot_stats_snapshot_total_shards",
                    "Last snapshot total shards",
                    last_snapshot.shards.total,
                    tags.clone(),
                ),
                Metric::gauge_with_tags(
                    "elasticsearch_snapshot_stats_snapshot_failed_shards",
                    "Last snapshot failed shards",
                    last_snapshot.shards.failed,
                    tags.clone(),
                ),
                Metric::gauge_with_tags(
                    "elasticsearch_snapshot_stats_snapshot_successful_shards",
                    "Last snapshot successful shards",
                    last_snapshot.shards.successful,
                    tags,
                ),
            ])
        }

        Ok(metrics)
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
                r#"{"test1":{"type":"fs","settings":{"location":"/tmp/test1"}}}"#,
                r#"{"snapshots":[{"snapshot":"snapshot_1","version_id":1070699,"version":"1.7.6","indices":["foo_1","foo_2"],"state":"SUCCESS","start_time":"2018-09-04T09:09:02.427Z","start_time_in_millis":1536052142427,"end_time":"2018-09-04T09:09:02.755Z","end_time_in_millis":1536052142755,"duration_in_millis":328,"failures":[],"shards":{"total":10,"failed":0,"successful":10}}]}"#,
            ),
            (
                "2.4.5",
                r#"{"test1":{"type":"fs","settings":{"location":"/tmp/test1"}}}"#,
                r#"{"snapshots":[{"snapshot":"snapshot_1","version_id":2040599,"version":"2.4.5","indices":["foo_2","foo_1"],"state":"SUCCESS","start_time":"2018-09-04T09:25:25.818Z","start_time_in_millis":1536053125818,"end_time":"2018-09-04T09:25:26.326Z","end_time_in_millis":1536053126326,"duration_in_millis":508,"failures":[],"shards":{"total":10,"failed":0,"successful":10}}]}"#,
            ),
            (
                "5.4.2",
                r#"{"test1":{"type":"fs","settings":{"location":"/tmp/test1"}}}"#,
                r#"{"snapshots":[{"snapshot":"snapshot_1","uuid":"VZ_c_kKISAW8rpcqiwSg0w","version_id":5040299,"version":"5.4.2","indices":["foo_2","foo_1"],"state":"SUCCESS","start_time":"2018-09-04T09:29:13.971Z","start_time_in_millis":1536053353971,"end_time":"2018-09-04T09:29:14.477Z","end_time_in_millis":1536053354477,"duration_in_millis":506,"failures":[],"shards":{"total":10,"failed":0,"successful":10}}]}"#,
            ),
            (
                "5.4.2-failed",
                r#"{"test1":{"type":"fs","settings":{"location":"/tmp/test1"}}}"#,
                r#"{"snapshots":[{"snapshot":"snapshot_1","uuid":"VZ_c_kKISAW8rpcqiwSg0w","version_id":5040299,"version":"5.4.2","indices":["foo_2","foo_1"],"state":"SUCCESS","start_time":"2018-09-04T09:29:13.971Z","start_time_in_millis":1536053353971,"end_time":"2018-09-04T09:29:14.477Z","end_time_in_millis":1536053354477,"duration_in_millis":506,"failures":[{"index" : "index_name","index_uuid" : "index_name","shard_id" : 52,"reason" : "IndexShardSnapshotFailedException[error deleting index file [pending-index-5] during cleanup]; nested: NoSuchFileException[Blob [pending-index-5] does not exist]; ","node_id" : "pPm9jafyTjyMk0T5A101xA","status" : "INTERNAL_SERVER_ERROR"}],"shards":{"total":10,"failed":1,"successful":10}}]}"#,
            ),
        ];

        for (_version, repos, stats) in tests {
            let repos =
                serde_json::from_str::<BTreeMap<String, SnapshotRepository>>(repos).unwrap();
            let stats = serde_json::from_str::<SnapshotsStats>(stats).unwrap();

            assert!(repos.contains_key("test1"));
            let snap_stats = &stats.snapshots[0];

            assert_eq!(snap_stats.indices.len(), 2);
            assert_eq!(snap_stats.failures.len(), snap_stats.shards.failed as usize);
            assert_eq!(snap_stats.shards.total, 10);
            assert_eq!(snap_stats.shards.successful, 10);
            assert_eq!(stats.snapshots.len(), 1);
        }
    }
}
