use event::{Metric, tags};

use super::Error;
use super::connection::Connection;

// https://redis.io/commands/latency-latest
pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut metrics = vec![];
    let values: Vec<Vec<String>> = conn.execute(&["latency", "latest"]).await?;

    for parts in values {
        let event = parts[0].clone();
        let spike_last = parts[1].parse::<f64>()?;
        let spike_duration = parts[2].parse::<f64>()?;

        metrics.extend([
            Metric::gauge_with_tags(
                "redis_latency_spike_last",
                "When the latency spike last occurred",
                spike_last,
                tags!(
                    "event_name" => event.clone()
                ),
            ),
            Metric::gauge_with_tags(
                "redis_latency_spike_duration_seconds",
                "Length of the last latency spike in seconds",
                spike_duration / 1e3,
                tags!(
                    "event_name" => event
                ),
            ),
        ]);
    }

    Ok(metrics)
}
