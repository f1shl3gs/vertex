use event::tags::Tags;
use event::{Metric, tags};

use super::{Connection, Error, Flavor};

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let version = conn.version();
    let query = if version.flavor() == Flavor::MariaDB && version >= (10, 0, 1) {
        "SHOW ALL SLAVES STATUS"
    } else if version.flavor() == Flavor::MySQL && version >= (8, 0, 22) {
        "SHOW REPLICA STATUS"
    } else {
        "SHOW SLAVE STATUS"
    };

    let mut rows = conn.query(query).await?;

    let mut metrics = vec![];
    while let Some(mut row) = rows.next().await? {
        // build tags
        let mut tags = tags!(
            "channel_name" => "",
            "connection_name" => "",
            "master_uuid" => ""
        );
        for column in row.columns() {
            let value = row.get_str();

            match column.name() {
                "Master_UUID" | "Source_UUID" => tags.insert("master_uuid", value),
                "Master_Host" | "Source_Host" => tags.insert("master_host", value),
                "Channel_Name" => tags.insert("channel_name", value),
                "Connection_name" => tags.insert("connection_name", value),
                _ => continue,
            }
        }

        // generate metrics
        row.reset();
        for column in row.columns() {
            let value = row.get_str();
            if value.is_empty() {
                continue;
            }

            match column.name() {
                "Gtid_IO_Pos" | "Gtid_Slave_Pos" => {
                    metrics.extend(parse_gtid(column.name(), value, &tags));
                }
                "Master_UUID" | "Source_UUID" | "Master_Host" | "Source_Host" | "Channel_Name"
                | "Connection_name" => continue,
                name => {
                    // silently skip unparsable values
                    let Some(value) = parse_status(value) else {
                        continue;
                    };

                    metrics.push(Metric::gauge_with_tags(
                        format!("mysql_slave_status_{}", name.to_lowercase()),
                        "Generic metric from SHOW SLAVE STATUS",
                        value,
                        tags.clone(),
                    ))
                }
            }
        }
    }

    Ok(metrics)
}

fn parse_gtid(name: &str, value: &str, tags: &Tags) -> Vec<Metric> {
    let mut metrics = vec![];

    for gtid in value.split(",") {
        let mut parts = gtid.split("-");

        let Some(domain_id) = parts.next() else {
            continue;
        };
        let Some(server_id) = parts.next() else {
            continue;
        };
        let Some(sequence_num) = parts.next() else {
            continue;
        };
        let Ok(sequence_num) = sequence_num.parse::<u64>() else {
            continue;
        };

        let mut tags = tags.clone();
        tags.insert("domain_id", domain_id);
        tags.insert("server_id", server_id);

        metrics.push(Metric::gauge_with_tags(
            format!("mysql_slave_status_{}", name.to_lowercase()),
            format!("{} metric from SHOW SLAVE STATUS", name),
            sequence_num,
            tags,
        ));
    }

    metrics
}

fn parse_status(value: &str) -> Option<f64> {
    if let Ok(value) = value.parse::<f64>() {
        return Some(value);
    }

    // SHOW SLAVE STATUS Slave_IO_Running can return "Connecting" which is a non-running state
    if value == "Connecting" {
        return Some(0.0);
    }

    if value == "Yes" {
        return Some(1.0);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::mysql::assert_contains;
    use crate::sources::mysql::connection::mock;
    use event::tags;

    #[tokio::test]
    async fn smoke() {
        let mut conn = mock(|_query| {
            (
                vec![
                    "Master_Host",
                    "Read_Master_Log_Pos",
                    "Slave_IO_Running",
                    "Slave_SQL_Running",
                    "Seconds_Behind_Master",
                    "Gtid_IO_Pos",
                ],
                vec![vec![
                    "127.0.0.1",
                    "1",
                    "Connecting",
                    "Yes",
                    "2",
                    "0-1-2,3-4-5",
                ]],
            )
        })
        .await;

        let metrics = collect(&mut conn).await.unwrap();
        assert_contains(
            &metrics,
            vec![
                (
                    tags!("channel_name" => "", "connection_name" => "", "master_host" => "127.0.0.1", "master_uuid" => ""),
                    1.0,
                ),
                (
                    tags!("channel_name" => "", "connection_name" => "", "master_host" => "127.0.0.1", "master_uuid" => ""),
                    0.0,
                ),
                (
                    tags!("channel_name" => "", "connection_name" => "", "master_host" => "127.0.0.1", "master_uuid" => ""),
                    1.0,
                ),
                (
                    tags!("channel_name" => "", "connection_name" => "", "master_host" => "127.0.0.1", "master_uuid" => ""),
                    2.0,
                ),
                (
                    tags!("channel_name" => "", "connection_name" => "", "master_host" => "127.0.0.1", "master_uuid" => "", "domain_id" => "0", "server_id" => "1"),
                    2.0,
                ),
                (
                    tags!("channel_name" => "", "connection_name" => "", "master_host" => "127.0.0.1", "master_uuid" => "", "domain_id" => "3", "server_id" => "4"),
                    5.0,
                ),
            ],
            vec![],
        )
    }
}
