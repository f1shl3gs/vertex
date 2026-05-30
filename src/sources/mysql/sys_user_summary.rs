use event::{Metric, tags};

use super::connection::{Connection, Error};

const SYS_USER_SUMMARY_QUERY: &str = "SELECT
  user,
  statements,
  statement_latency,
  table_scans,
  file_ios,
  file_io_latency,
  current_connections,
  total_connections,
  unique_hosts,
  current_memory,
  total_memory_allocated
FROM sys.x$user_summary";

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut rows = conn.query(SYS_USER_SUMMARY_QUERY).await?;

    let mut metrics = vec![];
    while let Some(mut row) = rows.next().await? {
        let user = row.get_str();
        let statements = row.get_str().parse::<u64>()?;
        let statement_latency = row.get_str().parse::<f64>()?;
        let table_scans = row.get_str().parse::<u64>()?;
        let file_ios = row.get_str().parse::<u64>()?;
        let file_io_latency = row.get_str().parse::<f64>()?;
        let current_connections = row.get_str().parse::<u64>()?;
        let total_connections = row.get_str().parse::<u64>()?;
        let unique_hosts = row.get_str().parse::<u64>()?;
        let current_memory = row.get_str().parse::<i64>()?;
        let total_memory_allocated = row.get_str().parse::<u64>()?;

        metrics.extend([
            Metric::sum_with_tags(
                "mysql_sys_statements_total",
                "The total number of statements for the user",
                statements,
                tags!("user" => user),
            ),
            Metric::sum_with_tags(
                "mysql_sys_statement_latency",
                "The total wait time of timed statements for the user",
                statement_latency / 1e12,
                tags!("user" => user),
            ),
            Metric::sum_with_tags(
                "mysql_sys_table_scans_total",
                "The total number of table scans for the user",
                table_scans,
                tags!("user" => user),
            ),
            Metric::sum_with_tags(
                "mysql_sys_file_ios_total",
                "The total number of file I/O events for the user",
                file_ios,
                tags!("user" => user),
            ),
            Metric::sum_with_tags(
                "mysql_sys_file_io_seconds_total",
                "The total wait time of timed file I/O events for the user",
                file_io_latency / 1e12,
                tags!("user" => user),
            ),
            Metric::gauge_with_tags(
                "mysql_sys_current_connections",
                "The current number of connections for the user",
                current_connections,
                tags!("user" => user),
            ),
            Metric::sum_with_tags(
                "mysql_sys_connections_total",
                "The total number of connections for the user",
                total_connections,
                tags!("user" => user),
            ),
            Metric::sum_with_tags(
                "mysql_sys_unique_hosts_total",
                "The number of distinct hosts from which connections for the user have originated",
                unique_hosts,
                tags!("user" => user),
            ),
            Metric::gauge_with_tags(
                "mysql_sys_current_memory_bytes",
                "The current amount of allocated memory for the user",
                current_memory,
                tags!("user" => user),
            ),
            Metric::sum_with_tags(
                "mysql_sys_memory_allocated_bytes_total",
                "The total amount of allocated memory for the user",
                total_memory_allocated,
                tags!("user" => user),
            ),
        ]);
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use event::MetricValue;

    use super::*;
    use crate::sources::mysql::connection::mock;

    #[tokio::test]
    async fn smoke() {
        let mut conn = mock(|_| {
            (
                vec![
                    "user",
                    "statemets",
                    "statement_latency",
                    "table_scans",
                    "file_ios",
                    "file_io_latency",
                    "current_connections",
                    "total_connections",
                    "unique_hosts",
                    "current_memory",
                    "total_memory_allocated",
                ],
                vec![
                    vec![
                        "user1", "110", "120", "140", "150", "160", "170", "180", "190", "110",
                        "111",
                    ],
                    vec![
                        "user2", "210", "220", "240", "250", "260", "270", "280", "290", "210",
                        "211",
                    ],
                    vec![
                        "user3", "310", "320", "340", "350", "360", "370", "380", "390", "-16360",
                        "411",
                    ],
                ],
            )
        })
        .await;

        let metrics = collect(&mut conn).await.unwrap();
        let want = [
            (1, tags!("user" => "user1"), 110.0),
            (1, tags!("user" => "user1"), 1.2e-10),
            (1, tags!("user" => "user1"), 140.0),
            (1, tags!("user" => "user1"), 150.0),
            (1, tags!("user" => "user1"), 1.6e-10),
            (2, tags!("user" => "user1"), 170.0),
            (1, tags!("user" => "user1"), 180.0),
            (1, tags!("user" => "user1"), 190.0),
            (2, tags!("user" => "user1"), 110.0),
            (1, tags!("user" => "user1"), 111.0),
            (1, tags!("user" => "user2"), 210.0),
            (1, tags!("user" => "user2"), 2.2e-10),
            (1, tags!("user" => "user2"), 240.0),
            (1, tags!("user" => "user2"), 250.0),
            (1, tags!("user" => "user2"), 2.6e-10),
            (2, tags!("user" => "user2"), 270.0),
            (1, tags!("user" => "user2"), 280.0),
            (1, tags!("user" => "user2"), 290.0),
            (2, tags!("user" => "user2"), 210.0),
            (1, tags!("user" => "user2"), 211.0),
            (1, tags!("user" => "user3"), 310.0),
            (1, tags!("user" => "user3"), 3.2e-10),
            (1, tags!("user" => "user3"), 340.0),
            (1, tags!("user" => "user3"), 350.0),
            (1, tags!("user" => "user3"), 3.6e-10),
            (2, tags!("user" => "user3"), 370.0),
            (1, tags!("user" => "user3"), 380.0),
            (1, tags!("user" => "user3"), 390.0),
            (2, tags!("user" => "user3"), -16360.0),
            (1, tags!("user" => "user3"), 411.0),
        ];
        for (metric, want) in metrics.iter().zip(want.iter()) {
            if want.0 == 1 {
                assert_matches!(metric.value(), MetricValue::Sum(got) if *got == want.2);
            } else {
                assert_matches!(metric.value(), MetricValue::Gauge(got) if *got == want.2);
            }
        }
    }
}
