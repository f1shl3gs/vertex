// `performance_schema.events_waits_summary_global_by_event_name`

use event::{Metric, tags};

use super::{Connection, Error};

const WAITS_QUERY: &str = "	SELECT EVENT_NAME, COUNT_STAR, SUM_TIMER_WAIT FROM performance_schema.events_waits_summary_global_by_event_name";

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut rows = conn.query(WAITS_QUERY).await?;

    let mut metrics = vec![];
    while let Some(mut row) = rows.next().await? {
        let event = row.get_str();
        let count = row.get_str().parse::<u64>()?;
        let time = row.get_str().parse::<f64>()?;

        metrics.extend([
            Metric::sum_with_tags(
                "mysql_perf_schema_events_waits_total",
                "The total events waits by event name.",
                count,
                tags!("event_name" => event),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_events_waits_seconds_total",
                "The total seconds of events waits by event name.",
                time / 1e12,
                tags!("event_name" => event),
            ),
        ]);
    }

    Ok(metrics)
}
