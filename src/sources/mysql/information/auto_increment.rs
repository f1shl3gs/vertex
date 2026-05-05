use event::{Metric, tags};

use super::{Connection, Error};

const AUTO_INCREMENT_QUERY: &str = "SELECT c.table_schema, c.table_name, column_name, auto_increment,
  pow(2, case data_type
    when 'tinyint'   then 7
    when 'smallint'  then 15
    when 'mediumint' then 23
    when 'int'       then 31
    when 'bigint'    then 63
  end+(column_type like '% unsigned'))-1 as max_int
FROM information_schema.columns c
STRAIGHT_JOIN information_schema.tables t ON (BINARY c.table_schema=t.table_schema AND BINARY c.table_name=t.table_name)
WHERE c.extra = 'auto_increment' AND t.auto_increment IS NOT NULL";

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut rows = conn.query(AUTO_INCREMENT_QUERY).await?;

    let mut metrics = Vec::new();
    while let Some(mut row) = rows.next().await? {
        let schema = row.get_str();
        let table = row.get_str();
        let column = row.get_str();
        let value = row.get_str().parse::<f64>()?;
        let max = row.get_str().parse::<f64>()?;

        metrics.extend([
            Metric::gauge_with_tags(
                "mysql_info_schema_auto_increment_column",
                "The current value of an auto_increment column from information_schema.",
                value,
                tags! {"schema" => schema, "table" => table, "column" => column},
            ),
            Metric::gauge_with_tags(
                "mysql_info_schema_auto_increment_column_max",
                "The max value of an auto_increment column from information_schema.",
                max,
                tags! {"schema" => schema, "table" => table, "column" => column},
            ),
        ]);
    }

    Ok(metrics)
}
