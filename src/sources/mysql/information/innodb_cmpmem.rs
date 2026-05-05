// `information_schema.innodb_cmp`

use event::{Metric, tags};

use super::{Connection, Error};

const INNODB_CMP_MEM_QUERY: &str = "SELECT page_size, buffer_pool_instance, pages_used, pages_free, relocation_ops, relocation_time FROM information_schema.innodb_cmpmem";

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut rows = conn.query(INNODB_CMP_MEM_QUERY).await?;

    let mut metrics = Vec::new();
    while let Some(mut row) = rows.next().await? {
        let page_size = row.get_str();
        let buffer_pool = row.get_str();
        let pages_used = row.get_str().parse::<f64>()?;
        let pages_free = row.get_str().parse::<f64>()?;
        let relocation_ops = row.get_str().parse::<f64>()?;
        let relocation_time = row.get_str().parse::<f64>()?;

        metrics.extend([
            Metric::sum_with_tags(
                "mysql_info_schema_innodb_cmpmem_pages_used_total",
                "Number of blocks of the size PAGE_SIZE that are currently in use.",
                pages_used,
                tags!("page_size" => page_size, "buffer_pool" => buffer_pool),
            ),
            Metric::sum_with_tags(
                "mysql_info_schema_innodb_cmpmem_pages_free_total",
                "Number of blocks of the size PAGE_SIZE that are currently available for allocation.",
                pages_free,
                tags!("page_size" => page_size, "buffer_pool" => buffer_pool),
            ),
            Metric::sum_with_tags(
                "mysql_info_schema_innodb_cmpmem_relocation_ops_total",
                "Number of times a block of the size PAGE_SIZE has been relocated.",
                relocation_ops,
                tags!("page_size" => page_size, "buffer_pool" => buffer_pool),
            ),
            Metric::sum_with_tags(
                "mysql_info_schema_innodb_cmpmem_relocation_time_seconds_total",
                "Total time in seconds spent in relocating blocks.",
                relocation_time,
                tags!("page_size" => page_size, "buffer_pool" => buffer_pool),
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
    async fn innodb_cmpmem() {
        let mut conn = mock(|_| {
            (
                vec![
                    "page_size",
                    "buffer_pool",
                    "pages_used",
                    "pages_free",
                    "relocation_ops",
                    "relocation_time",
                ],
                vec![vec!["1024", "0", "30", "40", "50", "6000"]],
            )
        })
        .await;

        let metrics = collect(&mut conn).await.unwrap();
        assert_contains(
            &metrics,
            vec![],
            vec![
                (tags!("page_size" => "1024", "buffer_pool" => "0"), 30.0),
                (tags!("page_size" => "1024", "buffer_pool" => "0"), 40.0),
                (tags!("page_size" => "1024", "buffer_pool" => "0"), 50.0),
                (tags!("page_size" => "1024", "buffer_pool" => "0"), 6.0),
            ],
        );
    }
}
