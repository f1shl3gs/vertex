use event::{tags, Metric};
use sqlx::MySqlPool;

use super::MysqlError;

const INNODB_CMP_MEMORY_QUERY: &str = r#"SELECT page_size, buffer_pool_instance, pages_used, pages_free, relocation_ops, relocation_time FROM information_schema.innodb_cmpmem"#;

#[derive(Debug, sqlx::FromRow)]
struct Record {
    page_size: i32,
    buffer_pool_instance: i32,
    pages_used: i32,
    pages_free: i32,
    relocation_ops: i32,
    relocation_time: i32,
}

pub async fn gather(pool: &MySqlPool) -> Result<Vec<Metric>, MysqlError> {
    let records = sqlx::query_as::<_, Record>(INNODB_CMP_MEMORY_QUERY)
        .fetch_all(pool)
        .await
        .map_err(|err| MysqlError::Query {
            query: INNODB_CMP_MEMORY_QUERY,
            err,
        })?;

    let mut metrics = Vec::with_capacity(records.len() * 4);
    for Record {
        page_size,
        buffer_pool_instance,
        pages_used,
        pages_free,
        relocation_ops,
        relocation_time,
    } in records
    {
        let page_size = &page_size.to_string();
        let buffer_pool = &buffer_pool_instance.to_string();

        metrics.extend([
            Metric::sum_with_tags(
                "mysql_info_schema_innodb_cmpmem_pages_used_total",
                "Number of blocks of the size PAGE_SIZe that are currently in use",
                pages_used,
                tags!(
                    "page_size" => page_size,
                    "buffer_pool" => buffer_pool
                ),
            ),
            Metric::sum_with_tags(
                "mysql_info_schema_innodb_cmpmem_pages_free_total",
                "Number of blocks of the size PAGE_SIZE that are currently available for allocation",
                pages_free,
                tags!(
                    "page_size" => page_size,
                    "buffer_pool" => buffer_pool
                ),
            ),
            Metric::sum_with_tags(
                "mysql_info_schema_innodb_cmpmem_relocation_ops_total",
                "Number of times a block of the size PAGE_SIZE has been relocated",
                relocation_ops,
                tags!(
                    "page_size" => page_size,
                    "buffer_pool" => buffer_pool
                ),
            ),
            Metric::sum_with_tags(
                "mysql_info_schema_innodb_cmpmem_relocation_time_seconds_total",
                "Total time in seconds spent in relocating blocks",
                relocation_time,
                tags!(
                    "page_size" => page_size,
                    "buffer_pool" => buffer_pool
                ),
            )
        ])
    }

    Ok(metrics)
}
