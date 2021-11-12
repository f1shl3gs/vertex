use std::collections::BTreeMap;
use nom::InputIter;
use sqlx::{Column, MySqlPool, Row};
use event::{Metric, tags};
use crate::config::DataType::Metric;
use crate::Error;

#[derive(Debug, sqlx::FromRow)]
struct GlobalStatus {
    #[sqlx(rename = "Variable_name")]
    name: String,
    #[sqlx(rename = "Value")]
    value: String,
}

pub async fn query(pool: &MySqlPool) -> Result<Vec<Metric>, Error> {
    let stats = sqlx::query_as::<_, GlobalStatus>(r#"SHOW GLOBAL STATUS"#)
        .fetch_all(pool)
        .await
        .unwrap();

    let mut metrics = vec![];
    let mut text_items = BTreeMap::new();
    text_items.insert("wsrep_local_state_uuid".to_string(), "".to_string());
    text_items.insert("wsrep_cluster_state_uuid".to_string(), "".to_string());
    text_items.insert("wsrep_provider_version".to_string(), "".to_string());
    text_items.insert("wsrep_evs_repl_latency".to_string(), "".to_string());

    for stat in stats.iter() {
        let key = valid_name(&stat.name);
        let fv = match stat.value.parse::<f64>() {
            Ok(v) => v,
            _ => {
                if text_items.contains(&key) {
                    text_items.insert(key.clone(), stat.value.clone());
                }
                continue;
            }
        };

        match key.as_str() {
            "com" => metrics.push(Metric::sum_with_tags(
                "mysql_global_status_commands_total",
                "Total number of executed MySQL commands",
                fv,
                tags!(
                    "command" => name
                )
            )),
            "handler" => metrics.push(Metric::sum_with_tags(
                "mysql_global_status_handlers_total",
                "Total number of executed MySQL handlers",
                fv,
                tags!(
                    "handler" => name
                )
            )),
            "connection_errors" => metrics.push(Metric::sum_with_tags(
                "mysql_global_status_connection_errors_total",
                "Total number of MySQL connection errors",
                fv,
                tags!(
                    "error" => name
                )
            )),
            "innodb_buffer_pool_pages" => {
                match name {
                    "data" | "free" | "misc" | "old" => {
                        metrics.push(Metric::gauge_with_tags(
                            "mysql_global_status_buffer_pool_pages",
                            "Innodb buffer pool pages by state",
                            fv,
                            tags!(
                                "state" => name
                            )
                        ));
                    }
                    "dirty" => {
                        metrics.push(Metric::gauge(
                            "mysql_global_status_buffer_pool_dirty_pages",
                            "Innodb buffer pool dirty pages",
                            fv,
                        ));
                    }
                    "total" => continue,
                    _ => {
                        metrics.push(Metric::gauge_with_tags(
                            "mysql_global_status_buffer_pool_page_changes_total",
                            "Innodb buffer pool page state changes",
                            fv,
                            tags!(
                                "operation" => name
                            )
                        ));
                    }
                }
            }
            "innodb_rows" => metrics.push(Metric::sum_with_tags(
                "mysql_global_status_innodb_row_ops_total",
                "Total number of MySQL InnoDB row operations",
                fv,
                tags!(
                    "operation" => name
                )
            )),
            "performance_schema" => metrics.push(Metric::sum_with_tags(
                "mysql_global_status_performance_schema_lost_total",
                "Total number of MySQL instrumentations that could not be loaded or created due to memory constraints.",
                fv,
                tags!(
                    "instrumentation" => name
                )
            )),
            _ => {
                metrics.push(Metric::gauge(
                    "mysql_global_status_" + key,
                    "Generic metric from SHOW GLOBAL STATUS",
                    fv,
                ));
            }
        }
    }

    // mysql_galera_variables_info metric
    if text_items.get("wsrep_local_state_uuid").unwrap() != "" {
        metrics.push(Metric::gauge_with_tags(
            "mysql_galera_status_info",
            "PXC/Galera status information.",
            1,
            tags!(
                "wsrep_local_state_uuid" => text_items.get("wsrep_local_state_uuid").unwrap(),
                "wsrep_cluster_state_uuid" => text_items.get("wsrep_cluster_state_uuid").unwrap(),
                "wsrep_provider_version" => text_items.get("wsrep_provider_version").unwrap()
            )
        ));
    }

    // mysql_galera_evs_repl_latency
    if text_items.get("wsrep_evs_repl_latency").unwrap() != "" {
        let mut evs = [
            ("min_seconds", 0f64, 0usize, "PXC/Galera group communication latency. Min value."),
            ("avg_seconds", 0f64, 1usize, "PXC/Galera group communication latency. Avg value."),
            ("max_seconds", 0f64, 2usize, "PXC/Galera group communication latency. Max value."),
            ("stdev", 0f64, 3usize, "PXC/Galera group communication latency. Standard Deviation."),
            ("sample_size", 0f64, 4usize, "PXC/Galera group communication latency. Sample Size.")
        ];

        let mut parsing_success = true;
        let values = text_items.get("wsrep_evs_repl_latency")
            .unwrap()
            .split("/")
            .collect::<Vec<_>>();

        if evs.len() == values.len() {
            for (_, value, index, _) in evs.iter_mut() {
                match values[index].parse::<f64>() {
                    Ok(v) => *value = v,
                    Err(_) => parsing_success = false
                }
            }

            if parsing_success {
                for (name, value, _, desc) in evs {
                    metrics.push(Metric::gauge(
                        "mysql_galera_evs_repl_latency_" + name,
                        desc,
                        value
                    ));
                }
            }
        }
    }

    Ok(metrics)
}

fn is_global_status(name: &str) -> bool {
    const GLOBAL_STATUS_PREFIXES: [&str; 6] = [
        "com_",
        "handler_",
        "connection_errors_",
        "innodb_buffer_pool_pages_",
        "innodb_rows_",
        "performance_schema_"
    ];

    GLOBAL_STATUS_PREFIXES.contains(&name)
}

fn valid_name(s: &str) -> String {
    s.chars()
        .map(|x| {
            if x.is_alphanumeric() {
                x
            }

            '_'
        })
        .collect::<String>()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use sqlx::{Connection, ConnectOptions, MySql};
    use sqlx::mysql::{MySqlConnectOptions, MySqlPoolOptions, MySqlSslMode};
    use sqlx::pool::PoolOptions;
    use super::*;

    #[tokio::test]
    async fn test_query() {
        let options = MySqlConnectOptions::new()
            .host("127.0.0.1")
            .port(3306)
            .username("root")
            .password("password")
            .ssl_mode(MySqlSslMode::Disabled);
        let pool = MySqlPool::connect_with(options)
            .await
            .unwrap();

        let result = query(&pool).await;
    }

    #[tokio::test]
    async fn test_options_from_uri() {
        let uri = r#"mysql://root:password@127.0.0.1/?ssl=disabled"#;
        let mut options = uri.parse::<MySqlConnectOptions>().unwrap();
        // options = options.ssl_mode(MySqlSslMode::Disabled);

        println!("{:#?}", options);

        let pool = MySqlPool::connect_with(options).await.unwrap();
    }
}