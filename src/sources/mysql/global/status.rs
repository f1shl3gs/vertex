use event::{Metric, tags};

use super::{Connection, Error, parse_value, sanitize};

const GLOBAL_STATUS_QUERY: &str = r#"SHOW GLOBAL STATUS"#;

#[derive(Default)]
struct TextItems {
    wsrep_local_state_uuid: Option<String>,
    wsrep_cluster_state_uuid: Option<String>,
    wsrep_provider_version: Option<String>,
    wsrep_evs_repl_latency: Option<String>,
}

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut rows = conn.query(GLOBAL_STATUS_QUERY).await?;

    let mut text_items = TextItems::default();
    let mut metrics = Vec::with_capacity(32);
    while let Some(mut row) = rows.next().await? {
        let name = row.get_str();
        let value = row.get_str();

        let Some(value) = parse_value(value) else {
            match name {
                "wsrep_local_state_uuid" => text_items.wsrep_local_state_uuid = Some(value.into()),
                "wsrep_cluster_state_uuid" => {
                    text_items.wsrep_cluster_state_uuid = Some(value.into())
                }
                "wsrep_provider_version" => text_items.wsrep_provider_version = Some(value.into()),
                "wsrep_evs_repl_latency" => text_items.wsrep_evs_repl_latency = Some(value.into()),
                _ => {}
            }

            continue;
        };

        if let Some(command) = name.strip_prefix("Com_") {
            metrics.push(Metric::sum_with_tags(
                "mysql_global_status_commands_total",
                "Total number of executed MySQL commands.",
                value,
                tags!("command" => command),
            ));
        } else if let Some(handler) = name.strip_prefix("Handler_") {
            metrics.push(Metric::sum_with_tags(
                "mysql_global_status_handlers_total",
                "Total number of executed MySQL handlers",
                value,
                tags!("handler" => handler),
            ));
        } else if let Some(err) = name.strip_prefix("Connection_errors_") {
            metrics.push(Metric::sum_with_tags(
                "mysql_global_status_connection_errors_total",
                "Total number of MySQL connection errors",
                value,
                tags!("error" => err),
            ));
        } else if let Some(state) = name.strip_prefix("Innodb_buffer_pool_pages_") {
            match state {
                "data" | "free" | "misc" | "old" => metrics.push(Metric::gauge_with_tags(
                    "mysql_global_status_buffer_pool_pages",
                    "Innodb buffer pool pages by state",
                    value,
                    tags!("state" => state),
                )),
                "dirty" => {
                    metrics.push(Metric::gauge(
                        "mysql_global_status_buffer_pool_dirty_pages",
                        "Innodb buffer pool dirty pages",
                        value,
                    ));
                }
                "total" => continue,
                _ => metrics.push(Metric::sum_with_tags(
                    "mysql_global_status_buffer_pool_page_changes_total",
                    "Innodb buffer pool page state changes",
                    value,
                    tags!("operation" => sanitize(state)),
                )),
            }
        } else if let Some(operation) = name.strip_prefix("Innodb_rows_") {
            metrics.push(Metric::sum_with_tags(
                "mysql_global_status_innodb_row_ops_total",
                "Total number of MySQL InnoDB row operations",
                value,
                tags!("operation" => operation),
            ))
        } else if let Some(instrumentation) = name.strip_prefix("Performance_schema_") {
            metrics.push(Metric::sum_with_tags(
                "mysql_global_status_performance_schema_lost_total",
                "Total number of MySQL instrumentations that could not be loaded or created due to memory constraints",
                value,
                tags!("instrumentation" => instrumentation),
            ))
        } else {
            metrics.push(Metric::gauge(
                format!("mysql_global_status_{}", sanitize(name)),
                "Generic metric from SHOW GLOBAL STATUS.",
                value,
            ));
        }
    }

    // mysql_galera_variables_info metric
    if let Some(wsrep_local_state_uuid) = text_items.wsrep_local_state_uuid {
        metrics.push(Metric::gauge_with_tags(
            "mysql_galera_status_info",
            "PXC/Galera status information.",
            1,
            tags!(
                "wsrep_local_state_uuid" => wsrep_local_state_uuid,
                "wsrep_cluster_state_uuid" => text_items.wsrep_cluster_state_uuid.unwrap_or_default(),
                "wsrep_provider_version" => text_items.wsrep_provider_version.unwrap_or_default(),
            ),
        ))
    }

    if let Some(value) = text_items.wsrep_evs_repl_latency
        && let Ok(values) = value
            .split("/")
            .take(5)
            .map(|s| s.parse::<f64>())
            .collect::<Result<Vec<_>, _>>()
        && values.len() == 5
    {
        metrics.extend([
            Metric::gauge(
                "mysql_galera_evs_repl_latency_min_seconds",
                "PXC/Galera group communication latency. Min value.",
                values[0],
            ),
            Metric::gauge(
                "mysql_galera_evs_repl_latency_avg_seconds",
                "PXC/Galera group communication latency. Avg value.",
                values[1],
            ),
            Metric::gauge(
                "mysql_galera_evs_repl_latency_max_seconds",
                "PXC/Galera group communication latency. Max value.",
                values[2],
            ),
            Metric::gauge(
                "mysql_galera_evs_repl_latency_stdev",
                "PXC/Galera group communication latency. Standard Deviation.",
                values[3],
            ),
            Metric::gauge(
                "mysql_galera_evs_repl_latency_sample_size",
                "PXC/Galera group communication latency. Sample Size.",
                values[4],
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
        let mut conn = mock(|_query| {
            (
                vec!["Variable_name", "Value"],
                vec![
                    vec!["Com_alter_db", "1"],
                    vec!["Com_show_status", "2"],
                    vec!["Com_select", "3"],
                    vec!["Connection_errors_internal", "4"],
                    vec!["Handler_commit", "5"],
                    vec!["Innodb_buffer_pool_pages_data", "6"],
                    vec!["Innodb_buffer_pool_pages_flushed", "7"],
                    vec!["Innodb_buffer_pool_pages_dirty", "7"],
                    vec!["Innodb_buffer_pool_pages_free", "8"],
                    vec!["Innodb_buffer_pool_pages_misc", "9"],
                    vec!["Innodb_buffer_pool_pages_old", "10"],
                    vec!["Innodb_buffer_pool_pages_total", "11"],
                    vec!["Innodb_buffer_pool_pages_lru_flushed", "13"],
                    vec!["Innodb_buffer_pool_pages_made_not_young", "14"],
                    vec!["Innodb_buffer_pool_pages_made_young", "15"],
                    vec!["Innodb_rows_read", "8"],
                    vec!["Performance_schema_users_lost", "9"],
                    vec!["Slave_running", "OFF"],
                    vec!["Ssl_version", ""],
                    vec!["Uptime", "10"],
                    vec!["validate_password.dictionary_file_words_count", "11"],
                    vec!["wsrep_cluster_status", "Primary"],
                    vec![
                        "wsrep_local_state_uuid",
                        "6c06e583-686f-11e6-b9e3-8336ad58138c",
                    ],
                    vec![
                        "wsrep_cluster_state_uuid",
                        "6c06e583-686f-11e6-b9e3-8336ad58138c",
                    ],
                    vec!["wsrep_provider_version", "3.16(r5c765eb)"],
                    vec![
                        "wsrep_evs_repl_latency",
                        "0.000227664/0.00034135/0.000544298/6.03708e-05/212",
                    ],
                ],
            )
        })
        .await;

        let metrics = collect(&mut conn).await.unwrap();
        assert_contains(
            &metrics,
            vec![
                (tags!("state" => "data"), 1.0),
                (tags!(), 7.0),
                (tags!("state" => "free"), 8.0),
                (tags!("state" => "misc"), 9.0),
                (tags!("state" => "old"), 10.0),
                (
                    tags!(
                        "wsrep_local_state_uuid" => "6c06e583-686f-11e6-b9e3-8336ad58138c",
                        "wsrep_cluster_state_uuid" => "6c06e583-686f-11e6-b9e3-8336ad58138c",
                        "wsrep_provider_version" => "3.16(r5c765eb)"),
                    1.0,
                ),
                (tags!(), 0.000227664),
                (tags!(), 0.00034135),
                (tags!(), 0.000544298),
                (tags!(), 6.03708e-05),
                (tags!(), 0.0),
                (tags!(), 10.0),
                (tags!(), 11.0),
                (tags!(), 1.0),
            ],
            vec![
                (tags!("command" => "alter_db"), 1.0),
                (tags!("command" => "show_status"), 2.0),
                (tags!("command" => "select"), 3.0),
                (tags!("error" => "internal"), 4.0),
                (tags!("handler" => "commit"), 5.0),
                (tags!("operation" => "flushed"), 7.0),
                (tags!("operation" => "lru_flushed"), 13.0),
                (tags!("operation" => "made_not_young"), 14.0),
                (tags!("operation" => "made_young"), 15.0),
                (tags!("operation" => "read"), 16.0),
                (tags!("instrumentation" => "users_lost"), 9.0),
            ],
        );
    }
}
