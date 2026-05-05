// `information_schema.client_statistics`

use event::{Metric, tags};

use super::{Connection, Error, sanitize};

const CLIENT_STAT_QUERY: &str = "SELECT * FROM information_schema.client_statistics";

const METRIC_INFOS: [(&str, &str, &str); 26] = [
    (
        "TOTAL_CONNECTIONS",
        "mysql_info_schema_client_statistics_total_connections",
        "The number of connections created for this client.",
    ),
    (
        "CONCURRENT_CONNECTIONS",
        "mysql_info_schema_client_statistics_concurrent_connections",
        "The number of concurrent connections for this client.",
    ),
    (
        "CONNECTED_TIME",
        "mysql_info_schema_client_statistics_connected_time_seconds_total",
        "The cumulative number of seconds elapsed while there were connections from this client.",
    ),
    (
        "BUSY_TIME",
        "mysql_info_schema_client_statistics_busy_seconds_total",
        "The cumulative number of seconds there was activity on connections from this client.",
    ),
    (
        "CPU_TIME",
        "mysql_info_schema_client_statistics_cpu_time_seconds_total",
        "The cumulative CPU time elapsed, in seconds, while servicing this client's connections.",
    ),
    (
        "BYTES_RECEIVED",
        "mysql_info_schema_client_statistics_bytes_received_total",
        "The number of bytes received from this client’s connections.",
    ),
    (
        "BYTES_SENT",
        "mysql_info_schema_client_statistics_bytes_sent_total",
        "The number of bytes sent to this client’s connections.",
    ),
    (
        "BINLOG_BYTES_WRITTEN",
        "mysql_info_schema_client_statistics_binlog_bytes_written_total",
        "The number of bytes written to the binary log from this client’s connections.",
    ),
    (
        "ROWS_READ",
        "mysql_info_schema_client_statistics_rows_read_total",
        "The number of rows read by this client’s connections.",
    ),
    (
        "ROWS_SENT",
        "mysql_info_schema_client_statistics_rows_sent_total",
        "The number of rows sent by this client’s connections.",
    ),
    (
        "ROWS_DELETED",
        "mysql_info_schema_client_statistics_rows_deleted_total",
        "The number of rows deleted by this client’s connections.",
    ),
    (
        "ROWS_INSERTED",
        "mysql_info_schema_client_statistics_rows_inserted_total",
        "The number of rows inserted by this client’s connections.",
    ),
    (
        "ROWS_FETCHED",
        "mysql_info_schema_client_statistics_rows_fetched_total",
        "The number of rows fetched by this client’s connections.",
    ),
    (
        "ROWS_UPDATED",
        "mysql_info_schema_client_statistics_rows_updated_total",
        "The number of rows updated by this client’s connections.",
    ),
    (
        "TABLE_ROWS_READ",
        "mysql_info_schema_client_statistics_table_rows_read_total",
        "The number of rows read from tables by this client’s connections. (It may be different from ROWS_FETCHED.)",
    ),
    (
        "SELECT_COMMANDS",
        "mysql_info_schema_client_statistics_select_commands_total",
        "The number of SELECT commands executed from this client’s connections.",
    ),
    (
        "UPDATE_COMMANDS",
        "mysql_info_schema_client_statistics_update_commands_total",
        "The number of UPDATE commands executed from this client’s connections.",
    ),
    (
        "OTHER_COMMANDS",
        "mysql_info_schema_client_statistics_other_commands_total",
        "The number of other commands executed from this client’s connections.",
    ),
    (
        "COMMIT_TRANSACTIONS",
        "mysql_info_schema_client_statistics_commit_transactions_total",
        "The number of COMMIT commands issued by this client’s connections.",
    ),
    (
        "ROLLBACK_TRANSACTIONS",
        "mysql_info_schema_client_statistics_rollback_transactions_total",
        "The number of ROLLBACK commands issued by this client’s connections.",
    ),
    (
        "DENIED_CONNECTIONS",
        "mysql_info_schema_client_statistics_denied_connections_total",
        "The number of connections denied to this client.",
    ),
    (
        "LOST_CONNECTIONS",
        "mysql_info_schema_client_statistics_lost_connections_total",
        "The number of this client’s connections that were terminated uncleanly.",
    ),
    (
        "ACCESS_DENIED",
        "mysql_info_schema_client_statistics_access_denied_total",
        "The number of times this client’s connections issued commands that were denied.",
    ),
    (
        "EMPTY_QUERIES",
        "mysql_info_schema_client_statistics_empty_queries_total",
        "The number of times this client’s connections sent empty queries to the server.",
    ),
    (
        "TOTAL_SSL_CONNECTIONS",
        "mysql_info_schema_client_statistics_total_ssl_connections_total",
        "The number of times this client’s connections connected using SSL to the server.",
    ),
    (
        "MAX_STATEMENT_TIME_EXCEEDED",
        "mysql_info_schema_client_statistics_max_statement_time_exceeded_total",
        "The number of times a statement was aborted, because it was executed longer than its MAX_STATEMENT_TIME threshold.",
    ),
];

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut rows = conn.query(CLIENT_STAT_QUERY).await?;

    let mut metrics = vec![];
    while let Some(mut row) = rows.next().await? {
        let columns = row.columns();
        let client = row.get_str();

        for column in columns.iter().skip(1) {
            let value = row.get_str().parse::<f64>()?;

            match METRIC_INFOS.iter().find(|info| info.0 == column.name()) {
                Some((_, name, desc)) => {
                    if column.name() == "CONCURRENT_CONNECTIONS" {
                        metrics.push(Metric::gauge_with_tags(
                            *name,
                            *desc,
                            value,
                            tags!("client" => client),
                        ))
                    } else {
                        metrics.push(Metric::sum_with_tags(
                            *name,
                            *desc,
                            value,
                            tags!("client" => client),
                        ))
                    }
                }
                None => {
                    let name = format!(
                        "mysql_info_schema_client_statistics_{}",
                        sanitize(column.name())
                    );
                    let desc = format!("Unsupported metric from column {}", column.name());

                    metrics.push(Metric::gauge_with_tags(
                        name,
                        desc,
                        value,
                        tags!("client" => client),
                    ))
                }
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

    #[tokio::test]
    async fn client_stats() {
        let mut conn = mock(|_query| {
            (
                vec![
                    "CLIENT",
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
                    "localhost",
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
            )
        })
        .await;

        let metrics = collect(&mut conn).await.unwrap();
        assert_contains(
            &metrics,
            vec![],
            vec![
                (tags!("client" => "localhost"), 1002.0),
                (tags!("client" => "localhost"), 0.0),
                (tags!("client" => "localhost"), 127027.0),
                (tags!("client" => "localhost"), 286.0),
                (tags!("client" => "localhost"), 245.0),
                (tags!("client" => "localhost"), 2565104853.0),
                (tags!("client" => "localhost"), 21090856.0),
                (tags!("client" => "localhost"), 2380108042.0),
                (tags!("client" => "localhost"), 767691.0),
                (tags!("client" => "localhost"), 1764.0),
                (tags!("client" => "localhost"), 8778.0),
                (tags!("client" => "localhost"), 1210741.0),
                (tags!("client" => "localhost"), 0.0),
                (tags!("client" => "localhost"), 1764.0),
                (tags!("client" => "localhost"), 1214416.0),
                (tags!("client" => "localhost"), 293.0),
                (tags!("client" => "localhost"), 2430888.0),
                (tags!("client" => "localhost"), 0.0),
                (tags!("client" => "localhost"), 0.0),
                (tags!("client" => "localhost"), 0.0),
                (tags!("client" => "localhost"), 0.0),
                (tags!("client" => "localhost"), 0.0),
            ],
        );
    }
}
