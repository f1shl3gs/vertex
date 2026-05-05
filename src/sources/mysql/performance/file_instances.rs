// `performance_schema.file_summary_by_instance`

use configurable::Configurable;
use event::{Metric, tags};
use serde::{Deserialize, Serialize};

use super::{Connection, Error};

fn default_filter() -> String {
    String::from("*")
}

fn default_remove_prefix() -> String {
    String::from("/var/lib/mysql/")
}

#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    /// RegEx file_name filter for performance_schema.file_summary_by_instance
    #[serde(default = "default_filter")]
    filter: String,

    /// Remove path prefix in performance_schema.file_summary_by_instance
    #[serde(default = "default_remove_prefix")]
    remove_prefix: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            filter: default_filter(),
            remove_prefix: default_remove_prefix(),
        }
    }
}

pub async fn collect(conn: &mut Connection, conf: &Config) -> Result<Vec<Metric>, Error> {
    let query = format!(
        "SELECT
	    FILE_NAME, EVENT_NAME,
	    COUNT_READ, COUNT_WRITE,
	    SUM_NUMBER_OF_BYTES_READ, SUM_NUMBER_OF_BYTES_WRITE
	  FROM performance_schema.file_summary_by_instance
	     where FILE_NAME REGEXP {}",
        conf.filter
    );

    let mut rows = conn.query(query).await?;

    let mut metrics = vec![];
    while let Some(mut row) = rows.next().await? {
        let filename = row.get_str();
        let event = row.get_str();
        let count_read = row.get_str().parse::<u64>()?;
        let count_write = row.get_str().parse::<u64>()?;
        let sum_bytes_read = row.get_str().parse::<u64>()?;
        let sum_bytes_written = row.get_str().parse::<u64>()?;

        let filename = filename
            .strip_prefix(&conf.remove_prefix)
            .unwrap_or(filename);

        metrics.extend([
            Metric::sum_with_tags(
                "mysql_perf_schema_file_instances_total",
                "The total number of file read/write operations.",
                count_read,
                tags!("file_name" => filename, "event" => event, "mode" => "read"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_file_instances_total",
                "The total number of file read/write operations.",
                count_write,
                tags!("file_name" => filename, "event" => event, "mode" => "write"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_file_instances_bytes",
                "The number of bytes processed by file read/write operations.",
                sum_bytes_read,
                tags!("file_name" => filename, "event" => event, "mode" => "read"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_file_instances_bytes",
                "The number of bytes processed by file read/write operations.",
                sum_bytes_written,
                tags!("file_name" => filename, "event" => event, "mode" => "write"),
            ),
        ]);
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::mysql::assert_contains;
    use crate::sources::mysql::connection::mock;

    #[tokio::test]
    async fn smoke() {
        let mut conn = mock(|_query| {
            (
                vec![
                    "FILE_NAME",
                    "EVENT_NAME",
                    "COUNT_READ",
                    "COUNT_WRITE",
                    "SUM_NUMBER_OF_BYTES_READ",
                    "SUM_NUMBER_OF_BYTES_WRITE",
                ],
                vec![
                    vec!["/var/lib/mysql/db1/file", "event1", "3", "4", "725", "128"],
                    vec![
                        "/var/lib/mysql/db2/file",
                        "event2",
                        "23",
                        "12",
                        "3123",
                        "967",
                    ],
                    vec!["db3/file", "event3", "45", "32", "1337", "326"],
                ],
            )
        })
        .await;

        let metrics = collect(&mut conn, &Config::default()).await.unwrap();
        assert_contains(
            &metrics,
            vec![],
            vec![
                (
                    tags!("file_name" => "db1/file", "event" => "event1", "mode" => "read"),
                    3.0,
                ),
                (
                    tags!("file_name" => "db1/file", "event" => "event1", "mode" => "write"),
                    4.0,
                ),
                (
                    tags!("file_name" => "db1/file", "event" => "event1", "mode" => "read"),
                    725.0,
                ),
                (
                    tags!("file_name" => "db1/file", "event" => "event1", "mode" => "write"),
                    128.0,
                ),
                (
                    tags!("file_name" => "db2/file", "event" => "event2", "mode" => "read"),
                    23.0,
                ),
                (
                    tags!("file_name" => "db2/file", "event" => "event2", "mode" => "write"),
                    12.0,
                ),
                (
                    tags!("file_name" => "db2/file", "event" => "event2", "mode" => "read"),
                    3123.0,
                ),
                (
                    tags!("file_name" => "db2/file", "event" => "event2", "mode" => "write"),
                    967.0,
                ),
                (
                    tags!("file_name" => "db3/file", "event" => "event3", "mode" => "read"),
                    45.0,
                ),
                (
                    tags!("file_name" => "db3/file", "event" => "event3", "mode" => "write"),
                    32.0,
                ),
                (
                    tags!("file_name" => "db3/file", "event" => "event3", "mode" => "read"),
                    1337.0,
                ),
                (
                    tags!("file_name" => "db3/file", "event" => "event3", "mode" => "write"),
                    326.0,
                ),
            ],
        );
    }
}
