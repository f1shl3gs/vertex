use std::collections::HashMap;

use configurable::Configurable;
use event::{Metric, tags};
use framework::config::default_true;
use serde::{Deserialize, Serialize};

use super::{Connection, Error};

#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    /// Minimum time a thread must be in each state to be counted
    min_time: u32,

    /// Enable collecting the number of processes by user
    #[serde(default = "default_true")]
    processes_by_user: bool,

    /// Enable collecting the number of processes by host
    #[serde(default = "default_true")]
    processes_by_host: bool,
}

// `information_schema.processlist`
pub async fn collect(conn: &mut Connection, conf: &Config) -> Result<Vec<Metric>, Error> {
    let statement = format!(
        "SELECT
  user,
  SUBSTRING_INDEX(host, ':', 1) AS host,
  COALESCE(command, '') AS command,
  COALESCE(state, '') AS state,
  COUNT(*) AS processes,
  SUM(time) AS seconds
FROM information_schema.processlist
WHERE ID != connection_id()
  AND TIME >= {}
GROUP BY user, host, command, state",
        conf.min_time
    );

    let mut rows = conn.query(statement).await?;

    // command -> state -> (count, time)
    let mut state_counts = HashMap::<String, HashMap<String, (u32, u32)>>::new();
    let mut state_host_counts = HashMap::<String, u32>::new();
    let mut state_user_counts = HashMap::<String, u32>::new();

    while let Some(mut row) = rows.next().await? {
        let user = row.get_str();
        let host = row.get_str();
        let command = row.get_str();
        let state = row.get_str();
        let count = row.get_str().parse::<u32>()?;
        let time = row.get_str().parse::<u32>()?;

        let host = if host.is_empty() { "unknown" } else { host };
        let command = sanitize_state(command);
        let state = sanitize_state(state);

        state_counts
            .entry(command)
            .or_default()
            .entry(state)
            .and_modify(|value| {
                value.0 += count;
                value.1 += time;
            })
            .or_insert((count, time));

        state_host_counts
            .entry(host.to_string())
            .and_modify(|value| *value += count)
            .or_insert(count);
        state_user_counts
            .entry(user.to_string())
            .and_modify(|value| *value += count)
            .or_insert(count);
    }

    let mut metrics =
        Vec::with_capacity(state_counts.values().fold(0usize, |acc, m| acc + m.len()));
    for (command, map) in state_counts {
        for (state, (count, time)) in map {
            metrics.extend([
                Metric::gauge_with_tags(
                    "mysql_info_schema_processlist_threads",
                    "The number of threads split by current state",
                    count,
                    tags!( "command" => &command, "state" => &state),
                ),
                Metric::gauge_with_tags(
                    "mysql_info_schema_processlist_seconds",
                    "The number of seconds threads have used split by current state",
                    time,
                    tags!( "command" => &command, "state" => &state),
                ),
            ]);
        }
    }

    if conf.processes_by_user {
        metrics.extend(state_user_counts.into_iter().map(|(user, count)| {
            Metric::gauge_with_tags(
                "mysql_info_schema_processlist_processes_by_user",
                "The number of processes by user",
                count,
                tags!("mysql_user" => user),
            )
        }));
    }

    if conf.processes_by_host {
        metrics.extend(state_host_counts.into_iter().map(|(host, count)| {
            Metric::gauge_with_tags(
                "mysql_info_schema_processlist_processes_by_host",
                "The number of processed by host",
                count,
                tags!("client_host" => host),
            )
        }))
    }

    Ok(metrics)
}

fn sanitize_state(state: &str) -> String {
    if state.is_empty() {
        return "unknown".to_string();
    }

    let mut output = String::with_capacity(state.len());
    for ch in state.chars() {
        if [';', ',', ':', '.', '(', ')'].contains(&ch) {
            continue;
        }

        if ch == ' ' || ch == '-' {
            output.push('_');
        } else {
            output.push(ch.to_ascii_lowercase());
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::mysql::assert_contains;
    use crate::sources::mysql::connection::mock;

    #[tokio::test]
    async fn process_list() {
        let config = Config {
            processes_by_host: true,
            processes_by_user: true,
            min_time: 0,
        };

        let mut conn = mock(|_query| {
            (
                vec!["user", "host", "command", "state", "processes", "seconds"],
                vec![
                    vec!["manager", "10.0.7.234", "Sleep", "", "10", "87"],
                    vec!["feedback", "10.0.7.154", "Sleep", "", "8", "842"],
                    vec!["root", "10.0.7.253", "Sleep", "", "1", "20"],
                    vec!["feedback", "10.0.7.179", "Sleep", "", "2", "14"],
                    vec![
                        "system user",
                        "",
                        "Connect",
                        "waiting for handler commit",
                        "1",
                        "7271248",
                    ],
                    vec!["manager", "10.0.7.234", "Sleep", "", "4", "62"],
                    vec![
                        "system user",
                        "",
                        "Query",
                        "Slave has read all relay log; waiting for more updates",
                        "1",
                        "7271248",
                    ],
                    vec![
                        "event_scheduler",
                        "localhost",
                        "Daemon",
                        "Waiting on empty queue",
                        "1",
                        "7271248",
                    ],
                ],
            )
        })
        .await;

        let metrics = collect(&mut conn, &config).await.unwrap();

        assert_contains(
            &metrics,
            vec![
                (
                    tags!("command" => "connect", "state" => "waiting_for_handler_commit"),
                    1.0,
                ),
                (
                    tags!("command" => "connect", "state" => "waiting_for_handler_commit"),
                    7271248.0,
                ),
                (
                    tags!("command" => "daemon", "state" => "waiting_on_empty_queue"),
                    1.0,
                ),
                (
                    tags!("command" => "daemon", "state" => "waiting_on_empty_queue"),
                    7271248.0,
                ),
                (
                    tags!("command" => "query", "state" => "slave_has_read_all_relay_log_waiting_for_more_updates"),
                    1.0,
                ),
                (
                    tags!("command" => "query", "state" => "slave_has_read_all_relay_log_waiting_for_more_updates"),
                    7271248.0,
                ),
                (tags!("command" => "sleep", "state" => "unknown"), 25.0),
                (tags!("command" => "sleep", "state" => "unknown"), 1025.0),
                (tags!("client_host" => "10.0.7.154"), 8.0),
                (tags!("client_host" => "10.0.7.179"), 2.0),
                (tags!("client_host" => "10.0.7.234"), 14.0),
                (tags!("client_host" => "10.0.7.253"), 1.0),
                (tags!("client_host" => "localhost"), 1.0),
                (tags!("client_host" => "unknown"), 2.0),
                (tags!("mysql_user" => "event_scheduler"), 1.0),
                (tags!("mysql_user" => "feedback"), 10.0),
                (tags!("mysql_user" => "manager"), 14.0),
                (tags!("mysql_user" => "root"), 1.0),
                (tags!("mysql_user" => "system user"), 2.0),
            ],
            vec![],
        );
    }
}
