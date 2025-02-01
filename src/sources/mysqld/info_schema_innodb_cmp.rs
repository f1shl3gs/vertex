use event::{tags, Metric};
use sqlx::MySqlPool;

use super::MysqlError;

const INNODB_CMP_QUERY: &str = r#"SELECT page_size, compress_ops, compress_ops_ok, compress_time, uncompress_ops, uncompress_time FROM information_schema.innodb_cmp"#;

#[derive(Debug, sqlx::FromRow)]
struct Record {
    page_size: i32,
    compress_ops: i32,
    compress_ops_ok: i32,
    compress_time: i32,
    uncompress_ops: i32,
    uncompress_time: i32,
}

pub async fn gather(pool: &MySqlPool) -> Result<Vec<Metric>, MysqlError> {
    let records = sqlx::query_as::<_, Record>(INNODB_CMP_QUERY)
        .fetch_all(pool)
        .await
        .map_err(|err| MysqlError::Query {
            err,
            query: INNODB_CMP_QUERY,
        })?;

    let mut metrics = Vec::with_capacity(5 * records.len());

    for record in records {
        let page_size = &record.page_size.to_string();

        metrics.extend([
            Metric::sum_with_tags(
                "mysql_info_schema_innodb_cmp_compress_ops_total",
                "Number of times a B-tree page of the size PAGE_SIZE has been compressed",
                record.compress_ops,
                tags!(
                    "page_size" => page_size
                ),
            ),
            Metric::sum_with_tags(
                "mysql_info_schema_innodb_cmp_compress_ops_ok_total",
                "Number of times a B-tree page of the size PAGE_SIZE has been successfully compressed",
                record.compress_ops_ok,
                tags!(
                    "page_size" => page_size
                ),
            ),
            Metric::sum_with_tags(
                "mysql_info_schema_innodb_cmp_compress_time_seconds_total",
                "Number of times a B-tree page of the size PAGE_SIZE has been successfully compressed",
                record.compress_time,
                tags!(
                    "page_size" => page_size
                ),
            ),
            Metric::sum_with_tags(
                "mysql_info_schema_innodb_cmp_uncompress_ops_total",
                "Number of times a B-tree page of the size PAGE_SIZe has been uncompressed",
                record.uncompress_ops,
                tags!(
                    "page_size" => page_size,
                ),
            ),
            Metric::sum_with_tags(
                "mysql_info_schema_innodb_cmp_uncompress_time_seconds_total",
                "Total time in secnods spent in uncompressing B-tree pages",
                record.uncompress_time,
                tags!(
                    "page_size" => page_size
                ),
            )
        ]);
    }

    Ok(metrics)
}
