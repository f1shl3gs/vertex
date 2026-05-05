// `information_schema.query_response_time`

use event::{Bucket, Metric};

use super::{Connection, Error};

const RESPONSE_CHECK_QUERY: &str = "SELECT @@query_response_time_stats";

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut rows = match conn.query(RESPONSE_CHECK_QUERY).await {
        Ok(rows) => rows,
        Err(err) => {
            if matches!(err, Error::Server { code, .. } if code == 1193) {
                debug!(
                    message = "Query response time distribution is not available",
                    %err
                );

                return Ok(vec![]);
            }

            return Err(err);
        }
    };

    let Some(mut row) = rows.next().await? else {
        // draining all incoming packets (include eof)
        while rows.next().await?.is_some() {}

        debug!(message = "Empty response", query = RESPONSE_CHECK_QUERY);
        return Ok(vec![]);
    };

    let query_stats = row.get_str().parse::<u8>()?;
    // draining all incoming packets (include eof)
    while rows.next().await?.is_some() {}

    if query_stats == 0 {
        debug!(
            message = "MySQL variable 'query_response_time_stats' is OFF",
            variable = "query_response_time_stats"
        );
        return Ok(vec![]);
    }

    let mut metrics = vec![];
    for (index, (query, name, desc)) in [
        (
            "SELECT TIME, COUNT, TOTAL FROM INFORMATION_SCHEMA.QUERY_RESPONSE_TIME",
            "mysql_info_schema_query_response_time_seconds",
            "The number of all queries by duration they took to execute.",
        ),
        (
            "SELECT TIME, COUNT, TOTAL FROM INFORMATION_SCHEMA.QUERY_RESPONSE_TIME_READ",
            "mysql_info_schema_read_query_response_time_seconds",
            "The number of read queries by duration they took to execute.",
        ),
        (
            "SELECT TIME, COUNT, TOTAL FROM INFORMATION_SCHEMA.QUERY_RESPONSE_TIME_WRITE",
            "mysql_info_schema_write_query_response_time_seconds",
            "The number of write queries by duration they took to execute.",
        ),
    ]
    .iter()
    .enumerate()
    {
        match process_table(conn, query).await {
            Ok((sum, total, buckets)) => {
                metrics.push(Metric::histogram(*name, *desc, sum, total, buckets));
            }
            Err(err) => {
                if index == 0 {
                    return Err(err);
                }

                debug!(message = "Query failed", %err);
            }
        }
    }

    Ok(metrics)
}

async fn process_table(
    conn: &mut Connection,
    query: &str,
) -> Result<(u64, f64, Vec<Bucket>), Error> {
    let mut rows = conn.query(query).await?;

    let mut histogram_count = 0;
    let mut histogram_sum = 0.0;
    let mut buckets: Vec<Bucket> = Vec::new();

    while let Some(mut row) = rows.next().await? {
        let length = row.get_str().parse::<f64>().unwrap_or_default();
        let count = row.get_str().parse::<u64>()?;
        let total = row.get_str().parse::<f64>().unwrap_or_default();

        histogram_count += count;
        histogram_sum += total;

        // Special case for "TOO LONG" row where we take into account the count
        // field which is the only available and do not add it as a part of
        // histogram or metric
        if length == 0.0 {
            continue;
        }

        match buckets.iter_mut().find(|bucket| bucket.upper == length) {
            Some(bucket) => bucket.count = histogram_count,
            None => buckets.push(Bucket {
                upper: length,
                count: histogram_count,
            }),
        }
    }

    buckets.sort_by(|a, b| a.upper.partial_cmp(&b.upper).unwrap());

    Ok((histogram_count, histogram_sum, buckets))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::mysql::connection::mock;
    use event::MetricValue;

    #[tokio::test]
    async fn smoke() {
        let mut conn = mock(|query| {
            if query == RESPONSE_CHECK_QUERY {
                (vec![""], vec![vec!["1"]])
            } else {
                (
                    vec!["TIME", "COUNT", "TOTAL"],
                    vec![
                        vec!["0.000001", "124", "0.000000"],
                        vec!["0.000010", "179", "0.000797"],
                        vec!["0.000100", "2859", "0.107321"],
                        vec!["0.001000", "1085", "0.335395"],
                        vec!["0.010000", "269", "0.522264"],
                        vec!["0.100000", "11", "0.344209"],
                        vec!["1.000000", "1", "0.267369"],
                        vec!["10.000000", "0", "0.000000"],
                        vec!["100.000000", "0", "0.000000"],
                        vec!["1000.000000", "0", "0.000000"],
                        vec!["10000.000000", "0", "0.000000"],
                        vec!["100000.000000", "0", "0.000000"],
                        vec!["1000000.000000", "0", "0.000000"],
                        vec!["TOO LONG", "0", "TOO LONG"],
                    ],
                )
            }
        })
        .await;

        let metrics = collect(&mut conn).await.unwrap();
        for metric in &metrics {
            assert_eq!(
                metric.value,
                MetricValue::Histogram {
                    count: 4528,
                    sum: 1.5773549999999998,
                    buckets: vec![
                        Bucket {
                            upper: 1e-06,
                            count: 124
                        },
                        Bucket {
                            upper: 1e-05,
                            count: 303
                        },
                        Bucket {
                            upper: 0.0001,
                            count: 3162
                        },
                        Bucket {
                            upper: 0.001,
                            count: 4247
                        },
                        Bucket {
                            upper: 0.01,
                            count: 4516
                        },
                        Bucket {
                            upper: 0.1,
                            count: 4527
                        },
                        Bucket {
                            upper: 1.0,
                            count: 4528
                        },
                        Bucket {
                            upper: 10.0,
                            count: 4528
                        },
                        Bucket {
                            upper: 100.0,
                            count: 4528
                        },
                        Bucket {
                            upper: 1000.0,
                            count: 4528
                        },
                        Bucket {
                            upper: 10000.0,
                            count: 4528
                        },
                        Bucket {
                            upper: 100000.0,
                            count: 4528
                        },
                        Bucket {
                            upper: 1e+06,
                            count: 4528
                        },
                    ],
                }
            )
        }
    }
}
