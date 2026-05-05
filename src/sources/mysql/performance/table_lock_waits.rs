// `performance_schema.table_lock_waits_summary_by_table`

use super::{Connection, Error};
use event::{Metric, tags};

const TABLE_LOCK_WAITS_QUERY: &str = "SELECT
  OBJECT_SCHEMA,
  OBJECT_NAME,
  COUNT_READ_NORMAL,
  COUNT_READ_WITH_SHARED_LOCKS,
  COUNT_READ_HIGH_PRIORITY,
  COUNT_READ_NO_INSERT,
  COUNT_READ_EXTERNAL,
  COUNT_WRITE_ALLOW_WRITE,
  COUNT_WRITE_CONCURRENT_INSERT,
  COUNT_WRITE_LOW_PRIORITY,
  COUNT_WRITE_NORMAL,
  COUNT_WRITE_EXTERNAL,
  SUM_TIMER_READ_NORMAL,
  SUM_TIMER_READ_WITH_SHARED_LOCKS,
  SUM_TIMER_READ_HIGH_PRIORITY,
  SUM_TIMER_READ_NO_INSERT,
  SUM_TIMER_READ_EXTERNAL,
  SUM_TIMER_WRITE_ALLOW_WRITE,
  SUM_TIMER_WRITE_CONCURRENT_INSERT,
  SUM_TIMER_WRITE_LOW_PRIORITY,
  SUM_TIMER_WRITE_NORMAL,
  SUM_TIMER_WRITE_EXTERNAL
FROM performance_schema.table_lock_waits_summary_by_table
WHERE OBJECT_SCHEMA NOT IN ('mysql', 'performance_schema', 'information_schema')";

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut rows = conn.query(TABLE_LOCK_WAITS_QUERY).await?;

    let mut metrics = vec![];
    while let Some(mut row) = rows.next().await? {
        let schema = row.get_str();
        let name = row.get_str();

        let count_read_normal = row.get_str().parse::<u64>()?;
        let count_read_with_shared_locks = row.get_str().parse::<u64>()?;
        let count_read_high_priority = row.get_str().parse::<u64>()?;
        let count_read_no_insert = row.get_str().parse::<u64>()?;
        let count_read_external = row.get_str().parse::<u64>()?;
        let count_write_allow_write = row.get_str().parse::<u64>()?;
        let count_write_concurrent_insert = row.get_str().parse::<u64>()?;
        let count_write_low_priority = row.get_str().parse::<u64>()?;
        let count_write_normal = row.get_str().parse::<u64>()?;
        let count_write_external = row.get_str().parse::<u64>()?;
        let time_read_normal = row.get_str().parse::<u64>()?;
        let time_read_with_shared_locks = row.get_str().parse::<u64>()?;
        let time_read_high_priority = row.get_str().parse::<u64>()?;
        let time_read_no_insert = row.get_str().parse::<u64>()?;
        let time_read_external = row.get_str().parse::<u64>()?;
        let time_write_allow_write = row.get_str().parse::<u64>()?;
        let time_write_concurrent_insert = row.get_str().parse::<u64>()?;
        let time_write_low_priority = row.get_str().parse::<u64>()?;
        let time_write_normal = row.get_str().parse::<u64>()?;
        let time_write_external = row.get_str().parse::<u64>()?;

        metrics.extend([
            Metric::sum_with_tags(
                "mysql_perf_schema_sql_lock_waits_total",
                "The total number of SQL lock wait events for each table and operation.",
                count_read_normal,
                tags!("schema" => schema, "name" => name, "operation" => "read_normal"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_sql_lock_waits_total",
                "The total number of SQL lock wait events for each table and operation.",
                count_read_with_shared_locks,
                tags!("schema" => schema, "name" => name, "operation" => "read_with_shared_locks"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_sql_lock_waits_total",
                "The total number of SQL lock wait events for each table and operation.",
                count_read_high_priority,
                tags!("schema" => schema, "name" => name, "operation" => "read_high_priority"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_sql_lock_waits_total",
                "The total number of SQL lock wait events for each table and operation.",
                count_read_no_insert,
                tags!("schema" => schema, "name" => name, "operation" => "read_no_insert"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_sql_lock_waits_total",
                "The total number of SQL lock wait events for each table and operation.",
                count_write_normal,
                tags!("schema" => schema, "name" => name, "operation" => "write_normal"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_sql_lock_waits_total",
                "The total number of SQL lock wait events for each table and operation.",
                count_write_allow_write,
                tags!("schema" => schema, "name" => name, "operation" => "write_allow_write"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_sql_lock_waits_total",
                "The total number of SQL lock wait events for each table and operation.",
                count_write_concurrent_insert,
                tags!("schema" => schema, "name" => name, "operation" => "write_concurrent_insert"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_sql_lock_waits_total",
                "The total number of SQL lock wait events for each table and operation.",
                count_write_low_priority,
                tags!("schema" => schema, "name" => name, "operation" => "write_low_priority"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_external_lock_waits_total",
                "The total number of external lock wait events for each table and operation.",
                count_read_external,
                tags!("schema" => schema, "name" => name, "operation" => "read"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_external_lock_waits_total",
                "The total number of external lock wait events for each table and operation.",
                count_write_external,
                tags!("schema" => schema, "name" => name, "operation" => "write"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_sql_lock_waits_seconds_total",
                "The total time of SQL lock wait events for each table and operation.",
                time_read_normal as f64 / 1e12,
                tags!("schema" => schema, "name" => name, "operation" => "read_normal"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_sql_lock_waits_seconds_total",
                "The total time of SQL lock wait events for each table and operation.",
                time_read_with_shared_locks as f64 / 1e12,
                tags!("schema" => schema, "name" => name, "operation" => "read_with_shared_locks"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_sql_lock_waits_seconds_total",
                "The total time of SQL lock wait events for each table and operation.",
                time_read_high_priority as f64 / 1e12,
                tags!("schema" => schema, "name" => name, "operation" => "read_high_priority"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_sql_lock_waits_seconds_total",
                "The total time of SQL lock wait events for each table and operation.",
                time_read_no_insert as f64 / 1e12,
                tags!("schema" => schema, "name" => name, "operation" => "read_no_insert"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_sql_lock_waits_seconds_total",
                "The total time of SQL lock wait events for each table and operation.",
                time_write_normal as f64 / 1e12,
                tags!("schema" => schema, "name" => name, "operation" => "write_normal"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_sql_lock_waits_seconds_total",
                "The total time of SQL lock wait events for each table and operation.",
                time_write_allow_write as f64 / 1e12,
                tags!("schema" => schema, "name" => name, "operation" => "write_allow_write"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_sql_lock_waits_seconds_total",
                "The total time of SQL lock wait events for each table and operation.",
                time_write_concurrent_insert as f64 / 1e12,
                tags!("schema" => schema, "name" => name, "operation" => "write_concurrent_insert"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_sql_lock_waits_seconds_total",
                "The total time of SQL lock wait events for each table and operation.",
                time_write_low_priority as f64 / 1e12,
                tags!("schema" => schema, "name" => name, "operation" => "write_low_priority"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_external_lock_waits_total",
                "The total time of external lock wait events for each table and operation.",
                time_read_external as f64 / 1e12,
                tags!("schema" => schema, "name" => name, "operation" => "read"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_external_lock_waits_total",
                "The total time of external lock wait events for each table and operation.",
                time_write_external as f64 / 1e12,
                tags!("schema" => schema, "name" => name, "operation" => "write"),
            ),
        ]);
    }

    Ok(metrics)
}
