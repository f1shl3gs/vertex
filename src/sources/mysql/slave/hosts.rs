use event::{Metric, tags};

use super::{Connection, Error};

const SLAVE_HOSTS_QUERY: &str = "SHOW SLAVE_HOSTS";
const SHOW_REPLICAS_QUERY: &str = "SHOW REPLICAS";

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let version = conn.version();

    let mysql8022 = version >= (8, 0, 22);
    let query = if mysql8022 {
        SHOW_REPLICAS_QUERY
    } else {
        SLAVE_HOSTS_QUERY
    };

    let mut rows = conn.query(query).await?;

    let mut metrics = vec![];
    while let Some(mut row) = rows.next().await? {
        // Newer versions of mysql have the following
        // 		Server_id, Host, Port, Master_id, Slave_UUID
        // Older versions of mysql have the following
        // 		Server_id, Host, Port, Rpl_recovery_rank, Master_id
        // MySQL 5.5 and MariaDB 10.5 have the following
        // 		Server_id, Host, Port, Master_id
        let mut tags = tags!(
            "server_id" => "",
            "slave_host" => "",
            "port" => "",
            "master_id" => "",
            "slave_uuid" => "",
        );
        for column in row.columns() {
            let value = row.get_str();

            if column.name() == "Host" {
                tags.insert("slave_host", value);
            } else {
                tags.insert(column.name().to_lowercase(), value);
            }
        }

        metrics.push(Metric::gauge_with_tags(
            "mysql_heartbeat_mysql_slave_hosts_info",
            "Information about running slaves",
            1,
            tags,
        ))
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::mysql::assert_contains;
    use crate::sources::mysql::connection::mock;

    #[tokio::test]
    async fn slave_hosts_old_format() {
        let mut conn = mock(|_query| {
            (
                vec![
                    "Server_id",
                    "Host",
                    "Port",
                    "Rpl_recovery_rank",
                    "Master_id",
                ],
                vec![
                    vec!["380239978", "backup_server_1", "0", "1", "192168011"],
                    vec!["11882498", "backup_server_2", "0", "1", "192168011"],
                ],
            )
        })
        .await;

        let metrics = collect(&mut conn).await.unwrap();
        assert_contains(
            &metrics,
            vec![
                (
                    tags!("master_id" => "192168011", "port" => "0", "rpl_recovery_rank" => "1", "server_id" => "380239978", "slave_host" => "backup_server_1", "slave_uuid" => ""),
                    1.0,
                ),
                (
                    tags!("master_id" => "192168011", "port" => "0", "rpl_recovery_rank" => "1", "server_id" => "11882498", "slave_host" => "backup_server_2", "slave_uuid" => ""),
                    1.0,
                ),
            ],
            vec![],
        );
    }

    #[tokio::test]
    async fn slave_hosts_new_format() {
        let mut conn = mock(|_query| {
            (
                vec!["Server_id", "Host", "Port", "Master_id", "Slave_UUID"],
                vec![
                    vec![
                        "192168010",
                        "iconnect2",
                        "3306",
                        "192168011",
                        "14cb6624-7f93-11e0-b2c0-c80aa9429562",
                    ],
                    vec![
                        "1921680101",
                        "athena",
                        "3306",
                        "192168011",
                        "07af4990-f41f-11df-a566-7ac56fdaf645",
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
                    tags!("server_id" => "192168010", "slave_host" => "iconnect2", "port" => "3306", "master_id" => "192168011", "slave_uuid" => "14cb6624-7f93-11e0-b2c0-c80aa9429562"),
                    1.0,
                ),
                (
                    tags!("server_id" => "1921680101", "slave_host" =>  "athena", "port" => "3306", "master_id" => "192168011", "slave_uuid" => "07af4990-f41f-11df-a566-7ac56fdaf645"),
                    1.0,
                ),
            ],
            vec![],
        )
    }

    #[tokio::test]
    async fn slave_hosts_without_slave_uuid() {
        let mut conn = mock(|_query| {
            (
                vec!["Server_id", "Host", "Port", "Master_id"],
                vec![
                    vec!["192168010", "iconnect2", "3306", "192168012"],
                    vec!["1921680101", "athena", "3306", "192168012"],
                ],
            )
        })
        .await;

        let metrics = collect(&mut conn).await.unwrap();
        assert_contains(
            &metrics,
            vec![
                (
                    tags!("server_id" => "192168010", "slave_host" => "iconnect2", "port" => "3306", "master_id" => "192168012", "slave_uuid" => ""),
                    1.0,
                ),
                (
                    tags!("server_id" => "1921680101", "slave_host" => "athena", "port" => "3306", "master_id" => "192168012", "slave_uuid" => ""),
                    1.0,
                ),
            ],
            vec![],
        );
    }
}
