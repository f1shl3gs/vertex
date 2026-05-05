use event::Metric;

use super::{Connection, Error};

const LOGBIN_QUERY: &str = "SELECT @@log_bin";
const BINGLOG_QUERY: &str = "SHOW BINARY LOGS";

// `SHOW BINARY LOGS`
pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut rows = conn.query(LOGBIN_QUERY).await?;
    let mut first = rows.next().await?.ok_or(Error::NoData)?;
    let Ok(value) = first.get_str().parse::<u8>() else {
        return Err(Error::InvalidData);
    };

    // draining the eof
    while rows.next().await?.is_some() {}

    let mut count = 0;
    let mut filesize = 0;
    let mut rows = conn.query(BINGLOG_QUERY).await?;
    while let Some(mut row) = rows.next().await? {
        let _filename = row.get_str();
        let Ok(size) = row.get_str().parse::<u64>() else {
            return Err(Error::InvalidData);
        };

        count += 1;
        filesize += size;
    }

    Ok(vec![
        Metric::gauge(
            "mysql_binlog_size_bytes",
            "Combined size of all registered binlog files.",
            filesize,
        ),
        Metric::gauge(
            "mysql_binlog_files",
            "Number of registered binlog files.",
            count,
        ),
        Metric::gauge(
            "mysql_binlog_file_number",
            "The last binlog file number.",
            value,
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::super::connection::mock;
    use super::*;
    use event::MetricValue;

    #[tokio::test]
    async fn binlog_size() {
        let mut conn = mock(|query| match query {
            LOGBIN_QUERY => (vec![""], vec![vec!["1"]]),
            BINGLOG_QUERY => (
                vec!["Log_name", "File_size"],
                vec![
                    vec!["centos6-bin.000001", "1813"],
                    vec!["centos6-bin.000002", "120"],
                    vec!["centos6-bin.000444", "573009"],
                ],
            ),
            _ => unimplemented!("{}", query),
        })
        .await;

        let metrics = collect(&mut conn).await.unwrap();
        assert_eq!(metrics.len(), 3);
        assert_eq!(
            metrics[0].value,
            MetricValue::Gauge(1813.0 + 120.0 + 573009.0)
        );
        assert_eq!(metrics[1].value, MetricValue::Gauge(3.0));
        assert_eq!(metrics[2].value, MetricValue::Gauge(1.0));
    }
}
