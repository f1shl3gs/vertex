use event::{Metric, tags};

use super::{Connection, Error};

const SCHEMA_STAT_QUERY: &str = "SELECT
  TABLE_SCHEMA,
  SUM(ROWS_READ) AS ROWS_READ,
  SUM(ROWS_CHANGED) AS ROWS_CHANGED,
  SUM(ROWS_CHANGED_X_INDEXES) AS ROWS_CHANGED_X_INDEXES
FROM information_schema.TABLE_STATISTICS
GROUP BY TABLE_SCHEMA";

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut rows = conn.query(SCHEMA_STAT_QUERY).await?;

    let mut metrics = vec![];
    while let Some(mut row) = rows.next().await? {
        let schema = row.get_str();
        let rows_read = row.get_str().parse::<u64>()?;
        let rows_changed = row.get_str().parse::<u64>()?;
        let rows_changed_x_indexes = row.get_str().parse::<u64>()?;

        metrics.extend([
            Metric::sum_with_tags(
                "mysql_info_schema_schema_statistics_rows_read_total",
                "The number of rows read from the schema.",
                rows_read,
                tags!("schema" => schema)
            ),
            Metric::sum_with_tags(
                "mysql_info_schema_schema_statistics_rows_changed_total",
                "The number of rows changed in the schema.",
                rows_changed,
                tags!("schema" => schema)
            ),
            Metric::sum_with_tags(
                "mysql_info_schema_schema_statistics_rows_changed_x_indexes_total",
                "The number of rows changed in the schema, multiplied by the number of indexes changed.",
                rows_changed_x_indexes,
                tags!("schema" => schema)
            )
        ])
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
                    "ROWS_READ",
                    "ROWS_CHANGED",
                    "ROWS_CHANGED_X_INDEXES",
                ],
                vec![
                    vec!["mysql", "238", "0", "8"],
                    vec!["default", "99", "1", "0"],
                ],
            )
        })
        .await;

        let metrics = collect(&mut conn).await.unwrap();
        assert_contains(
            &metrics,
            vec![],
            vec![
                (tags!("schema" => "mysql"), 238.0),
                (tags!("schema" => "mysql"), 0.0),
                (tags!("schema" => "mysql"), 8.0),
                (tags!("schema" => "default"), 99.0),
                (tags!("schema" => "default"), 1.0),
                (tags!("schema" => "default"), 0.0),
            ],
        )
    }
}
