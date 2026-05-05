// `performance_schema.events_statements_summary_by_digest`

use event::Metric;

use super::{Connection, Error};

const EVENTS_STATEMENTS_SUM_QUERY: &str = "SELECT
  SUM(COUNT_STAR) AS SUM_COUNT_STAR,
  SUM(SUM_CREATED_TMP_DISK_TABLES) AS SUM_SUM_CREATED_TMP_DISK_TABLES,
  SUM(SUM_CREATED_TMP_TABLES) AS SUM_SUM_CREATED_TMP_TABLES,
  SUM(SUM_ERRORS) AS SUM_SUM_ERRORS,
  SUM(SUM_LOCK_TIME) AS SUM_SUM_LOCK_TIME,
  SUM(SUM_NO_GOOD_INDEX_USED) AS SUM_SUM_NO_GOOD_INDEX_USED,
  SUM(SUM_NO_INDEX_USED) AS SUM_SUM_NO_INDEX_USED,
  SUM(SUM_ROWS_AFFECTED) AS SUM_SUM_ROWS_AFFECTED,
  SUM(SUM_ROWS_EXAMINED) AS SUM_SUM_ROWS_EXAMINED,
  SUM(SUM_ROWS_SENT) AS SUM_SUM_ROWS_SENT,
  SUM(SUM_SELECT_FULL_JOIN) AS SUM_SUM_SELECT_FULL_JOIN,
  SUM(SUM_SELECT_FULL_RANGE_JOIN) AS SUM_SUM_SELECT_FULL_RANGE_JOIN,
  SUM(SUM_SELECT_RANGE) AS SUM_SUM_SELECT_RANGE,
  SUM(SUM_SELECT_RANGE_CHECK) AS SUM_SUM_SELECT_RANGE_CHECK,
  SUM(SUM_SELECT_SCAN) AS SUM_SUM_SELECT_SCAN,
  SUM(SUM_SORT_MERGE_PASSES) AS SUM_SUM_SORT_MERGE_PASSES,
  SUM(SUM_SORT_RANGE) AS SUM_SUM_SORT_RANGE,
  SUM(SUM_SORT_ROWS) AS SUM_SUM_SORT_ROWS,
  SUM(SUM_SORT_SCAN) AS SUM_SUM_SORT_SCAN,
  SUM(SUM_TIMER_WAIT) AS SUM_SUM_TIMER_WAIT,
  SUM(SUM_WARNINGS) AS SUM_SUM_WARNINGS
FROM performance_schema.events_statements_summary_by_digest";

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut rows = conn.query(EVENTS_STATEMENTS_SUM_QUERY).await?;

    let mut metrics = vec![];
    while let Some(mut row) = rows.next().await? {
        let total = row.get_str().parse::<u64>()?;
        let created_tmp_disk_tables = row.get_str().parse::<u64>()?;
        let created_tmp_tables = row.get_str().parse::<u64>()?;
        let errors = row.get_str().parse::<u64>()?;
        let lock_time = row.get_str().parse::<u64>()?;
        let no_good_index_used = row.get_str().parse::<u64>()?;
        let no_index_used = row.get_str().parse::<u64>()?;
        let rows_affected = row.get_str().parse::<u64>()?;
        let rows_examined = row.get_str().parse::<u64>()?;
        let rows_sent = row.get_str().parse::<u64>()?;
        let select_full_join = row.get_str().parse::<u64>()?;
        let select_full_range_join = row.get_str().parse::<u64>()?;
        let select_range = row.get_str().parse::<u64>()?;
        let select_range_check = row.get_str().parse::<u64>()?;
        let select_scan = row.get_str().parse::<u64>()?;
        let sort_merge_passes = row.get_str().parse::<u64>()?;
        let sort_range = row.get_str().parse::<u64>()?;
        let sort_rows = row.get_str().parse::<u64>()?;
        let sort_scan = row.get_str().parse::<u64>()?;
        let timer_wait = row.get_str().parse::<u64>()?;
        let warnings = row.get_str().parse::<u64>()?;

        metrics.extend([
            Metric::sum(
                "mysql_perf_schema_events_statements_sum_total",
                "The total count of events statements.",
                total,
            ),
            Metric::sum(
                "mysql_perf_schema_events_statements_sum_created_tmp_disk_tables",
                "The number of on-disk temporary tables created.",
                created_tmp_disk_tables
            ),
            Metric::sum(
               "mysql_perf_schema_events_statements_sum_created_tmp_tables",
               "The number of temporary tables created.",
               created_tmp_tables
            ),
            Metric::sum(
               "mysql_perf_schema_events_statements_sum_errors",
               "Number of errors.",
               errors
            ),
            Metric::sum(
               "mysql_perf_schema_events_statements_sum_lock_time",
               "Time in picoseconds spent waiting for locks.",
               lock_time
            ),
            Metric::sum(
               "mysql_perf_schema_events_statements_sum_no_good_index_used",
               "Number of times no good index was found.",
               no_good_index_used
            ),
            Metric::sum(
               "mysql_perf_schema_events_statements_sum_no_index_used",
               "Number of times no index was found.",
               no_index_used,
            ),
            Metric::sum(
               "mysql_perf_schema_events_statements_sum_rows_affected",
               "Number of rows affected by statements.",
               rows_affected
            ),
            Metric::sum(
               "mysql_perf_schema_events_statements_sum_rows_examined",
               "Number of rows read during statements' execution.",
               rows_examined
            ),
            Metric::sum(
               "mysql_perf_schema_events_statements_sum_rows_sent",
               "Number of rows returned.",
               rows_sent
            ),
            Metric::sum(
               "mysql_perf_schema_events_statements_sum_select_full_join",
               "Number of joins performed by statements which did not use an index.",
               select_full_join
            ),
            Metric::sum(
               "mysql_perf_schema_events_statements_sum_select_full_range_join",
               "Number of joins performed by statements which used a range search of the first table.",
               select_full_range_join
            ),
            Metric::sum(
               "mysql_perf_schema_events_statements_sum_select_range",
               "Number of joins performed by statements which used a range of the first table.",
               select_range
            ),
            Metric::sum(
               "mysql_perf_schema_events_statements_sum_select_range_check",
               "Number of joins without keys performed by statements that check for key usage after each row.",
               select_range_check
            ),
            Metric::sum(
               "mysql_perf_schema_events_statements_sum_select_scan",
               "Number of joins performed by statements which used a full scan of the first table.",
               select_scan
            ),
            Metric::sum(
               "mysql_perf_schema_events_statements_sum_sort_merge_passes",
               "Number of merge passes by the sort algorithm performed by statements.",
               sort_merge_passes
            ),
            Metric::sum(
               "mysql_perf_schema_events_statements_sum_sort_range",
               "Number of sorts performed by statements which used a range.",
               sort_range
            ),
            Metric::sum(
               "mysql_perf_schema_events_statements_sum_sort_rows",
               "Number of rows sorted.",
               sort_rows
            ),
            Metric::sum(
               "mysql_perf_schema_events_statements_sum_sort_scan",
               "Number of sorts performed by statements which used a full table scan.",
               sort_scan
            ),
            Metric::sum(
               "mysql_perf_schema_events_statements_sum_timer_wait",
               "Total wait time of the summarized events that are timed.",
               timer_wait
            ),
            Metric::sum(
               "mysql_perf_schema_events_statements_sum_warnings",
               "Number of warnings.",
               warnings
            ),
        ]);
    }

    Ok(metrics)
}
