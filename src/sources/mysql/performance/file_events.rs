// `performance_schema.file_summary_by_event_name`

use super::{Connection, Error};
use event::{Metric, tags};

const FILE_EVENTS_QUERY: &str = "SELECT
  EVENT_NAME,
  COUNT_READ, SUM_TIMER_READ, SUM_NUMBER_OF_BYTES_READ,
  COUNT_WRITE, SUM_TIMER_WRITE, SUM_NUMBER_OF_BYTES_WRITE,
  COUNT_MISC, SUM_TIMER_MISC
FROM performance_schema.file_summary_by_event_name";

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut rows = conn.query(FILE_EVENTS_QUERY).await?;

    let mut metrics = vec![];
    while let Some(mut row) = rows.next().await? {
        let event = row.get_str();
        let count_read = row.get_str().parse::<u64>()?;
        let time_read = row.get_str().parse::<u64>()?;
        let bytes_read = row.get_str().parse::<u64>()?;
        let count_write = row.get_str().parse::<u64>()?;
        let time_write = row.get_str().parse::<u64>()?;
        let bytes_write = row.get_str().parse::<u64>()?;
        let count_misc = row.get_str().parse::<u64>()?;
        let time_misc = row.get_str().parse::<u64>()?;

        metrics.extend([
            Metric::sum_with_tags(
                "mysql_perf_schema_file_events_total",
                "The total file events by event name/mode.",
                count_read,
                tags!("event_name" => event, "mode" => "read"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_file_events_seconds_total",
                "The total seconds of file events by event name/mode.",
                time_read as f64 / 1e12,
                tags!("event_name" => event, "mode" => "read"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_file_events_bytes_total",
                "The total bytes of file events by event name/mode.",
                bytes_read,
                tags!("event_name" => event, "mode" => "read"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_file_events_total",
                "The total file events by event name/mode.",
                count_write,
                tags!("event_name" => event, "mode" => "write"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_file_events_seconds_total",
                "The total seconds of file events by event name/mode.",
                time_write as f64 / 1e12,
                tags!("event_name" => event, "mode" => "write"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_file_events_bytes_total",
                "The total bytes of file events by event name/mode.",
                bytes_write,
                tags!("event_name" => event, "mode" => "write"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_file_events_total",
                "The total file events by event name/mode.",
                count_misc,
                tags!("event_name" => event, "mode" => "misc"),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_file_events_seconds_total",
                "The total seconds of file events by event name/mode.",
                time_misc as f64 / 1e12,
                tags!("event_name" => event, "mode" => "misc"),
            ),
        ]);
    }

    Ok(metrics)
}
