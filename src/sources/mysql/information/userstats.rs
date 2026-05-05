// `information_schema.user_statistics`

use event::{Metric, tags};

use super::{Connection, Error, sanitize};

const GAUGE_METRI_INFOS: [(&str, &str, &str); 1] = [(
    "CONCURRENT_CONNECTIONS",
    "mysql_info_schema_user_statistics_concurrent_connections",
    "The number of concurrent connections for this user.",
)];

const COUNTER_METRIC_INFOS: [(&str, &str, &str); 24] = [
    (
        "TOTAL_CONNECTIONS",
        "mysql_info_schema_user_statistics_total_connections",
        "The number of connections created for this user.",
    ),
    (
        "CONNECTED_TIME",
        "mysql_info_schema_user_statistics_connected_time_seconds_total",
        "The cumulative number of seconds elapsed while there were connections from this user.",
    ),
    (
        "BUSY_TIME",
        "mysql_info_schema_user_statistics_busy_seconds_total",
        "The cumulative number of seconds there was activity on connections from this user.",
    ),
    (
        "CPU_TIME",
        "mysql_info_schema_user_statistics_cpu_time_seconds_total",
        "The cumulative CPU time elapsed, in seconds, while servicing this user's connections.",
    ),
    (
        "BYTES_RECEIVED",
        "mysql_info_schema_user_statistics_bytes_received_total",
        "The number of bytes received from this user’s connections.",
    ),
    (
        "BYTES_SENT",
        "mysql_info_schema_user_statistics_bytes_sent_total",
        "The number of bytes sent to this user’s connections.",
    ),
    (
        "BINLOG_BYTES_WRITTEN",
        "mysql_info_schema_user_statistics_binlog_bytes_written_total",
        "The number of bytes written to the binary log from this user’s connections.",
    ),
    (
        "ROWS_READ",
        "mysql_info_schema_user_statistics_rows_read_total",
        "The number of rows read by this user's connections.",
    ),
    (
        "ROWS_SENT",
        "mysql_info_schema_user_statistics_rows_sent_total",
        "The number of rows sent by this user's connections.",
    ),
    (
        "ROWS_DELETED",
        "mysql_info_schema_user_statistics_rows_deleted_total",
        "The number of rows deleted by this user's connections.",
    ),
    (
        "ROWS_INSERTED",
        "mysql_info_schema_user_statistics_rows_inserted_total",
        "The number of rows inserted by this user's connections.",
    ),
    (
        "ROWS_FETCHED",
        "mysql_info_schema_user_statistics_rows_fetched_total",
        "The number of rows fetched by this user’s connections.",
    ),
    (
        "ROWS_UPDATED",
        "mysql_info_schema_user_statistics_rows_updated_total",
        "The number of rows updated by this user’s connections.",
    ),
    (
        "TABLE_ROWS_READ",
        "mysql_info_schema_user_statistics_table_rows_read_total",
        "The number of rows read from tables by this user’s connections. (It may be different from ROWS_FETCHED.)",
    ),
    (
        "SELECT_COMMANDS",
        "mysql_info_schema_user_statistics_select_commands_total",
        "The number of SELECT commands executed from this user’s connections.",
    ),
    (
        "UPDATE_COMMANDS",
        "mysql_info_schema_user_statistics_update_commands_total",
        "The number of UPDATE commands executed from this user’s connections.",
    ),
    (
        "OTHER_COMMANDS",
        "mysql_info_schema_user_statistics_other_commands_total",
        "The number of other commands executed from this user’s connections.",
    ),
    (
        "COMMIT_TRANSACTIONS",
        "mysql_info_schema_user_statistics_commit_transactions_total",
        "The number of COMMIT commands issued by this user’s connections.",
    ),
    (
        "ROLLBACK_TRANSACTIONS",
        "mysql_info_schema_user_statistics_rollback_transactions_total",
        "The number of ROLLBACK commands issued by this user’s connections.",
    ),
    (
        "DENIED_CONNECTIONS",
        "mysql_info_schema_user_statistics_denied_connections_total",
        "The number of connections denied to this user.",
    ),
    (
        "LOST_CONNECTIONS",
        "mysql_info_schema_user_statistics_lost_connections_total",
        "The number of this user’s connections that were terminated uncleanly.",
    ),
    (
        "ACCESS_DENIED",
        "mysql_info_schema_user_statistics_access_denied_total",
        "The number of times this user’s connections issued commands that were denied.",
    ),
    (
        "EMPTY_QUERIES",
        "mysql_info_schema_user_statistics_empty_queries_total",
        "The number of times this user’s connections sent empty queries to the server.",
    ),
    (
        "TOTAL_SSL_CONNECTIONS",
        "mysql_info_schema_user_statistics_total_ssl_connections_total",
        "The number of times this user’s connections connected using SSL to the server.",
    ),
];

const USERSTAT_QUERY: &str = "SELECT * FROM information_schema.user_statistics";

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut rows = conn.query(USERSTAT_QUERY).await?;

    let mut metrics = vec![];
    while let Some(mut row) = rows.next().await? {
        let user = row.get_str();

        for column in row.columns().iter().skip(1) {
            let value = row.get_str().parse::<f64>()?;

            if let Some((_, name, desc)) = COUNTER_METRIC_INFOS
                .iter()
                .find(|item| item.0 == column.name())
            {
                metrics.push(Metric::sum_with_tags(
                    *name,
                    *desc,
                    value,
                    tags!("user" => user),
                ))
            } else if let Some((_, name, desc)) = GAUGE_METRI_INFOS
                .iter()
                .find(|item| item.0 == column.name())
            {
                metrics.push(Metric::gauge_with_tags(
                    *name,
                    *desc,
                    value,
                    tags!("user" => user),
                ))
            } else {
                metrics.push(Metric::gauge_with_tags(
                    format!(
                        "mysql_info_schema_user_statistics_{}",
                        sanitize(column.name())
                    ),
                    format!("Unsupported metric from column {}", column.name()),
                    value,
                    tags!("user" => user),
                ))
            }
        }
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::super::USERSTAT_CHECK_QUERY;
    use super::*;
    use crate::sources::mysql::assert_contains;
    use crate::sources::mysql::connection::mock;

    #[tokio::test]
    async fn smoke() {
        let mut conn = mock(|query| match query {
            USERSTAT_CHECK_QUERY => (vec!["Variable_name", "Value"], vec![vec!["userstat", "ON"]]),
            _ => (
                vec![
                    "USER",
                    "TOTAL_CONNECTIONS",
                    "CONCURRENT_CONNECTIONS",
                    "CONNECTED_TIME",
                    "BUSY_TIME",
                    "CPU_TIME",
                    "BYTES_RECEIVED",
                    "BYTES_SENT",
                    "BINLOG_BYTES_WRITTEN",
                    "ROWS_READ",
                    "ROWS_SENT",
                    "ROWS_DELETED",
                    "ROWS_INSERTED",
                    "ROWS_UPDATED",
                    "SELECT_COMMANDS",
                    "UPDATE_COMMANDS",
                    "OTHER_COMMANDS",
                    "COMMIT_TRANSACTIONS",
                    "ROLLBACK_TRANSACTIONS",
                    "DENIED_CONNECTIONS",
                    "LOST_CONNECTIONS",
                    "ACCESS_DENIED",
                    "EMPTY_QUERIES",
                ],
                vec![vec![
                    "user_test",
                    "1002",
                    "0",
                    "127027",
                    "286",
                    "245",
                    "2565104853",
                    "21090856",
                    "2380108042",
                    "767691",
                    "1764",
                    "8778",
                    "1210741",
                    "0",
                    "1764",
                    "1214416",
                    "293",
                    "2430888",
                    "0",
                    "0",
                    "0",
                    "0",
                    "0",
                ]],
            ),
        })
        .await;

        let metrics = collect(&mut conn).await.unwrap();
        assert_contains(
            &metrics,
            vec![(tags!("user" => "user_test"), 0.0)],
            vec![
                (tags!("user" => "user_test"), 1002.0),
                (tags!("user" => "user_test"), 127027.0),
                (tags!("user" => "user_test"), 286.0),
                (tags!("user" => "user_test"), 245.0),
                (tags!("user" => "user_test"), 2565104853.0),
                (tags!("user" => "user_test"), 21090856.0),
                (tags!("user" => "user_test"), 2380108042.0),
                (tags!("user" => "user_test"), 767691.0),
                (tags!("user" => "user_test"), 1764.0),
                (tags!("user" => "user_test"), 8778.0),
                (tags!("user" => "user_test"), 1210741.0),
                (tags!("user" => "user_test"), 0.0),
                (tags!("user" => "user_test"), 1764.0),
                (tags!("user" => "user_test"), 1214416.0),
                (tags!("user" => "user_test"), 293.0),
                (tags!("user" => "user_test"), 2430888.0),
                (tags!("user" => "user_test"), 0.0),
                (tags!("user" => "user_test"), 0.0),
                (tags!("user" => "user_test"), 0.0),
                (tags!("user" => "user_test"), 0.0),
                (tags!("user" => "user_test"), 0.0),
            ],
        )
    }
}
