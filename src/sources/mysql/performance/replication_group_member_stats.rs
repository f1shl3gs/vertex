// `performance_schema.replication_group_members`

use event::Metric;

use super::{Connection, Error};

const REPLICATION_GROUP_MEMBER_STATS_QUERY: &str =
    "SELECT * FROM performance_schema.replication_group_member_stats WHERE MEMBER_ID=@@server_uuid";

const GAUGE_METRIC_INFOS: [(&str, &str, &str); 2] = [
    (
        "COUNT_TRANSACTIONS_IN_QUEUE",
        "mysql_perf_schema_transactions_in_queue",
        "The number of transactions in the queue pending conflict detection checks.",
    ),
    (
        "COUNT_TRANSACTIONS_REMOTE_IN_APPLIER_QUEUE",
        "mysql_perf_schema_transactions_remote_in_applier_queue",
        "The number of transactions that this member has received from the replication group which are waiting to be applied.",
    ),
];

const COUNTER_METRIC_INFOS: [(&str, &str, &str); 6] = [
    (
        "COUNT_TRANSACTIONS_CHECKED",
        "mysql_perf_schema_transactions_checked_total",
        "The number of transactions that have been checked for conflicts.",
    ),
    (
        "COUNT_CONFLICTS_DETECTED",
        "mysql_perf_schema_conflicts_detected_total",
        "The number of transactions that have not passed the conflict detection check.",
    ),
    (
        "COUNT_TRANSACTIONS_ROWS_VALIDATING",
        "mysql_perf_schema_transactions_rows_validating_total",
        "Number of transaction rows which can be used for certification, but have not been garbage collected.",
    ),
    (
        "COUNT_TRANSACTIONS_REMOTE_APPLIED",
        "mysql_perf_schema_transactions_remote_applied_total",
        "Number of transactions this member has received from the group and applied.",
    ),
    (
        "COUNT_TRANSACTIONS_LOCAL_PROPOSED",
        "mysql_perf_schema_transactions_local_proposed_total",
        "Number of transactions which originated on this member and were sent to the group.",
    ),
    (
        "COUNT_TRANSACTIONS_LOCAL_ROLLBACK",
        "mysql_perf_schema_transactions_local_rollback_total",
        "Number of transactions which originated on this member and were rolled back by the group.",
    ),
];

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut rows = conn.query(REPLICATION_GROUP_MEMBER_STATS_QUERY).await?;

    let mut metrics = vec![];
    while let Some(mut row) = rows.next().await? {
        for column in row.columns() {
            let value = row.get_str();

            if let Some((_, name, desc)) = GAUGE_METRIC_INFOS
                .iter()
                .find(|item| item.0 == column.name())
            {
                let value = value.parse::<f64>()?;

                metrics.push(Metric::gauge(*name, *desc, value));
            } else if let Some((_, name, desc)) = COUNTER_METRIC_INFOS
                .iter()
                .find(|item| item.0 == column.name())
            {
                let value = value.parse::<f64>()?;

                metrics.push(Metric::sum(*name, *desc, value))
            }
        }
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::mysql::assert_contains;
    use crate::sources::mysql::connection::mock;
    use event::tags;

    #[tokio::test]
    async fn smoke() {
        let mut conn = mock(|_query| {
            (
                vec![
                    "CHANNEL_NAME",
                    "VIEW_ID",
                    "MEMBER_ID",
                    "COUNT_TRANSACTIONS_IN_QUEUE",
                    "COUNT_TRANSACTIONS_CHECKED",
                    "COUNT_CONFLICTS_DETECTED",
                    "COUNT_TRANSACTIONS_ROWS_VALIDATING",
                    "TRANSACTIONS_COMMITTED_ALL_MEMBERS",
                    "LAST_CONFLICT_FREE_TRANSACTION",
                    "COUNT_TRANSACTIONS_REMOTE_IN_APPLIER_QUEUE",
                    "COUNT_TRANSACTIONS_REMOTE_APPLIED",
                    "COUNT_TRANSACTIONS_LOCAL_PROPOSED",
                    "COUNT_TRANSACTIONS_LOCAL_ROLLBACK",
                ],
                vec![
                    vec![
                        "group_replication_applier",
                        "15813535259046852:43",
                        "e14c4f71-025f-11ea-b800-0620049edbec",
                        "0",
                        "7389775",
                        "1",
                        "48",
                        "0515b3c2-f59f-11e9-881b-0620049edbec:1-15270987,\n8f782839-34f7-11e7-a774-060ac4f023ae:4-39:2387-161606",
                        "0515b3c2-f59f-11e9-881b-0620049edbec:15271011",
                        "2",
                        "22",
                        "7389759",
                        "7",
                    ]
                ]
            )
        }).await;

        let metrics = collect(&mut conn).await.unwrap();
        assert_contains(
            &metrics,
            vec![(tags!(), 0.0), (tags!(), 2.0)],
            vec![
                (tags!(), 7389775.0),
                (tags!(), 1.0),
                (tags!(), 48.0),
                (tags!(), 22.0),
                (tags!(), 7389759.0),
                (tags!(), 7.0),
            ],
        );
    }
}
