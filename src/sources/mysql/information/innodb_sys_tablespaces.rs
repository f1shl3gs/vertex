// information_schema.innodb_sys_tablespaces

use event::{Metric, tags};

use super::{Connection, Error, Flavor};

const INNODB_TABLESPACES_TABLE_NAME_QUERY: &str = "
SELECT table_name
FROM information_schema.tables
WHERE table_name = 'INNODB_SYS_TABLESPACES' OR table_name = 'INNODB_TABLESPACES'";

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let version = conn.version();

    let mut rows = conn.query(INNODB_TABLESPACES_TABLE_NAME_QUERY).await?;
    let Some(mut row) = rows.next().await? else {
        return Err(Error::NoData);
    };

    let name = row.get_str();
    let query = if name == "INNODB_SYS_TABLESPACES" || name == "INNODB_TABLESPACES" {
        if version.flavor() == Flavor::MariaDB && version >= 10.5 {
            format!(
                "SELECT
	    SPACE,
	    NAME,
	    ifnull((SELECT column_name
			FROM information_schema.COLUMNS
			WHERE TABLE_SCHEMA = 'information_schema'
			  AND TABLE_NAME = '{}' +
			  AND COLUMN_NAME = 'FILE_FORMAT' LIMIT 1), 'NONE') as FILE_FORMAT,
	    ifnull(ROW_FORMAT, 'NONE') as ROW_FORMAT,
	    FILE_SIZE,
	    ALLOCATED_SIZE
	  FROM information_schema.{}",
                name, name
            )
        } else {
            format!(
                "SELECT
	    SPACE,
	    NAME,
	    ifnull((SELECT column_name
			FROM information_schema.COLUMNS
			WHERE TABLE_SCHEMA = 'information_schema'
			  AND TABLE_NAME =  + '{}' +
			  AND COLUMN_NAME = 'FILE_FORMAT' LIMIT 1), 'NONE') as FILE_FORMAT,
	    ifnull(ROW_FORMAT, 'NONE') as ROW_FORMAT,
	    ifnull(SPACE_TYPE, 'NONE') as SPACE_TYPE,
	    FILE_SIZE,
	    ALLOCATED_SIZE
	  FROM information_schema.{}",
                name, name
            )
        }
    } else {
        debug!(
            message =
                "couldn't find INNODB_SYS_TABLESPACES or INNODB_TABLESPACES in information_schema",
        );

        // draining all incoming packets (include eof)
        while rows.next().await?.is_some() {}

        return Err(Error::NoData);
    };

    // draining all incoming packets (include eof)
    while rows.next().await?.is_some() {}

    let mut rows = conn.query(query).await?;
    let mut metrics = vec![];

    while let Some(mut row) = rows.next().await? {
        let table_space = row.get_str().parse::<u32>()?;
        let table_name = row.get_str();
        let file_format = row.get_str();
        let row_format = row.get_str();

        let space_type = if row.columns().len() == 7 {
            row.get_str()
        } else {
            ""
        };

        let file_size = row.get_str().parse::<u64>()?;
        let allocated_size = row.get_str().parse::<u64>()?;

        metrics.extend([
            Metric::gauge_with_tags(
                "mysql_info_schema_innodb_tablespace_space_info",
                "The Tablespace information and Space ID.",
                table_space,
                tags!(
                    "tablespace_name" => table_name,
                    "file_format" => file_format,
                    "row_format" => row_format,
                    "space_type" => space_type
                )
            ),
            Metric::gauge_with_tags(
                "mysql_info_schema_innodb_tablespace_file_size_bytes",
                "The apparent size of the file, which represents the maximum size of the file, uncompressed.",
                file_size,
                tags!("tablespace_name" => table_name)
            ),
            Metric::gauge_with_tags(
                "mysql_info_schema_innodb_tablespace_allocated_size_bytes",
                "The actual size of the file, which is the amount of space allocated on disk.",
                allocated_size,
                tags!("tablespace_name" => table_name)
            )
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
    async fn tablespaces() {
        let mut conn = mock(|query| {
            if query == INNODB_TABLESPACES_TABLE_NAME_QUERY {
                (vec!["TABLE_NAME"], vec![vec!["INNODB_SYS_TABLESPACES"]])
            } else {
                (
                    vec![
                        "SPACE",
                        "NAME",
                        "FILE_FORMAT",
                        "ROW_FORMAT",
                        "SPACE_TYPE",
                        "FILE_SIZE",
                        "ALLOCATED_SIZE",
                    ],
                    vec![
                        vec![
                            "1",
                            "sys/sys_config",
                            "Barracuda",
                            "Dynamic",
                            "Single",
                            "100",
                            "100",
                        ],
                        vec![
                            "2",
                            "db/compressed",
                            "Barracuda",
                            "Compressed",
                            "Single",
                            "300",
                            "200",
                        ],
                    ],
                )
            }
        })
        .await;

        let metrics = collect(&mut conn).await.unwrap();

        assert_contains(
            &metrics,
            vec![
                (
                    tags!("tablespace_name" => "sys/sys_config", "file_format" => "Barracuda", "row_format" => "Dynamic", "space_type" => "Single"),
                    1.0,
                ),
                (tags!("tablespace_name" => "sys/sys_config"), 100.0),
                (tags!("tablespace_name" => "sys/sys_config"), 100.0),
                (
                    tags!("tablespace_name" => "db/compressed", "file_format" => "Barracuda", "row_format" => "Compressed", "space_type" => "Single"),
                    2.0,
                ),
                (tags!("tablespace_name" => "db/compressed"), 300.0),
                (tags!("tablespace_name" => "db/compressed"), 200.0),
            ],
            vec![],
        );
    }

    #[tokio::test]
    async fn without_tablespaces() {
        let mut conn = mock(|query| {
            if query == INNODB_TABLESPACES_TABLE_NAME_QUERY {
                (vec!["TABLE_NAME"], vec![vec!["INNODB_SYS_TABLESPACES"]])
            } else {
                (
                    vec![
                        "SPACE",
                        "NAME",
                        "FILE_FORMAT",
                        "ROW_FORMAT",
                        "FILE_SIZE",
                        "ALLOCATED_SIZE",
                    ],
                    vec![
                        vec!["1", "sys/sys_config", "Barracuda", "Dynamic", "100", "100"],
                        vec![
                            "2",
                            "db/compressed",
                            "Barracuda",
                            "Compressed",
                            "300",
                            "200",
                        ],
                    ],
                )
            }
        })
        .await;
        conn.set_flavor(Flavor::MariaDB);

        let metrics = collect(&mut conn).await.unwrap();

        assert_contains(
            &metrics,
            vec![
                (
                    tags!("tablespace_name" => "sys/sys_config", "file_format" => "Barracuda", "row_format" => "Dynamic", "space_type" => ""),
                    1.0,
                ),
                (tags!("tablespace_name" => "sys/sys_config"), 100.0),
                (tags!("tablespace_name" => "sys/sys_config"), 100.0),
                (
                    tags!("tablespace_name" => "db/compressed", "file_format" => "Barracuda", "row_format" => "Compressed", "space_type" => ""),
                    2.0,
                ),
                (tags!("tablespace_name" => "db/compressed"), 300.0),
                (tags!("tablespace_name" => "db/compressed"), 200.0),
            ],
            vec![],
        );
    }
}
