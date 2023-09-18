use std::borrow::Cow;
use std::collections::BTreeMap;

use crate::sources::mysqld::MysqlError;
use event::{tags, Metric};
use sqlx::MySqlPool;

use super::valid_name;

const GLOBAL_STATUS_QUERY: &str = r#"SHOW GLOBAL STATUS"#;

#[derive(Debug, sqlx::FromRow)]
struct GlobalStatus {
    #[sqlx(rename = "Variable_name")]
    name: String,
    #[sqlx(rename = "Value")]
    value: String,
}

pub async fn gather(pool: &MySqlPool) -> Result<Vec<Metric>, MysqlError> {
    let stats = sqlx::query_as::<_, GlobalStatus>(GLOBAL_STATUS_QUERY)
        .fetch_all(pool)
        .await
        .map_err(|err| MysqlError::Query {
            query: GLOBAL_STATUS_QUERY,
            err,
        })?;

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
                if text_items.contains_key(&key) {
                    text_items.insert(key.clone(), stat.value.clone());
                }
                continue;
            }
        };

        if is_global_status(&key) {
            metrics.push(Metric::gauge(
                format!("mysql_global_status_{}", key),
                "Generic metric from SHOW GLOBAL STATUS",
                fv,
            ));
            continue;
        }

        let (split_key, name) = match key.split_once('_') {
            Some((key, name)) => (key, Cow::from(name.to_string())),
            None => {
                // TODO: handle those metrics
                //   GlobalStatus { name: "Connections", value: "20" }
                //   GlobalStatus { name: "Queries", value: "248" }
                //   GlobalStatus { name: "Questions", value: "116" }
                //   GlobalStatus { name: "Uptime", value: "1321" }
                continue;
            }
        };

        match split_key {
            "com" => metrics.push(Metric::sum_with_tags(
                "mysql_global_status_commands_total",
                "Total number of executed MySQL commands",
                fv,
                tags!(
                    "command" => name
                ),
            )),
            "handler" => metrics.push(Metric::sum_with_tags(
                "mysql_global_status_handlers_total",
                "Total number of executed MySQL handlers",
                fv,
                tags!(
                    "handler" => name
                ),
            )),
            "connection_errors" => metrics.push(Metric::sum_with_tags(
                "mysql_global_status_connection_errors_total",
                "Total number of MySQL connection errors",
                fv,
                tags!(
                    "error" => name
                ),
            )),
            "innodb_buffer_pool_pages" => {
                match name.as_ref() {
                    "data" | "free" | "misc" | "old" => {
                        metrics.push(Metric::gauge_with_tags(
                            "mysql_global_status_buffer_pool_pages",
                            "Innodb buffer pool pages by state",
                            fv,
                            tags!(
                                "state" => name
                            ),
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
                            ),
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
                ),
            )),
            "performance_schema" => metrics.push(Metric::sum_with_tags(
                "mysql_global_status_performance_schema_lost_total",
                "Total number of MySQL instrumentations that could not be loaded or created due to memory constraints.",
                fv,
                tags!(
                    "instrumentation" => name
                ),
            )),
            _ => {
                metrics.push(Metric::gauge(
                    "mysql_global_status_".to_owned() + &key,
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
            ),
        ));
    }

    // mysql_galera_evs_repl_latency
    if text_items.get("wsrep_evs_repl_latency").unwrap() != "" {
        let mut evs = [
            (
                "min_seconds",
                0f64,
                0usize,
                "PXC/Galera group communication latency. Min value.",
            ),
            (
                "avg_seconds",
                0f64,
                1usize,
                "PXC/Galera group communication latency. Avg value.",
            ),
            (
                "max_seconds",
                0f64,
                2usize,
                "PXC/Galera group communication latency. Max value.",
            ),
            (
                "stdev",
                0f64,
                3usize,
                "PXC/Galera group communication latency. Standard Deviation.",
            ),
            (
                "sample_size",
                0f64,
                4usize,
                "PXC/Galera group communication latency. Sample Size.",
            ),
        ];

        let mut parsing_success = true;
        let values = text_items
            .get("wsrep_evs_repl_latency")
            .unwrap()
            .split('/')
            .collect::<Vec<_>>();

        if evs.len() == values.len() {
            for (_, value, index, _) in evs.iter_mut() {
                let index = *index;
                match values[index].parse::<f64>() {
                    Ok(v) => *value = v,
                    Err(_) => parsing_success = false,
                }
            }

            if parsing_success {
                for (name, value, _, desc) in evs {
                    metrics.push(Metric::gauge(
                        "mysql_galera_evs_repl_latency_".to_owned() + name,
                        desc,
                        value,
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
        "performance_schema_",
    ];

    GLOBAL_STATUS_PREFIXES.contains(&name)
}
