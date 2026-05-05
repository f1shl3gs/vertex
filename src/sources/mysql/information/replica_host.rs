// `information_schema.replica_host_status`

use event::{Metric, tags};

use super::{Connection, Error};

const REPLICA_HOST_QUERY: &str = "SELECT SERVER_ID
		   , if(SESSION_ID='MASTER_SESSION_ID','writer','reader') AS ROLE
		   , CPU
		   , MASTER_SLAVE_LATENCY_IN_MICROSECONDS
		   , REPLICA_LAG_IN_MILLISECONDS
		   , LOG_STREAM_SPEED_IN_KiB_PER_SECOND
		   , CURRENT_REPLAY_LATENCY_IN_MICROSECONDS
		FROM information_schema.replica_host_status";

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut rows = conn.query(REPLICA_HOST_QUERY).await?;

    let mut metrics = Vec::new();
    while let Some(mut row) = rows.next().await? {
        let server_id = row.get_str();
        let role = row.get_str();
        let cpu = row.get_str().parse::<f64>()?;
        let replica_latency = row.get_str().parse::<f64>()?;
        let replica_lag = row.get_str().parse::<f64>()?;
        let log_stream_speed = row.get_str().parse::<f64>()?;
        let replay_latency = row.get_str().parse::<f64>()?;

        metrics.extend([
            Metric::gauge_with_tags(
                "mysql_info_schema_replica_host_cpu_percent",
                "The CPU usage as a percentage.",
                cpu,
                tags!("server_id" => server_id, "role" => role),
            ),
            Metric::gauge_with_tags(
                "mysql_info_schema_replica_host_replica_latency_seconds",
                "The source-replica latency in seconds.",
                replica_latency * 0.000001,
                tags!("server_id" => server_id, "role" => role),
            ),
            Metric::gauge_with_tags(
                "mysql_info_schema_replica_host_lag_seconds",
                "The replica lag in seconds.",
                replica_lag * 0.001,
                tags!("server_id" => server_id, "role" => role),
            ),
            Metric::gauge_with_tags(
                "mysql_info_schema_replica_host_log_stream_speed",
                "The log stream speed in kilobytes per second.",
                log_stream_speed,
                tags!("server_id" => server_id, "role" => role),
            ),
            Metric::gauge_with_tags(
                "mysql_info_schema_replica_host_replay_latency_seconds",
                "The current replay latency in seconds.",
                replay_latency * 0.000001,
                tags!("server_id" => server_id, "role" => role),
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
                    "SERVER_ID",
                    "ROLE",
                    "CPU",
                    "MASTER_SLAVE_LATENCY_IN_MICROSECONDS",
                    "REPLICA_LAG_IN_MILLISECONDS",
                    "LOG_STREAM_SPEED_IN_KiB_PER_SECOND",
                    "CURRENT_REPLAY_LATENCY_IN_MICROSECONDS",
                ],
                vec![
                    vec![
                        "dbtools-cluster-us-west-2c",
                        "reader",
                        "1.2531328201293945",
                        "250000",
                        "20.069000244140625",
                        "2.0368164549078225",
                        "500000",
                    ],
                    vec![
                        "dbtools-cluster-writer",
                        "writer",
                        "1.9607843160629272",
                        "250000",
                        "0",
                        "2.0368164549078225",
                        "0",
                    ],
                ],
            )
        })
        .await;

        let metrics = collect(&mut conn).await.unwrap();
        assert_contains(
            &metrics,
            vec![
                (
                    tags!("server_id" => "dbtools-cluster-us-west-2c", "role" => "reader"),
                    1.2531328201293945,
                ),
                (
                    tags!("server_id" => "dbtools-cluster-us-west-2c", "role" => "reader"),
                    0.25,
                ),
                (
                    tags!("server_id" => "dbtools-cluster-us-west-2c", "role" => "reader"),
                    0.020069000244140625,
                ),
                (
                    tags!("server_id" => "dbtools-cluster-us-west-2c", "role" => "reader"),
                    2.0368164549078225,
                ),
                (
                    tags!("server_id" => "dbtools-cluster-us-west-2c", "role" => "reader"),
                    0.5,
                ),
                (
                    tags!("server_id" => "dbtools-cluster-writer", "role" => "writer"),
                    1.9607843160629272,
                ),
                (
                    tags!("server_id" => "dbtools-cluster-writer", "role" => "writer"),
                    0.25,
                ),
                (
                    tags!("server_id" => "dbtools-cluster-writer", "role" => "writer"),
                    0.0,
                ),
                (
                    tags!("server_id" => "dbtools-cluster-writer", "role" => "writer"),
                    2.0368164549078225,
                ),
                (
                    tags!("server_id" => "dbtools-cluster-writer", "role" => "writer"),
                    0.0,
                ),
            ],
            vec![],
        )
    }
}
