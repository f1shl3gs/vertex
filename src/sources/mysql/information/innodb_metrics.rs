// information_schema.innodb_metrics

use super::{Connection, Error};
use event::{Metric, tags};

const INNODB_METRICS_ENABLED_COLUMN_QUERY: &str = "SELECT
  column_name
FROM information_schema.columns
WHERE table_schema = 'information_schema'
  AND table_name = 'INNODB_METRICS'
  AND column_name IN ('status', 'enabled')
LIMIT 1";

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut rows = conn.query(INNODB_METRICS_ENABLED_COLUMN_QUERY).await?;
    let Some(mut row) = rows.next().await? else {
        while rows.next().await?.is_some() {}
        return Ok(vec![]);
    };

    let query = match row.get_str() {
        "STATUS" => {
            "SELECT name, subsystem, type, comment, count FROM information_schema.innodb_metrics WHERE status = enabled"
        }
        "ENABLED" => {
            "SELECT name, subsystem, type, comment, count FROM information_schema.innodb_metrics WHERE enabled = 1"
        }
        _ => {
            debug!(message = "couldn't find column STATUS or ENABLED in innodb_metrics table");

            while rows.next().await?.is_some() {}

            return Ok(vec![]);
        }
    };
    while rows.next().await?.is_some() {}

    let mut rows = conn.query(query).await?;

    let mut metrics = Vec::new();
    while let Some(mut row) = rows.next().await? {
        let name = row.get_str();
        let subsystem = row.get_str();
        let metric_type = row.get_str();
        let comment = row.get_str();
        let value = row.get_str().parse::<f64>()?;

        if subsystem == "buffer_page_io" {
            if let Some(stripped) = name.strip_prefix("buffer_page_read_") {
                metrics.push(Metric::sum_with_tags(
                    "mysql_info_schema_innodb_metrics_buffer_page_read_total",
                    "Total number of buffer pages read total.",
                    value,
                    tags!("type" => stripped),
                ))
            } else if let Some(stripped) = name.strip_prefix("buffer_page_written_") {
                metrics.push(Metric::sum_with_tags(
                    "mysql_info_schema_innodb_metrics_buffer_page_written_total",
                    "Total number of buffer pages written total.",
                    value,
                    tags!("type" => stripped),
                ))
            } else {
                debug!(
                    message = "innodb_metrics subsystem buffer_page_io returned an invalid name",
                    name,
                );
            }

            continue;
        }

        if subsystem == "buffer"
            && let Some(stripped) = name.strip_prefix("buffer_pool_pages_")
        {
            match stripped {
                // ignore total, it is an aggregation of the rest
                "total" => continue,
                "dirty" => {
                    // dirty pages are a separate metric, not in the total
                    metrics.push(Metric::gauge(
                        "mysql_info_schema_innodb_metrics_buffer_pool_dirty_pages",
                        "Total number of dirty pages in the buffer pool.",
                        value,
                    ));
                }
                _ => metrics.push(Metric::gauge_with_tags(
                    "mysql_info_schema_innodb_metrics_buffer_pool_pages",
                    "Total number of buffer pool pages by state.",
                    value,
                    tags!("state" => stripped),
                )),
            }

            continue;
        }

        // MySQL returns counters named two different ways. "counter" and "status_counter"
        // value >= 0 is necessary due to upstream bugs: http://bugs.mysql.com/bug.php?id=75966
        if value >= 0.0 && (metric_type == "counter" || metric_type == "status_counter") {
            metrics.push(Metric::sum(
                format!("mysql_info_schema_innodb_metrics_{}_total", name),
                comment.to_string(),
                value,
            ))
        } else {
            metrics.push(Metric::gauge(
                format!("mysql_info_schema_innodb_metrics_{}", name),
                comment.to_string(),
                value,
            ))
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
    async fn innodb_metrics() {
        let mut conn = mock(|query| {
            if query == INNODB_METRICS_ENABLED_COLUMN_QUERY {
                (vec!["COLUMN_NAME"], vec![vec!["STATUS"]])
            } else {
                (
                    vec!["name", "subsystem", "type", "comment", "count"],
                    vec![
                        vec![
                            "lock_timeouts",
                            "lock",
                            "counter",
                            "Number of lock timeouts",
                            "0",
                        ],
                        vec![
                            "buffer_pool_reads",
                            "buffer",
                            "status_counter",
                            "Number of reads directly from disk (innodb_buffer_pool_reads)",
                            "1",
                        ],
                        vec![
                            "buffer_pool_size",
                            "server",
                            "value",
                            "Server buffer pool size (all buffer pools) in bytes",
                            "2",
                        ],
                        vec![
                            "buffer_page_read_system_page",
                            "buffer_page_io",
                            "counter",
                            "Number of System Pages read",
                            "3",
                        ],
                        vec![
                            "buffer_page_written_undo_log",
                            "buffer_page_io",
                            "counter",
                            "Number of Undo Log Pages written",
                            "4",
                        ],
                        vec![
                            "buffer_pool_pages_dirty",
                            "buffer",
                            "gauge",
                            "Number of dirt buffer pool pages",
                            "5",
                        ],
                        vec![
                            "buffer_pool_pages_data",
                            "buffer",
                            "gauge",
                            "Number of data buffer pool pages",
                            "6",
                        ],
                        vec![
                            "buffer_pool_pages_total",
                            "buffer",
                            "gauge",
                            "Number of total buffer pool pages",
                            "7",
                        ],
                        vec![
                            "NOPE",
                            "buffer_page_io",
                            "counter",
                            "An invalid buffer_page_io metric",
                            "999",
                        ],
                    ],
                )
            }
        })
        .await;

        let metrics = collect(&mut conn).await.unwrap();
        assert_contains(
            &metrics,
            vec![
                (tags!(), 2.0),
                (tags!(), 5.0),
                (tags!("state" => "data"), 6.0),
            ],
            vec![
                (tags!(), 0.0),
                (tags!(), 1.0),
                (tags!("type" => "system_page"), 3.0),
                (tags!("type" => "undo_log"), 4.0),
            ],
        );
    }
}
