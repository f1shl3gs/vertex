use configurable::Configurable;
use event::{Metric, tags};
use serde::{Deserialize, Serialize};

use super::connection::{Connection, Error};

fn default_database() -> String {
    String::from("heartbeat")
}

fn default_heartbeat() -> String {
    String::from("heartbeat")
}

#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    /// Database from where to collect heartbeat data
    #[serde(default = "default_database")]
    database: String,

    /// Table from where to collect heartbeat data
    #[serde(default = "default_heartbeat")]
    table: String,

    /// Use UTC for timestamps of the current server
    #[serde(default)]
    utc: bool,
}

pub async fn collect(conn: &mut Connection, config: &Config) -> Result<Vec<Metric>, Error> {
    let mut rows = conn
        .query(format!(
            "SELECT UNIX_TIMESTAMP(ts), UNIX_TIMESTAMP({}), server_id from `{}`.`{}`",
            if config.utc {
                "UTC_TIMESTAMP(6)"
            } else {
                "NOW(6)"
            },
            config.database,
            config.table
        ))
        .await?;

    let mut metrics = Vec::new();
    while let Some(mut row) = rows.next().await? {
        let ts = row.get_str().parse::<f64>()?;
        let now = row.get_str().parse::<f64>()?;
        let server_id = row.get_str().parse::<i32>()?;

        metrics.extend([
            Metric::gauge_with_tags(
                "mysql_heartbeat_now_timestamp_seconds",
                "Timestamp of the current server.",
                now,
                tags!("server_id" => server_id),
            ),
            Metric::gauge_with_tags(
                "mysql_heartbeat_stored_timestamp_seconds",
                "Timestamp stored in the heartbeat table.",
                ts,
                tags!("server_id" => server_id),
            ),
        ]);
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::mysql::connection::mock;

    #[tokio::test]
    async fn heartbeat() {
        for config in [
            Config {
                database: "heartbeat-test".to_string(),
                table: "heartbeat-test".to_string(),
                utc: false,
            },
            Config {
                database: "heartbeat-test".to_string(),
                table: "heartbeat-test".to_string(),
                utc: true,
            },
        ] {
            let mut conn = mock(|_query| {
                (
                    vec!["UNIX_TIMESTAMP(ts)", "UNIX_TIMESTAMP(NOW(6))", "server_id"],
                    vec![vec!["1487597613.001320", "1487598113.448042", "1"]],
                )
            })
            .await;

            let metrics = collect(&mut conn, &config).await.unwrap();

            assert_eq!(
                metrics,
                vec![
                    Metric::gauge_with_tags(
                        "mysql_heartbeat_now_timestamp_seconds",
                        "Timestamp of the current server.",
                        1487598113.448042,
                        tags!("server_id" => 1),
                    ),
                    Metric::gauge_with_tags(
                        "mysql_heartbeat_stored_timestamp_seconds",
                        "Timestamp stored in the heartbeat table.",
                        1_487_597_613.001_32,
                        tags!("server_id" => 1),
                    ),
                ]
            );
        }
    }
}
