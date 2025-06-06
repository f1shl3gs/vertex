use event::Metric;

use super::Error;
use super::connection::{Connection, Error as ConnectionError};

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut metrics = vec![];

    match conn.execute::<i64>(&["slowlog", "len"]).await {
        Ok(length) => {
            metrics.push(Metric::gauge(
                "redis_slowlog_length",
                "Total slowlog",
                length,
            ));
        }
        Err(err) => match err {
            // sentinel does not support this failed
            ConnectionError::UnknownCommand(_) => return Ok(vec![]),
            err => {
                warn!(message = "slowlog length query failed", ?err)
            }
        },
    }

    let values: Vec<i64> = conn.execute(&["slowlog", "get", "1"]).await?;

    let mut last_id: i64 = 0;
    let mut last_slow_execution_second: f64 = 0.0;
    if !values.is_empty() {
        last_id = values[0];
        if values.len() > 2 {
            last_slow_execution_second = values[2] as f64 / 1e6
        }
    }

    metrics.extend([
        Metric::gauge(
            "redis_slowlog_last_id",
            "Last id of slowlog",
            last_id as f64,
        ),
        Metric::gauge(
            "redis_last_slow_execution_duration_seconds",
            "The amount of time needed for last slow execution, in seconds",
            last_slow_execution_second,
        ),
    ]);

    Ok(metrics)
}
