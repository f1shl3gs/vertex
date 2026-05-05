// performance_schema.table_io_waits_summary_by_table

use event::{Metric, tags};

use super::{Connection, Error};

const TABLE_IO_WAITS_QUERY: &str = "SELECT
  OBJECT_SCHEMA, OBJECT_NAME,
  COUNT_FETCH, COUNT_INSERT, COUNT_UPDATE, COUNT_DELETE,
  SUM_TIMER_FETCH, SUM_TIMER_INSERT, SUM_TIMER_UPDATE, SUM_TIMER_DELETE
FROM performance_schema.table_io_waits_summary_by_table
  WHERE OBJECT_SCHEMA NOT IN ('mysql', 'performance_schema')";

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut rows = conn.query(TABLE_IO_WAITS_QUERY).await?;

    let mut metrics = vec![];
    while let Some(mut row) = rows.next().await? {
        let schema = row.get_str();
        let name = row.get_str();

        let count_fetch = row.get_str().parse::<u64>()?;
        let count_insert = row.get_str().parse::<u64>()?;
        let count_update = row.get_str().parse::<u64>()?;
        let count_delete = row.get_str().parse::<u64>()?;

        let time_fetch = row.get_str().parse::<f64>()?;
        let time_insert = row.get_str().parse::<f64>()?;
        let time_update = row.get_str().parse::<f64>()?;
        let time_delete = row.get_str().parse::<f64>()?;

        for (operation, count, time) in [
            ("fetch", count_fetch, time_fetch),
            ("insert", count_insert, time_insert),
            ("update", count_update, time_update),
            ("delete", count_delete, time_delete),
        ] {
            metrics.extend([
                Metric::sum_with_tags(
                    "mysql_perf_schema_table_io_waits_total",
                    "The total number of table I/O wait events for each table and operation.",
                    count,
                    tags!("schema" => schema, "name" => name, "operation" => operation),
                ),
                Metric::sum_with_tags(
                    "mysql_perf_schema_table_io_waits_seconds_total",
                    "The total time of table I/O wait events for each table and operation.",
                    time / 1e12,
                    tags!("schema" => schema, "name" => name, "operation" => operation),
                ),
            ]);
        }
    }

    Ok(metrics)
}
