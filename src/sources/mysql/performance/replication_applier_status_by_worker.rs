// `performance_schema.replication_group_members`

use chrono::NaiveDateTime;
use event::{Metric, tags};

use super::{Connection, Error};

const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.6f";

const REPLICATION_APPLIER_STATS_BY_WORKER_QUERY: &str = "SELECT
  CHANNEL_NAME,
  WORKER_ID,
  LAST_APPLIED_TRANSACTION_ORIGINAL_COMMIT_TIMESTAMP,
  LAST_APPLIED_TRANSACTION_IMMEDIATE_COMMIT_TIMESTAMP,
  LAST_APPLIED_TRANSACTION_START_APPLY_TIMESTAMP,
  LAST_APPLIED_TRANSACTION_END_APPLY_TIMESTAMP,
  APPLYING_TRANSACTION_ORIGINAL_COMMIT_TIMESTAMP,
  APPLYING_TRANSACTION_IMMEDIATE_COMMIT_TIMESTAMP,
  APPLYING_TRANSACTION_START_APPLY_TIMESTAMP
FROM performance_schema.replication_applier_status_by_worker";

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut rows = conn
        .query(REPLICATION_APPLIER_STATS_BY_WORKER_QUERY)
        .await?;

    let mut metrics = vec![];
    while let Some(mut row) = rows.next().await? {
        let channel = row.get_str();
        let worker = row.get_str();

        let last_applied_transaction_original_commit =
            NaiveDateTime::parse_from_str(row.get_str(), TIMESTAMP_FORMAT)
                .map(|datetime| datetime.and_utc().timestamp_micros() as f64 / 1e6)
                .unwrap_or_default();
        let last_applied_transaction_immediate_commit =
            NaiveDateTime::parse_from_str(row.get_str(), TIMESTAMP_FORMAT)
                .map(|datetime| datetime.and_utc().timestamp_micros() as f64 / 1e6)
                .unwrap_or_default();
        let last_applied_transaction_start_apply =
            NaiveDateTime::parse_from_str(row.get_str(), TIMESTAMP_FORMAT)
                .map(|datetime| datetime.and_utc().timestamp_micros() as f64 / 1e6)
                .unwrap_or_default();
        let last_applied_transaction_end_apply =
            NaiveDateTime::parse_from_str(row.get_str(), TIMESTAMP_FORMAT)
                .map(|datetime| datetime.and_utc().timestamp_micros() as f64 / 1e6)
                .unwrap_or_default();
        let applying_transaction_original_commit =
            NaiveDateTime::parse_from_str(row.get_str(), TIMESTAMP_FORMAT)
                .map(|datetime| datetime.and_utc().timestamp_micros() as f64 / 1e6)
                .unwrap_or_default();
        let applying_transaction_immediate_commit =
            NaiveDateTime::parse_from_str(row.get_str(), TIMESTAMP_FORMAT)
                .map(|datetime| datetime.and_utc().timestamp_micros() as f64 / 1e6)
                .unwrap_or_default();
        let applying_transaction_start_apply =
            NaiveDateTime::parse_from_str(row.get_str(), TIMESTAMP_FORMAT)
                .map(|datetime| datetime.and_utc().timestamp_micros() as f64 / 1e6)
                .unwrap_or_default();

        metrics.extend([
            Metric::gauge_with_tags(
                "mysql_perf_schema_last_applied_transaction_original_commit_timestamp_seconds",
                "A timestamp shows when the last transaction applied by this worker was committed on the original master.",
                last_applied_transaction_original_commit,
                tags!("channel" => channel, "member_id" => worker),
            ),
            Metric::gauge_with_tags(
                "mysql_perf_schema_last_applied_transaction_immediate_commit_timestamp_seconds",
                "A timestamp shows when the last transaction applied by this worker was committed on the immediate master.",
                last_applied_transaction_immediate_commit,
                tags!("channel" => channel, "member_id" => worker),
            ),
            Metric::gauge_with_tags(
                "mysql_perf_schema_last_applied_transaction_start_apply_timestamp_seconds",
                "A timestamp shows when this worker started applying the last applied transaction.",
                last_applied_transaction_start_apply,
                tags!("channel" => channel, "member_id" => worker),
            ),
            Metric::gauge_with_tags(
                "mysql_perf_schema_last_applied_transaction_end_apply_timestamp_seconds",
                "A shows when this worker finished applying the last applied transaction.",
                last_applied_transaction_end_apply,
                tags!("channel" => channel, "member_id" => worker),
            ),
            Metric::gauge_with_tags(
                "mysql_perf_schema_applying_transaction_original_commit_timestamp_seconds",
                "A timestamp that shows when the transaction this worker is currently applying was committed on the original master.",
                applying_transaction_original_commit,
                tags!("channel" => channel, "member_id" => worker),
            ),
            Metric::gauge_with_tags(
                "mysql_perf_schema_applying_transaction_immediate_commit_timestamp_seconds",
                "A timestamp shows when the transaction this worker is currently applying was committed on the immediate master.",
                applying_transaction_immediate_commit,
                tags!("channel" => channel, "member_id" => worker),
            ),
            Metric::gauge_with_tags(
                "mysql_perf_schema_applying_transaction_start_apply_timestamp_seconds",
                "A timestamp shows when this worker started its first attempt to apply the transaction that is currently being applied.",
                applying_transaction_start_apply,
                tags!("channel" => channel, "member_id" => worker),
            )
        ]);
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::mysql::assert_contains;
    use crate::sources::mysql::connection::mock;
    use event::tags;

    const TIME_ZERO: &str = "0000-00-00 00:00:00.000000";

    #[tokio::test]
    async fn smoke() {
        let mut conn = mock(|_| {
            (
                vec![
                    "CHANNEL_NAME",
                    "WORKER_ID",
                    "LAST_APPLIED_TRANSACTION_ORIGINAL_COMMIT_TIMESTAMP",
                    "LAST_APPLIED_TRANSACTION_IMMEDIATE_COMMIT_TIMESTAMP",
                    "LAST_APPLIED_TRANSACTION_START_APPLY_TIMESTAMP",
                    "LAST_APPLIED_TRANSACTION_END_APPLY_TIMESTAMP",
                    "APPLYING_TRANSACTION_ORIGINAL_COMMIT_TIMESTAMP",
                    "APPLYING_TRANSACTION_IMMEDIATE_COMMIT_TIMESTAMP",
                    "APPLYING_TRANSACTION_START_APPLY_TIMESTAMP",
                ],
                vec![
                    vec![
                        "dummy_0", "0", TIME_ZERO, TIME_ZERO, TIME_ZERO, TIME_ZERO, TIME_ZERO,
                        TIME_ZERO, TIME_ZERO,
                    ],
                    vec![
                        "dummy_1",
                        "1",
                        "2019-03-14 00:00:00.001000",
                        "2019-03-14 00:01:00.001000",
                        "2019-03-14 00:02:00.001000",
                        "2019-03-14 00:03:00.001000",
                        "2019-03-14 00:04:00.001000",
                        "2019-03-14 00:05:00.001000",
                        "2019-03-14 00:06:00.001000",
                    ],
                ],
            )
        })
        .await;

        let metrics = collect(&mut conn).await.unwrap();
        assert_contains(
            &metrics,
            vec![
                (tags!("channel" => "dummy_0", "member_id" => "0"), 0.0),
                (tags!("channel" => "dummy_0", "member_id" => "0"), 0.0),
                (tags!("channel" => "dummy_0", "member_id" => "0"), 0.0),
                (tags!("channel" => "dummy_0", "member_id" => "0"), 0.0),
                (tags!("channel" => "dummy_0", "member_id" => "0"), 0.0),
                (tags!("channel" => "dummy_0", "member_id" => "0"), 0.0),
                (tags!("channel" => "dummy_0", "member_id" => "0"), 0.0),
                (
                    tags!("channel" => "dummy_1", "member_id" => "1"),
                    1.552521600001e+9,
                ),
                (
                    tags!("channel" => "dummy_1", "member_id" => "1"),
                    1.552521660001e+9,
                ),
                (
                    tags!("channel" => "dummy_1", "member_id" => "1"),
                    1.552521720001e+9,
                ),
                (
                    tags!("channel" => "dummy_1", "member_id" => "1"),
                    1.552521780001e+9,
                ),
                (
                    tags!("channel" => "dummy_1", "member_id" => "1"),
                    1.552521840001e+9,
                ),
                (
                    tags!("channel" => "dummy_1", "member_id" => "1"),
                    1.552521900001e+9,
                ),
                (
                    tags!("channel" => "dummy_1", "member_id" => "1"),
                    1.552521960001e+9,
                ),
            ],
            vec![],
        );
    }
}
