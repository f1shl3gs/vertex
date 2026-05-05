// `performance_schema.memory_summary_global_by_event_name`

use configurable::Configurable;
use event::{Metric, tags};
use serde::{Deserialize, Serialize};

use super::{Connection, Error};

const MEMORY_EVENTS_QUERY: &str = "SELECT
  EVENT_NAME, SUM_NUMBER_OF_BYTES_ALLOC, SUM_NUMBER_OF_BYTES_FREE,
  CURRENT_NUMBER_OF_BYTES_USED
FROM performance_schema.memory_summary_global_by_event_name
  where COUNT_ALLOC > 0";

fn default_remove_prefix() -> String {
    String::from("memory/")
}

#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    /// Remove instrument prefix in performance_schema.memory_summary_global_by_event_name
    #[serde(default = "default_remove_prefix")]
    remove_prefix: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            remove_prefix: default_remove_prefix(),
        }
    }
}

pub async fn collect(conn: &mut Connection, conf: &Config) -> Result<Vec<Metric>, Error> {
    let mut rows = conn.query(MEMORY_EVENTS_QUERY).await?;

    let mut metrics = vec![];
    while let Some(mut row) = rows.next().await? {
        let event = row.get_str();
        let bytes_alloc = row.get_str().parse::<u64>()?;
        let bytes_free = row.get_str().parse::<u64>()?;
        let current_bytes = row.get_str().parse::<i64>()?;

        let event = event
            .strip_prefix(conf.remove_prefix.as_str())
            .unwrap_or(event);
        metrics.extend([
            Metric::sum_with_tags(
                "mysql_perf_schema_memory_events_alloc_bytes_total",
                "The total number of bytes allocated by events.",
                bytes_alloc,
                tags!("event" => event),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_memory_events_free_bytes_total",
                "The total number of bytes freed by events.",
                bytes_free,
                tags!("event" => event),
            ),
            Metric::gauge_with_tags(
                "mysql_perf_schema_memory_events_used_bytes",
                "The number of bytes currently allocated by events.",
                current_bytes,
                tags!("event" => event),
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
        let mut conn = mock(|_| {
            (
                vec![
                    "EVENT_NAME",
                    "SUM_NUMBER_OF_BYTES_ALLOC",
                    "SUM_NUMBER_OF_BYTES_FREE",
                    "CURRENT_NUMBER_OF_BYTES_USED",
                ],
                vec![
                    vec!["memory/innodb/event1", "1001", "500", "501"],
                    vec!["memory/performance_schema/event1", "6000", "7", "-83904"],
                    vec!["memory/innodb/event2", "2002", "1000", "1002"],
                    vec!["memory/sql/event1", "30", "4", "26"],
                ],
            )
        })
        .await;

        let metrics = collect(&mut conn, &Config::default()).await.unwrap();
        assert_contains(
            &metrics,
            vec![
                (tags!("event" => "innodb/event1"), 501.0),
                (tags!("event" => "performance_schema/event1"), -83904.0),
                (tags!("event" => "innodb/event2"), 1002.0),
                (tags!("event" => "sql/event1"), 26.0),
            ],
            vec![
                (tags!("event" => "innodb/event1"), 1001.0),
                (tags!("event" => "innodb/event1"), 500.0),
                (tags!("event" => "performance_schema/event1"), 6000.0),
                (tags!("event" => "performance_schema/event1"), 7.0),
                (tags!("event" => "innodb/event2"), 2002.0),
                (tags!("event" => "innodb/event2"), 1000.0),
                (tags!("event" => "sql/event1"), 30.0),
                (tags!("event" => "sql/event1"), 4.0),
            ],
        );
    }
}
