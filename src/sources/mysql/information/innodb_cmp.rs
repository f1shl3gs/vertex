// `information_schema.innodb_cmp`

use event::{Metric, tags};

use super::{Connection, Error};

const INNODB_CMP_QUERY: &str = "SELECT page_size, compress_ops, compress_ops_ok, compress_time, uncompress_ops, uncompress_time FROM information_schema.innodb_cmp";

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut rows = conn.query(INNODB_CMP_QUERY).await?;

    let mut metrics = Vec::new();
    while let Some(mut row) = rows.next().await? {
        let page_size = row.get_str();
        let compress_ops = row.get_str().parse::<f64>()?;
        let compress_ops_ok = row.get_str().parse::<f64>()?;
        let compress_time = row.get_str().parse::<f64>()?;
        let uncompress_ops = row.get_str().parse::<f64>()?;
        let uncompress_time = row.get_str().parse::<f64>()?;

        metrics.extend([
            Metric::sum_with_tags(
                "mysql_info_schema_innodb_cmp_compress_ops_total",
                "Number of times a B-tree page of the size PAGE_SIZE has been compressed.",
                compress_ops,
                tags!("page_size" => page_size),
            ),
            Metric::sum_with_tags(
                "mysql_info_schema_innodb_cmp_compress_ops_ok_total",
                "Number of times a B-tree page of the size PAGE_SIZE has been successfully compressed.",
                compress_ops_ok,
                tags!("page_size" => page_size),
            ),
            Metric::sum_with_tags(
                "mysql_info_schema_innodb_cmp_compress_time_seconds_total",
                "Total time in seconds spent in attempts to compress B-tree pages.",
                compress_time,
                tags!("page_size" => page_size),
            ),
            Metric::sum_with_tags(
                "mysql_info_schema_innodb_cmp_uncompress_ops_total",
                "Number of times a B-tree page of the size PAGE_SIZE has been uncompressed.",
                uncompress_ops,
                tags!("page_size" => page_size),
            ),
            Metric::sum_with_tags(
                "mysql_info_schema_innodb_cmp_uncompress_time_seconds_total",
                "Total time in seconds spent in uncompressing B-tree pages.",
                uncompress_time,
                tags!("page_size" => page_size),
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
    async fn innodb_cmp() {
        let mut conn = mock(|_| {
            (
                vec![
                    "page_size",
                    "compress_ops",
                    "compress_ops_ok",
                    "compress_time",
                    "uncompress_ops",
                    "uncompress_time",
                ],
                vec![vec!["1024", "10", "20", "30", "40", "50"]],
            )
        })
        .await;

        let metrics = collect(&mut conn).await.unwrap();
        assert_contains(
            &metrics,
            vec![],
            vec![
                (tags!("page_size" => "1024"), 10.0),
                (tags!("page_size" => "1024"), 20.0),
                (tags!("page_size" => "1024"), 30.0),
                (tags!("page_size" => "1024"), 40.0),
                (tags!("page_size" => "1024"), 50.0),
            ],
        );
    }
}
