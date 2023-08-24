use super::Elasticsearch;
use event::{tags, Metric};
use serde::Deserialize;

/// `PolicyStats` is representation of SLM stats for specific policies.
#[derive(Deserialize)]
struct PolicyStats {
    policy: String,
    snapshots_taken: i64,
    snapshots_failed: i64,
    snapshots_deleted: i64,
    snapshot_deletion_failures: i64,
}

#[allow(dead_code)]
/// `SlmStats` is representation of the SLM stats.
#[derive(Deserialize)]
struct SlmStats {
    retention_runs: i64,
    retention_failed: i64,
    retention_timed_out: i64,
    retention_deletion_time: String,
    retention_deletion_time_millis: i64,
    total_snapshots_taken: i64,
    total_snapshots_failed: i64,
    total_snapshots_deleted: i64,
    total_snapshot_deletion_failures: i64,
    #[serde(default)]
    policy_stats: Vec<PolicyStats>,
}

/// `SlmStatus` is representation of the SLM status
#[derive(Deserialize)]
struct SlmStatus {
    operation_mode: String,
}

impl Elasticsearch {
    pub async fn slm(&self) -> Vec<Metric> {
        let result = self.slm_stats_and_status().await;
        let up = result.is_ok();
        if !up {
            return vec![Metric::gauge(
                "elasticsearch_slm_stats_up",
                "Was the last scrape of the Elasticsearch SLM endpoint successful",
                0,
            )];
        }

        match result {
            Ok((stats, status)) => {
                let mut metrics = vec![];

                for s in ["RUNNING", "STOPPING", "STOPPED"] {
                    metrics.push(Metric::gauge_with_tags(
                        "elasticsearch_slm_stats_operation_mode",
                        "Operating status of SLM",
                        status.operation_mode == s,
                        tags!(
                            "operation_mode" => s,
                        ),
                    ));
                }

                metrics.extend_from_slice(&[
                    Metric::sum(
                        "elasticsearch_slm_stats_retention_runs_total",
                        "Total retention runs",
                        stats.retention_runs as f64,
                    ),
                    Metric::sum(
                        "elasticsearch_slm_stats_retention_failed_total",
                        "Total failed retention runs",
                        stats.retention_failed as f64,
                    ),
                    Metric::sum(
                        "elasticsearch_slm_stats_retention_timed_out_total",
                        "Total timed out retention runs",
                        stats.retention_timed_out as f64,
                    ),
                    Metric::gauge(
                        "elasticsearch_slm_stats_retention_deletion_time_seconds",
                        "Retention run deletion time",
                        stats.retention_deletion_time_millis / 1000,
                    ),
                    Metric::sum(
                        "elasticsearch_slm_stats_total_snapshots_taken_total",
                        "Total snapshots taken",
                        stats.total_snapshots_taken as f64,
                    ),
                    Metric::sum(
                        "elasticsearch_slm_stats_total_snapshots_failed_total",
                        "Total snapshots failed",
                        stats.total_snapshots_failed as f64,
                    ),
                    Metric::sum(
                        "elasticsearch_slm_stats_total_snapshots_deleted_total",
                        "Total snapshots_deleted",
                        stats.total_snapshots_deleted as f64,
                    ),
                    Metric::gauge(
                        "elasticsearch_slm_stats_slm_stats_total_snapshot_deletion_failures_total",
                        "Total snapshot deletion failures",
                        stats.total_snapshot_deletion_failures,
                    ),
                ]);

                for policy in stats.policy_stats {
                    let tags = tags!(
                        "policy" => policy.policy
                    );

                    metrics.extend_from_slice(&[
                        Metric::sum_with_tags(
                            "elasticsearch_slm_stats_snapshot_taken_total",
                            "Total snapshots taken",
                            policy.snapshots_taken,
                            tags.clone(),
                        ),
                        Metric::sum_with_tags(
                            "elasticsearch_slm_stats_snapshots_failed",
                            "Total snapshots failed",
                            policy.snapshots_failed,
                            tags.clone(),
                        ),
                        Metric::sum_with_tags(
                            "elasticsearch_slm_stats_snapshots_deleted_total",
                            "Total snapshots deleted",
                            policy.snapshots_deleted,
                            tags.clone(),
                        ),
                        Metric::sum_with_tags(
                            "elasticsearch_slm_stats_snapshots_deletion_failures_total",
                            "Total snapshot deletion failures",
                            policy.snapshot_deletion_failures,
                            tags,
                        ),
                    ]);
                }

                metrics
            }
            Err(err) => {
                warn!(message = "", ?err);

                vec![]
            }
        }
    }

    async fn slm_stats_and_status(&self) -> Result<(SlmStats, SlmStatus), crate::Error> {
        let stats = self.fetch("/_slm/stats").await?;
        let status = self.fetch("/_slm/status").await?;

        Ok((stats, status))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode() {
        let tests = [(
            "7.15.0",
            r#"{"retention_runs":9,"retention_failed":0,"retention_timed_out":0,"retention_deletion_time":"1.2m","retention_deletion_time_millis":72491,"total_snapshots_taken":103,"total_snapshots_failed":2,"total_snapshots_deleted":20,"total_snapshot_deletion_failures":0,"policy_stats":[{"policy":"everything","snapshots_taken":50,"snapshots_failed":2,"snapshots_deleted":20,"snapshot_deletion_failures":0}]}"#,
        )];

        for (_version, input) in tests {
            let stats = serde_json::from_str::<SlmStats>(input).unwrap();

            assert_eq!(stats.total_snapshots_taken, 103);
            assert_eq!(stats.policy_stats.first().unwrap().snapshots_taken, 50);
        }
    }
}
