use event::{Metric, tags};

use super::{Connection, Error};

const DATABASES_QUERY: &str = "SELECT
  SCHEMA_NAME
FROM information_schema.schemata
WHERE SCHEMA_NAME NOT IN ('mysql', 'performance_schema', 'information_schema', 'sys')";

pub async fn collect(conn: &mut Connection, databases: &[String]) -> Result<Vec<Metric>, Error> {
    let databases = if databases.iter().any(|db| db == "*") {
        // all
        let mut rows = conn.query(DATABASES_QUERY).await?;
        let mut databases = Vec::new();
        while let Some(mut row) = rows.next().await? {
            databases.push(row.get_str().to_string());
        }

        databases
    } else {
        databases.to_vec()
    };

    let mut metrics = Vec::with_capacity(databases.len() * 5);
    for database in databases {
        let mut rows = conn
            .query(format!(
                "SELECT
		    TABLE_SCHEMA,
		    TABLE_NAME,
		    TABLE_TYPE,
		    ifnull(ENGINE, 'NONE') as ENGINE,
		    ifnull(VERSION, '0') as VERSION,
		    ifnull(ROW_FORMAT, 'NONE') as ROW_FORMAT,
		    ifnull(TABLE_ROWS, '0') as TABLE_ROWS,
		    ifnull(DATA_LENGTH, '0') as DATA_LENGTH,
		    ifnull(INDEX_LENGTH, '0') as INDEX_LENGTH,
		    ifnull(DATA_FREE, '0') as DATA_FREE,
		    ifnull(CREATE_OPTIONS, 'NONE') as CREATE_OPTIONS
		  FROM information_schema.tables
		  WHERE TABLE_SCHEMA = {}",
                database
            ))
            .await?;

        while let Some(mut row) = rows.next().await? {
            let schema = row.get_str();
            let table = row.get_str();
            let table_type = row.get_str();
            let engine = row.get_str();
            let version = row.get_str().parse::<u64>()?;
            let row_format = row.get_str();
            let table_rows = row.get_str().parse::<u64>()?;
            let data_length = row.get_str().parse::<u64>()?;
            let index_length = row.get_str().parse::<u64>()?;
            let data_free = row.get_str().parse::<u64>()?;
            let create_options = row.get_str();

            metrics.extend([
                Metric::gauge_with_tags(
                    "mysql_info_schema_table_version",
                    "The version number of the table's .frm file",
                    version,
                    tags!(
                        "schema" => schema,
                        "table" => table,
                        "type" => table_type,
                        "engine" => engine,
                        "row_format" => row_format,
                        "create_options" => create_options,
                    ),
                ),
                Metric::gauge_with_tags(
                    "mysql_info_schema_table_rows",
                    "The estimated number of rows in the table from information_schema.tables",
                    table_rows,
                    tags!("schema" => schema, "table" => table),
                ),
                Metric::gauge_with_tags(
                    "mysql_info_schema_table_size",
                    "The size of the table components from information_schema.tables",
                    data_length,
                    tags!("schema" => schema, "table" => table, "component" => "data_length"),
                ),
                Metric::gauge_with_tags(
                    "mysql_info_schema_table_size",
                    "The size of the table components from information_schema.tables",
                    index_length,
                    tags!("schema" => schema, "table" => table, "component" => "index_length"),
                ),
                Metric::gauge_with_tags(
                    "mysql_info_schema_table_size",
                    "The size of the table components from information_schema.tables",
                    data_free,
                    tags!("schema" => schema, "table" => table, "component" => "data_free"),
                ),
            ]);
        }
    }

    Ok(metrics)
}
