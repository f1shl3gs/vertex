// `performance_schema.events_statements_summary_by_digest`

use std::time::Duration;

use configurable::Configurable;
use event::{Metric, Quantile, tags};
use serde::{Deserialize, Serialize};

use super::{Connection, Error, Flavor};

const PICO_SECONDS: f64 = 1e12;

const fn default_limit() -> usize {
    250
}

const fn default_time_limit() -> Duration {
    Duration::from_secs(86_400)
}

const fn default_digest_text_limit() -> usize {
    120
}

#[derive(Configurable, Clone, Default, Serialize, Deserialize, Debug)]
pub struct Config {
    /// Limit the number of events statements digests by response time
    #[serde(default = "default_limit")]
    limit: usize,

    /// Limit how old the 'last_seen' events statements can be
    #[serde(default = "default_time_limit", with = "humanize::duration::serde")]
    time_limit: Duration,

    /// Maximum length of the normalized statement text
    #[serde(default = "default_digest_text_limit")]
    digest_text_limit: usize,

    /// Additional schema name to exclude (always excludes mysql, performance_schema, information_schema)
    exclude_schemas: Vec<String>,
}

pub async fn collect(conn: &mut Connection, conf: &Config) -> Result<Vec<Metric>, Error> {
    let version = conn.version();
    let mysql_8028 = version.flavor() == Flavor::MySQL && version >= (8, 0, 28);
    let query = if mysql_8028 {
        format!(
            "SELECT
	    ifnull(SCHEMA_NAME, 'NONE') as SCHEMA_NAME,
	    DIGEST,
	    LEFT(DIGEST_TEXT, {}) as DIGEST_TEXT,
	    COUNT_STAR,
	    SUM_TIMER_WAIT,
	    SUM_LOCK_TIME,
	    SUM_CPU_TIME,
	    SUM_ERRORS,
	    SUM_WARNINGS,
	    SUM_ROWS_AFFECTED,
	    SUM_ROWS_SENT,
	    SUM_ROWS_EXAMINED,
	    SUM_CREATED_TMP_DISK_TABLES,
	    SUM_CREATED_TMP_TABLES,
	    SUM_SORT_MERGE_PASSES,
	    SUM_SORT_ROWS,
	    SUM_NO_INDEX_USED,
	    QUANTILE_95,
	    QUANTILE_99,
	    QUANTILE_999
	  FROM (
	    SELECT *
	    FROM performance_schema.events_statements_summary_by_digest
	    WHERE SCHEMA_NAME NOT IN ({})
	      AND LAST_SEEN > DATE_SUB(NOW(), INTERVAL {} SECOND)
	    ORDER BY LAST_SEEN DESC
	  )Q
	  GROUP BY
	    Q.SCHEMA_NAME,
	    Q.DIGEST,
	    Q.DIGEST_TEXT,
	    Q.COUNT_STAR,
	    Q.SUM_TIMER_WAIT,
	    Q.SUM_LOCK_TIME,
	    Q.SUM_CPU_TIME,
	    Q.SUM_ERRORS,
	    Q.SUM_WARNINGS,
	    Q.SUM_ROWS_AFFECTED,
	    Q.SUM_ROWS_SENT,
	    Q.SUM_ROWS_EXAMINED,
	    Q.SUM_CREATED_TMP_DISK_TABLES,
	    Q.SUM_CREATED_TMP_TABLES,
	    Q.SUM_SORT_MERGE_PASSES,
	    Q.SUM_SORT_ROWS,
	    Q.SUM_NO_INDEX_USED,
	    Q.QUANTILE_95,
	    Q.QUANTILE_99,
	    Q.QUANTILE_999
	  ORDER BY SUM_TIMER_WAIT DESC
	  LIMIT {}",
            conf.digest_text_limit,
            build_excluded_schemas(&conf.exclude_schemas),
            conf.time_limit.as_secs(),
            conf.limit
        )
    } else {
        format!(
            "SELECT
	    ifnull(SCHEMA_NAME, 'NONE') as SCHEMA_NAME,
	    DIGEST,
	    LEFT(DIGEST_TEXT, {}) as DIGEST_TEXT,
	    COUNT_STAR,
	    SUM_TIMER_WAIT,
	    SUM_ERRORS,
	    SUM_WARNINGS,
	    SUM_ROWS_AFFECTED,
	    SUM_ROWS_SENT,
	    SUM_ROWS_EXAMINED,
	    SUM_CREATED_TMP_DISK_TABLES,
	    SUM_CREATED_TMP_TABLES,
	    SUM_SORT_MERGE_PASSES,
	    SUM_SORT_ROWS,
	    SUM_NO_INDEX_USED
	  FROM (
	    SELECT *
	    FROM performance_schema.events_statements_summary_by_digest
	    WHERE SCHEMA_NAME NOT IN ({})
	      AND LAST_SEEN > DATE_SUB(NOW(), INTERVAL {} SECOND)
	    ORDER BY LAST_SEEN DESC
	  )Q
	  GROUP BY
	    Q.SCHEMA_NAME,
	    Q.DIGEST,
	    Q.DIGEST_TEXT,
	    Q.COUNT_STAR,
	    Q.SUM_TIMER_WAIT,
	    Q.SUM_ERRORS,
	    Q.SUM_WARNINGS,
	    Q.SUM_ROWS_AFFECTED,
	    Q.SUM_ROWS_SENT,
	    Q.SUM_ROWS_EXAMINED,
	    Q.SUM_CREATED_TMP_DISK_TABLES,
	    Q.SUM_CREATED_TMP_TABLES,
	    Q.SUM_SORT_MERGE_PASSES,
	    Q.SUM_SORT_ROWS,
	    Q.SUM_NO_INDEX_USED
	  ORDER BY SUM_TIMER_WAIT DESC
	  LIMIT {}",
            conf.digest_text_limit,
            build_excluded_schemas(&conf.exclude_schemas),
            conf.time_limit.as_secs(),
            conf.limit
        )
    };

    let mut rows = conn.query(query).await?;

    let mut metrics = vec![];
    while let Some(mut row) = rows.next().await? {
        let schema = row.get_str();
        let digest = row.get_str();
        let digest_text = row.get_str();
        let count = row.get_str().parse::<u64>()?;
        let query_time = row.get_str().parse::<f64>()?;

        let (lock_time, cpu_time) = if mysql_8028 {
            let lock_time = row.get_str().parse::<f64>()?;
            let cpu_time = row.get_str().parse::<f64>()?;
            (lock_time, cpu_time)
        } else {
            (0.0, 0.0)
        };

        let errors = row.get_str().parse::<u64>()?;
        let warnings = row.get_str().parse::<u64>()?;
        let rows_affected = row.get_str().parse::<u64>()?;
        let rows_sent = row.get_str().parse::<u64>()?;
        let rows_examined = row.get_str().parse::<u64>()?;
        let tmp_disk_tables = row.get_str().parse::<u64>()?;
        let tmp_tables = row.get_str().parse::<u64>()?;
        let sort_merge_passes = row.get_str().parse::<u64>()?;
        let sort_rows = row.get_str().parse::<u64>()?;
        let no_index_used = row.get_str().parse::<u64>()?;

        let (quantile_95, quantile_99, quantile_999) = if mysql_8028 {
            let quantile_95 = row.get_str().parse::<f64>()?;
            let quantile_99 = row.get_str().parse::<f64>()?;
            let quantile_999 = row.get_str().parse::<f64>()?;

            (quantile_95, quantile_99, quantile_999)
        } else {
            (0.0, 0.0, 0.0)
        };

        metrics.extend([
            Metric::sum_with_tags(
                "mysql_perf_schema_events_statements_total",
                "The total count of events statements by digest.",
                count,
                tags!("schema" => schema, "digest" => digest, "digest_text" => digest_text),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_events_statements_seconds_total",
                "The total time of events statements by digest.",
                query_time / PICO_SECONDS,
                tags!("schema" => schema, "digest" => digest, "digest_text" => digest_text),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_events_statements_lock_time_seconds_total",
                "The total lock time of events statements by digest.",
                lock_time / PICO_SECONDS,
                tags!("schema" => schema, "digest" => digest, "digest_text" => digest_text),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_events_statements_cpu_time_seconds_total",
                "The total cpu time of events statements by digest.",
                cpu_time / PICO_SECONDS,
                tags!("schema" => schema, "digest" => digest, "digest_text" => digest_text),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_events_statements_errors_total",
                "The errors of events statements by digest.",
                errors,
                tags!("schema" => schema, "digest" => digest, "digest_text" => digest_text),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_events_statements_warnings_total",
                "The warnings of events statements by digest.",
                warnings,
                tags!("schema" => schema, "digest" => digest, "digest_text" => digest_text),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_events_statements_rows_affected_total",
                "The total rows affected of events statements by digest.",
                rows_affected,
                tags!("schema" => schema, "digest" => digest, "digest_text" => digest_text),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_events_statements_rows_sent_total",
                "The total rows sent of events statements by digest.",
                rows_sent,
                tags!("schema" => schema, "digest" => digest, "digest_text" => digest_text),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_events_statements_rows_examined_total",
                "The total rows examined of events statements by digest.",
                rows_examined,
                tags!("schema" => schema, "digest" => digest, "digest_text" => digest_text),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_events_statements_tmp_tables_total",
                "The total tmp tables of events statements by digest.",
                tmp_tables,
                tags!("schema" => schema, "digest" => digest, "digest_text" => digest_text),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_events_statements_tmp_disk_tables_total",
                "The total tmp disk tables of events statements by digest.",
                tmp_disk_tables,
                tags!("schema" => schema, "digest" => digest, "digest_text" => digest_text),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_events_statements_sort_merge_passes_total",
                "The total number of merge passes by the sort algorithm performed by digest.",
                sort_merge_passes,
                tags!("schema" => schema, "digest" => digest, "digest_text" => digest_text),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_events_statements_sort_rows_total",
                "The total number of sorted rows by digest.",
                sort_rows,
                tags!("schema" => schema, "digest" => digest, "digest_text" => digest_text),
            ),
            Metric::sum_with_tags(
                "mysql_perf_schema_events_statements_no_index_used_total",
                "The total number of statements that used full table scans by digest.",
                no_index_used,
                tags!("schema" => schema, "digest" => digest, "digest_text" => digest_text),
            ),
            Metric::summary_with_tags(
                "mysql_perf_schema_events_statements_latency",
                "A summary of statement latency by digest",
                count,
                query_time / PICO_SECONDS,
                vec![
                    Quantile {
                        quantile: 95.0,
                        value: quantile_95 / PICO_SECONDS,
                    },
                    Quantile {
                        quantile: 99.0,
                        value: quantile_99 / PICO_SECONDS,
                    },
                    Quantile {
                        quantile: 999.0,
                        value: quantile_999 / PICO_SECONDS,
                    },
                ],
                tags!("schema" => schema, "digest" => digest, "digest_text" => digest_text),
            ),
        ]);
    }

    Ok(metrics)
}

fn build_excluded_schemas(extra_schemas: &[String]) -> String {
    let mut excluded = vec!["mysql", "performance_schema", "information_schema"];

    for extra in extra_schemas {
        if !excluded.contains(&extra.as_str()) {
            excluded.push(extra);
        }
    }

    excluded.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::mysql::assert_contains;
    use crate::sources::mysql::connection::mock;

    #[tokio::test]
    async fn smoke() {
        let mut conn = mock(|_| {
            (
                vec![
                    "SCHEMA_NAME",
                    "DIGEST",
                    "DIGEST_TEXT",
                    "COUNT_STAR",
                    "SUM_TIMER_WAIT",
                    "SUM_ERRORS",
                    "SUM_WARNINGS",
                    "SUM_ROWS_AFFECTED",
                    "SUM_ROWS_SENT",
                    "SUM_ROWS_EXAMINED",
                    "SUM_CREATED_TMP_DISK_TABLES",
                    "SUM_CREATED_TMP_TABLES",
                    "SUM_SORT_MERGE_PASSES",
                    "SUM_SORT_ROWS",
                    "SUM_NO_INDEX_USED",
                ],
                vec![vec![
                    "db1",
                    "digest1",
                    "SELECT * FROM test",
                    "100",
                    "1000",
                    "1",
                    "2",
                    "50",
                    "100",
                    "150",
                    "1",
                    "2",
                    "3",
                    "100",
                    "1",
                ]],
            )
        })
        .await;

        conn.set_flavor(Flavor::MySQL);
        conn.set_version(8, 0, 0);

        let conf = Config::default();
        let metrics = collect(&mut conn, &conf).await.unwrap();
        assert_contains(
            &metrics,
            vec![],
            vec![
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    100.0,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    1000.0 / PICO_SECONDS,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    0.0,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    0.0,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    1.0,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    2.0,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    50.0,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    100.0,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    150.0,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    2.0,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    1.0,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    3.0,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    100.0,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    1.0,
                ),
            ],
        );
    }

    #[tokio::test]
    async fn smoke_mysql_8028() {
        let mut conn = mock(|_| {
            (
                vec![
                    "SCHEMA_NAME",
                    "DIGEST",
                    "DIGEST_TEXT",
                    "COUNT_STAR",
                    "SUM_TIMER_WAIT",
                    "SUM_LOCK_TIME",
                    "SUM_CPU_TIME",
                    "SUM_ERRORS",
                    "SUM_WARNINGS",
                    "SUM_ROWS_AFFECTED",
                    "SUM_ROWS_SENT",
                    "SUM_ROWS_EXAMINED",
                    "SUM_CREATED_TMP_DISK_TABLES",
                    "SUM_CREATED_TMP_TABLES",
                    "SUM_SORT_MERGE_PASSES",
                    "SUM_SORT_ROWS",
                    "SUM_NO_INDEX_USED",
                    "QUANTILE_95",
                    "QUANTILE_99",
                    "QUANTILE_999",
                ],
                vec![vec![
                    "db1",
                    "digest1",
                    "SELECT * FROM test",
                    "100",
                    "1000",
                    "30",
                    "50",
                    "1",
                    "2",
                    "50",
                    "100",
                    "150",
                    "1",
                    "2",
                    "3",
                    "100",
                    "1",
                    "100",
                    "150",
                    "200",
                ]],
            )
        })
        .await;

        conn.set_flavor(Flavor::MySQL);
        conn.set_version(8, 0, 28);

        let metrics = collect(&mut conn, &Config::default()).await.unwrap();
        assert_contains(
            &metrics,
            vec![],
            vec![
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    100.0,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    1000.0 / PICO_SECONDS,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    30.0 / PICO_SECONDS,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    50.0 / PICO_SECONDS,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    1.0,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    2.0,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    50.0,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    100.0,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    150.0,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    2.0,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    1.0,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    3.0,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    100.0,
                ),
                (
                    tags!("schema" => "db1", "digest" => "digest1", "digest_text" => "SELECT * FROM test"),
                    1.0,
                ),
            ],
        )
    }
}
