// `performance_schema.table_io_waits_summary_by_index_usage`

use event::{Metric, tags};

use super::{Connection, Error};

const INDEX_IO_WAITS_QUERY: &str =
    "SELECT OBJECT_SCHEMA, OBJECT_NAME, ifnull(INDEX_NAME, 'NONE') as INDEX_NAME,
  COUNT_FETCH, COUNT_INSERT, COUNT_UPDATE, COUNT_DELETE,
  SUM_TIMER_FETCH, SUM_TIMER_INSERT, SUM_TIMER_UPDATE, SUM_TIMER_DELETE
FROM performance_schema.table_io_waits_summary_by_index_usage
  WHERE OBJECT_SCHEMA NOT IN ('mysql', 'performance_schema')";

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut rows = conn.query(INDEX_IO_WAITS_QUERY).await?;

    let mut metrics = vec![];
    while let Some(mut row) = rows.next().await? {
        let schema = row.get_str();
        let name = row.get_str();
        let index = row.get_str();

        let count_fetch = row.get_str().parse::<u64>()?;
        let count_insert = row.get_str().parse::<u64>()?;
        let count_update = row.get_str().parse::<u64>()?;
        let count_delete = row.get_str().parse::<u64>()?;
        let time_fetch = row.get_str().parse::<u64>()?;
        let time_insert = row.get_str().parse::<u64>()?;
        let time_update = row.get_str().parse::<u64>()?;
        let time_delete = row.get_str().parse::<u64>()?;

        metrics.reserve(8);

        // We only include the insert column when index is NONE
        if index == "NONE" {
            metrics.extend([
                Metric::sum_with_tags(
                    "mysql_perf_schema_index_io_waits_total",
                    "The total number of index I/O wait events for each index and operation.",
                    count_insert,
                    tags!("schema" => schema, "name" => name, "index" => index, "operation" => "insert"),
                ),
                Metric::sum_with_tags(
                    "mysql_perf_schema_index_io_waits_seconds_total",
                    "The total time of index I/O wait events for each index and operation.",
                    time_insert as f64 / 1e12,
                    tags!("schema" => schema, "name" => name, "index" => index, "operation" => "insert"),
                )
            ]);
        }

        metrics.extend([
            Metric::sum_with_tags(
                "mysql_perf_schema_index_io_waits_total",
                "The total number of index I/O wait events for each index and operation.",
                count_fetch,
                tags!("schema" => schema, "name" => name, "index" => index, "operation" => "fetch"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_index_io_waits_total",
                "The total number of index I/O wait events for each index and operation.",
                count_update,
                tags!("schema" => schema, "name" => name, "index" => index, "operation" => "update"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_index_io_waits_total",
                "The total number of index I/O wait events for each index and operation.",
                count_delete,
                tags!("schema" => schema, "name" => name, "index" => index, "operation" => "delete"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_index_io_waits_seconds_total",
                "The total time of index I/O wait events for each index and operation.",
                time_fetch as f64 / 1e12,
                tags!("schema" => schema, "name" => name, "index" => index, "operation" => "fetch"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_index_io_waits_seconds_total",
                "The total time of index I/O wait events for each index and operation.",
                time_update as f64 / 1e12,
                tags!("schema" => schema, "name" => name, "index" => index, "operation" => "update"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_index_io_waits_seconds_total",
                "The total time of index I/O wait events for each index and operation.",
                time_delete as f64 / 1e12,
                tags!("schema" => schema, "name" => name, "index" => index, "operation" => "delete"),
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

    #[tokio::test]
    async fn smoke() {
        let mut conn = mock(|_query| {
            (
                vec![
                    "OBJECT_SCHEMA",
                    "OBJECT_NAME",
                    "INDEX_NAME",
                    "COUNT_FETCH",
                    "COUNT_INSERT",
                    "COUNT_UPDATE",
                    "COUNT_DELETE",
                    "SUM_TIMER_FETCH",
                    "SUM_TIMER_INSERT",
                    "SUM_TIMER_UPDATE",
                    "SUM_TIMER_DELETE",
                ],
                vec![
                    vec![
                        "database",
                        "table",
                        "index",
                        "10",
                        "11",
                        "12",
                        "13",
                        "14000000000000",
                        "15000000000000",
                        "16000000000000",
                        "17000000000000",
                    ],
                    vec![
                        "database",
                        "table",
                        "NONE",
                        "20",
                        "21",
                        "22",
                        "23",
                        "24000000000000",
                        "25000000000000",
                        "26000000000000",
                        "27000000000000",
                    ],
                ],
            )
        })
        .await;

        let metrics = collect(&mut conn).await.unwrap();
        assert_contains(
            &metrics,
            vec![],
            vec![
                (
                    tags!("schema" => "database", "name" => "table", "index" => "index", "operation" => "fetch"),
                    10.0,
                ),
                (
                    tags!("schema" => "database", "name" => "table", "index" => "index", "operation" => "update"),
                    12.0,
                ),
                (
                    tags!("schema" => "database", "name" => "table", "index" => "index", "operation" => "delete"),
                    13.0,
                ),
                (
                    tags!("schema" => "database", "name" => "table", "index" => "index", "operation" => "fetch"),
                    14.0,
                ),
                (
                    tags!("schema" => "database", "name" => "table", "index" => "index", "operation" => "update"),
                    16.0,
                ),
                (
                    tags!("schema" => "database", "name" => "table", "index" => "index", "operation" => "delete"),
                    17.0,
                ),
                (
                    tags!("schema" => "database", "name" => "table", "index" => "NONE", "operation" => "fetch"),
                    20.0,
                ),
                (
                    tags!("schema" => "database", "name" => "table", "index" => "NONE", "operation" => "insert"),
                    21.0,
                ),
                (
                    tags!("schema" => "database", "name" => "table", "index" => "NONE", "operation" => "update"),
                    22.0,
                ),
                (
                    tags!("schema" => "database", "name" => "table", "index" => "NONE", "operation" => "delete"),
                    23.0,
                ),
                (
                    tags!("schema" => "database", "name" => "table", "index" => "NONE", "operation" => "fetch"),
                    24.0,
                ),
                (
                    tags!("schema" => "database", "name" => "table", "index" => "NONE", "operation" => "insert"),
                    25.0,
                ),
                (
                    tags!("schema" => "database", "name" => "table", "index" => "NONE", "operation" => "update"),
                    26.0,
                ),
                (
                    tags!("schema" => "database", "name" => "table", "index" => "NONE", "operation" => "delete"),
                    27.0,
                ),
            ],
        )
    }
}
