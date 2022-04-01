use event::{Bucket, Metric};
use snafu::ResultExt;
use sqlx;
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, MySqlPool, Row};

use super::{Error, QuerySnafu};

const RESPONSE_TIME_CHECK_QUERY: &str = r#"SELECT @@query_response_time_stats"#;
const RESPONSE_TIME_QUERY: &str =
    r#"SELECT TIME, COUNT, TOTAL FROM INFORMATION_SCHEMA.QUERY_RESPONSE_TIME"#;
const RESPONSE_TIME_READ_QUERY: &str =
    r#"SELECT TIME, COUNT, TOTAL FROM INFORMATION_SCHEMA.QUERY_RESPONSE_TIME_READ"#;
const RESPONSE_TIME_WRITE_QUERY: &str =
    r#"SELECT TIME, COUNT, TOTAL FROM INFORMATION_SCHEMA.QUERY_RESPONSE_TIME_WRITE"#;

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
    for query in [
        RESPONSE_TIME_QUERY,
        RESPONSE_TIME_READ_QUERY,
        RESPONSE_TIME_WRITE_QUERY,
    ] {
        metrics.push(query_response_time(pool, query).await?);
    }

    Ok(metrics)
}

async fn check_stats(pool: &MySqlPool) -> Result<bool, Error> {
    let status = sqlx::query_scalar::<_, i32>(RESPONSE_TIME_CHECK_QUERY)
        .fetch_one(pool)
        .await;

    match status {
        Ok(status) => Ok(status == 1),
        Err(err) => match err.as_database_error() {
            Some(db_err) => {
                if db_err.code() == Some("HY000".into()) {
                    Ok(false)
                } else {
                    Err(Error::Query {
                        source: err,
                        query: RESPONSE_TIME_CHECK_QUERY,
                    })
                }
            }
            _ => Err(Error::QuerySlaveStatus),
        },
    }
}

struct Statistic {
    time: f64,
    count: u64,
    total: f64,
}

impl<'r> FromRow<'r, MySqlRow> for Statistic {
    fn from_row(row: &'r MySqlRow) -> Result<Self, sqlx::Error> {
        let time = row
            .try_get::<String, _>("TIME")?
            .trim()
            .parse::<f64>()
            .unwrap_or(0.0);
        let count = row.try_get::<u64, _>("COUNT")?;
        let total = row
            .try_get::<String, _>("TOTAL")?
            .trim()
            .parse::<f64>()
            .unwrap_or(0.0);

        Ok(Self { time, count, total })
    }
}

async fn query_response_time(pool: &MySqlPool, query: &'static str) -> Result<Metric, Error> {
    let records = sqlx::query_as::<_, Statistic>(query)
        .fetch_all(pool)
        .await
        .context(QuerySnafu { query })?;

    let mut sum = 0.0;
    let mut count = 0;
    let mut buckets: Vec<Bucket> = Vec::with_capacity(16);

    for record in records.iter() {
        sum += record.total;
        count += record.count;

        // Special case for "TOO LONG" row where we take into account the count
        // field which is the only available and do not add it as a part of histogram
        // or metric
        if record.time == 0.0 {
            continue;
        }

        buckets.push(Bucket {
            upper: record.time,
            count: record.count,
        });
    }

    let (name, desc) = if query.contains("READ") {
        (
            "mysql_info_schema_read_query_response_time_seconds",
            "The number of read queries by duration they took to execute",
        )
    } else if query.contains("WRITE") {
        (
            "mysql_info_schema_write_query_response_time_seconds",
            "The number of write queries by duration they took to execute",
        )
    } else {
        (
            "mysql_info_schema_query_response_time_seconds",
            "The number of all queries by duration they took to execute",
        )
    };

    // buckets.sort_by(|a, b| a.upper.cmp(b.upper));

    Ok(Metric::histogram(name, desc, count, sum, buckets))
}
