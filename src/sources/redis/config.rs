use std::collections::BTreeMap;

use event::{Metric, tags};

use super::Error;
use super::connection::Connection;

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let config: BTreeMap<String, String> = conn.execute(&["config", "get", "*"]).await?;

    let mut metrics = Vec::with_capacity(8);
    for (key, value) in config {
        match key.as_str() {
            "databases" => {
                let _databases = value.parse::<u64>().unwrap_or(0);
            }
            "io-threads" => {
                let io_threads = value.parse::<u64>().unwrap_or(0);

                metrics.push(Metric::gauge("redis_config_io_threads", "", io_threads))
            }
            "maxclients" => {
                let max_clients = value.parse::<u64>().unwrap_or(0);

                metrics.push(Metric::gauge("redis_config_max_clients", "", max_clients));
            }
            "maxmemory" => {
                let max_memory = value.parse::<u64>().unwrap_or(0);

                metrics.push(Metric::gauge("redis_config_max_memory", "", max_memory))
            }
            "client-output-buffer-limit" => {
                // client-output-buffer-limit "normal 0 0 0 slave 1610612736 1610612736 0 pubsub 33554432 8388608 60"
                let mut fields = value.split_ascii_whitespace();
                loop {
                    let Some(class) = fields.next() else {
                        break;
                    };

                    let Some(value) = fields.next() else {
                        break;
                    };
                    if let Ok(value) = value.parse::<f64>() {
                        metrics.push(Metric::gauge_with_tags(
                            "redis_config_client_output_buffer_limit_bytes",
                            "The configured buffer limits per class",
                            value,
                            tags!(
                                "class" => class,
                                "limit" => "hard"
                            ),
                        ));
                    }

                    let Some(value) = fields.next() else {
                        break;
                    };
                    if let Ok(value) = value.parse::<f64>() {
                        metrics.push(Metric::gauge_with_tags(
                            "redis_config_client_output_buffer_limit_bytes",
                            "The configured buffer limits per class",
                            value,
                            tags!(
                                "class" => class,
                                "limit" => "soft"
                            ),
                        ));
                    }

                    let Some(value) = fields.next() else {
                        break;
                    };
                    if let Ok(value) = value.parse::<f64>() {
                        metrics.push(Metric::gauge_with_tags(
                            "redis_config_client_output_buffer_limit_overcome_seconds",
                            "How long for buffer limits per class to be exceeded before replicas are dropped",
                            value,
                            tags!(
                                "class" => class,
                                "limit" => "soft"
                            )
                        ))
                    }
                }
            }
            _ => {}
        }
    }

    Ok(metrics)
}
