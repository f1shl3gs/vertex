use event::{Metric, tags};

use super::{Connection, Error};

const TABLE_STAT_QUERY: &str = "SELECT
  TABLE_SCHEMA,
  TABLE_NAME,
  ROWS_READ,
  ROWS_CHANGED,
  ROWS_CHANGED_X_INDEXES
FROM information_schema.table_statistics";
pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut rows = conn.query(TABLE_STAT_QUERY).await?;

    let mut metrics = vec![];
    while let Some(mut row) = rows.next().await? {
        let schema = row.get_str();
        let table = row.get_str();
        let rows_read = row.get_str().parse::<u64>()?;
        let rows_changed = row.get_str().parse::<u64>()?;
        let rows_changed_x_indexes = row.get_str().parse::<u64>()?;

        metrics.extend([
            Metric::sum_with_tags(
                "mysql_info_schema_table_statistics_rows_read_total",
                "The number of rows read from the table.",
                rows_read,
                tags!("schema" => schema, "table" => table),
            ),
            Metric::sum_with_tags(
                "mysql_info_schema_table_statistics_rows_changed_total",
                "The number of rows changed in the table.",
                rows_changed,
                tags!("schema" => schema, "table" => table),
            ),
            Metric::sum_with_tags(
                "mysql_info_schema_table_statistics_rows_changed_x_indexes_total",
                "The number of rows changed in the table, multiplied by the number of indexes changed.",
                rows_changed_x_indexes,
                tags!("schema" => schema, "table" => table),
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
    async fn smoke() {
        let mut conn = mock(|_query| {
            (
                vec![
                    "TABLE_SCHEMA",
                    "TABLE_NAME",
                    "ROWS_READ",
                    "ROWS_CHANGED",
                    "ROWS_CHANGED_X_INDEXES",
                ],
                vec![
                    vec!["mysql", "db", "238", "0", "8"],
                    vec!["mysql", "proxies_priv", "99", "1", "0"],
                    vec!["mysql", "user", "1064", "2", "5"],
                ],
            )
        })
        .await;

        let metrics = collect(&mut conn).await.unwrap();
        assert_contains(
            &metrics,
            vec![],
            vec![
                (tags!("schema" => "mysql", "table" => "db"), 238.0),
                (tags!("schema" => "mysql", "table" => "db"), 0.0),
                (tags!("schema" => "mysql", "table" => "db"), 8.0),
                (tags!("schema" => "mysql", "table" => "proxies_priv"), 99.0),
                (tags!("schema" => "mysql", "table" => "proxies_priv"), 1.0),
                (tags!("schema" => "mysql", "table" => "proxies_priv"), 0.0),
                (tags!("schema" => "mysql", "table" => "user"), 1064.0),
                (tags!("schema" => "mysql", "table" => "user"), 2.0),
                (tags!("schema" => "mysql", "table" => "user"), 5.0),
            ],
        );
    }
}
