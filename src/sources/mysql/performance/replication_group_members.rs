// `performance_schema.replication_group_members`

use event::{Metric, tags};

use super::{Connection, Error};

const REPLICATION_GROUP_MEMBERS_QUERY: &str =
    "SELECT * FROM performance_schema.replication_group_members";

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut rows = conn.query(REPLICATION_GROUP_MEMBERS_QUERY).await?;

    let mut metrics = vec![];
    while let Some(mut row) = rows.next().await? {
        let mut tags = tags!();
        for column in row.columns() {
            let value = row.get_str();
            tags.insert(column.name().to_lowercase(), value);
        }

        metrics.push(Metric::gauge_with_tags(
            "mysql_perf_schema_replication_group_member_info",
            "Information about the replication group member: channel_name, member_id, member_host, member_port, member_state. (member_role and member_version where available)",
            1,
            tags,
        ));
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
                    "CHANNEL_NAME",
                    "MEMBER_ID",
                    "MEMBER_HOST",
                    "MEMBER_PORT",
                    "MEMBER_STATE",
                    "MEMBER_ROLE",
                    "MEMBER_VERSION",
                ],
                vec![
                    vec![
                        "group_replication_applier",
                        "uuid1",
                        "hostname1",
                        "3306",
                        "ONLINE",
                        "PRIMARY",
                        "8.0.19",
                    ],
                    vec![
                        "group_replication_applier",
                        "uuid2",
                        "hostname2",
                        "3306",
                        "ONLINE",
                        "SECONDARY",
                        "8.0.19",
                    ],
                    vec![
                        "group_replication_applier",
                        "uuid3",
                        "hostname3",
                        "3306",
                        "ONLINE",
                        "SECONDARY",
                        "8.0.19",
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
                    tags!("channel_name" => "group_replication_applier", "member_id" => "uuid1", "member_host" => "hostname1", "member_port" => "3306", "member_state" => "ONLINE", "member_role" => "PRIMARY", "member_version" => "8.0.19"),
                    1.0,
                ),
                (
                    tags!("channel_name" => "group_replication_applier", "member_id" => "uuid2", "member_host" => "hostname2", "member_port" => "3306", "member_state" => "ONLINE", "member_role" => "SECONDARY", "member_version" => "8.0.19"),
                    1.0,
                ),
                (
                    tags!("channel_name" => "group_replication_applier", "member_id" => "uuid3", "member_host" => "hostname3", "member_port" => "3306", "member_state" => "ONLINE", "member_role" => "SECONDARY", "member_version" => "8.0.19"),
                    1.0,
                ),
            ],
            vec![],
        );
    }

    #[tokio::test]
    async fn smoke_mysql57() {
        let mut conn = mock(|_| {
            (
                vec![
                    "CHANNEL_NAME",
                    "MEMBER_ID",
                    "MEMBER_HOST",
                    "MEMBER_PORT",
                    "MEMBER_STATE",
                ],
                vec![
                    vec![
                        "group_replication_applier",
                        "uuid1",
                        "hostname1",
                        "3306",
                        "ONLINE",
                    ],
                    vec![
                        "group_replication_applier",
                        "uuid2",
                        "hostname2",
                        "3306",
                        "ONLINE",
                    ],
                    vec![
                        "group_replication_applier",
                        "uuid3",
                        "hostname3",
                        "3306",
                        "ONLINE",
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
                    tags!("channel_name" => "group_replication_applier", "member_id" => "uuid1", "member_host" => "hostname1", "member_port" => "3306", "member_state" => "ONLINE"),
                    1.0,
                ),
                (
                    tags!("channel_name" => "group_replication_applier", "member_id" => "uuid2", "member_host" => "hostname2", "member_port" => "3306", "member_state" => "ONLINE"),
                    1.0,
                ),
                (
                    tags!("channel_name" => "group_replication_applier", "member_id" => "uuid3", "member_host" => "hostname3", "member_port" => "3306", "member_state" => "ONLINE"),
                    1.0,
                ),
            ],
            vec![],
        );
    }
}
