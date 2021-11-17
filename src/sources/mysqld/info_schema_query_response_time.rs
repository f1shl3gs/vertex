use std::collections::{BTreeMap, HashMap};
use std::ops::Index;
use futures::StreamExt;
use rdkafka::admin::ConfigSource::Default;
use snafu::ResultExt;
use sqlx::{FromRow, MySqlPool, Row};
use event::{Bucket, Metric, MetricValue};
use sqlx;
use sqlx::mysql::MySqlRow;

use super::{QueryFailed, Error};


const RESPONSE_TIME_CHECK_QUERY: &str = r#"SELECT @@query_response_time_stats"#;
const RESPONSE_TIME_QUERY: &str = r#"SELECT TIME, COUNT, TOTAL FROM INFORMATION_SCHEMA.QUERY_RESPONSE_TIME"#;
const RESPONSE_TIME_READ_QUERY: &str = r#"SELECT TIME, COUNT, TOTAL FROM INFORMATION_SCHEMA.QUERY_RESPONSE_TIME_READ"#;
const RESPONSE_TIME_WRITE_QUERY: &str = r#"SELECT TIME, COUNT, TOTAL FROM INFORMATION_SCHEMA.QUERY_RESPONSE_TIME_WRITE"#;

// 5.5 is the version of MySQL from which scraper is available.
pub async fn gather(pool: &MySqlPool) -> Result<Vec<Metric>, Error> {
    // This features is provided by a plugin called "QUERY_RESPONSE_TIME", and it is
    // disabled by default.
    //
    // more information: https://www.percona.com/doc/percona-server/5.7/diagnostics/response_time_distribution.html
    if !check_stats(pool).await? {
        return Ok(vec![]);
    }

    let mut metrics = Vec::new();
    for query in [RESPONSE_TIME_QUERY, RESPONSE_TIME_READ_QUERY, RESPONSE_TIME_WRITE_QUERY] {
        // todo
    }

    Ok(metrics)
}

async fn check_stats(pool: &MySqlPool) -> Result<bool, Error> {
    let status = sqlx::query_scalar::<_, i32>(RESPONSE_TIME_CHECK_QUERY)
        .fetch_one(pool)
        .await
        .context(QueryFailed { query: RESPONSE_TIME_CHECK_QUERY })?;

    Ok(status != 0)
}

/*
mysql> SELECT TIME, COUNT, TOTAL FROM INFORMATION_SCHEMA.QUERY_RESPONSE_TIME_READ;
+----------------+-------+----------------+
| TIME           | COUNT | TOTAL          |
+----------------+-------+----------------+
|       0.000001 |     0 |       0.000000 |
|       0.000010 |     0 |       0.000000 |
|       0.000100 |     3 |       0.000121 |
|       0.001000 |     0 |       0.000000 |
|       0.010000 |     0 |       0.000000 |
|       0.100000 |     0 |       0.000000 |
|       1.000000 |     0 |       0.000000 |
|      10.000000 |     0 |       0.000000 |
|     100.000000 |     0 |       0.000000 |
|    1000.000000 |     0 |       0.000000 |
|   10000.000000 |     0 |       0.000000 |
|  100000.000000 |     0 |       0.000000 |
| 1000000.000000 |     0 |       0.000000 |
| TOO LONG       |     0 | TOO LONG       |
+----------------+-------+----------------+
14 rows in set (0.00 sec)
*/
#[derive(Debug, sqlx::FromRow)]
struct Statistic {
    #[sqlx(rename = "TIME")]
    time: String,
    #[sqlx(rename = "COUNT")]
    count: u32,
    #[sqlx(rename = "TOTAL")]
    total: String,
}

/*
impl<'r> FromRow<'r, PgRow> for User {
    fn from_row(row: &'r PgRow) -> Result<Self, Error> {
        let name = row.try_get("name")?;
        let status = row.try_get("status")?;

        Ok(User{ name, status })
    }
}
*/

impl<'r> FromRow<'r, MySqlRow> for Statistic {
    fn from_row(row: &'r MySqlRow) -> Result<Self, sqlx::Error> {
        let time = row.try_get()
    }
}

async fn query_response_time(pool: &MySqlPool, query: &'static str) -> Result<Metric, Error> {
    let records = sqlx::query_as::<_, Statistic>(query)
        .fetch_all(pool)
        .await
        .context(QueryFailed { query })?;

    let mut total = 0.0;
    let mut count = 0;
    let mut buckets: Vec<Bucket> = Vec::with_capacity(16);

    'outer: for record in records.iter() {
        let record_time = match record.time.parse() {
            Ok(fv) => fv,
            Err(_) => continue
        };
        let record_total = match record.total.parse() {
            Ok(fv) => fv,
            Err(_) => continue
        };

        total += record.total;
        count += record.count;

        for b in buckets.iter_mut() {
            if b.upper != record.time {
                continue;
            }

            b.count = count;
            continue 'outer;
        }

        buckets.push(Bucket {
            upper: record.time,
            count: record.count,
        });
    }

    let (name, desc) = if query.contains("READ") {
        (
            "mysql_info_schema_read_query_response_time_seconds",
            "The number of read queries by duration they took to execute"
        )
    } else if query.contains("WRITE") {
        (
            "mysql_info_schema_write_query_response_time_seconds",
            "The number of write queries by duration they took to execute"
        )
    } else {
        (
            "mysql_info_schema_query_response_time_seconds",
            "The number of all queries by duration they took to execute"
        )
    };

    // buckets.sort_by(|a, b| a.upper.cmp(b.upper));

    Ok(Metric {
        name: name.to_string(),
        description: Some(desc.to_string()),
        tags: BTreeMap::new(),
        unit: None,
        timestamp: None,
        value: MetricValue::Histogram {
            count: count as u64,
            sum: total as f64,
            buckets,
        },
    })
}

#[cfg(test)]
mod tests {
    use sqlx::mysql::{MySqlConnectOptions, MySqlSslMode};
    use super::*;

    #[tokio::test]
    async fn test_gather() {
        let pool = MySqlPool::connect_with(MySqlConnectOptions::new()
            .host("127.0.0.1")
            .username("root")
            .password("password")
            .port(3307)
            .ssl_mode(MySqlSslMode::Disabled))
            .await
            .unwrap();

        let m = query_response_time(&pool, RESPONSE_TIME_QUERY).await.unwrap();
        println!("{:#?}", m);
    }
}