use event::Metric;

use super::{Connection, Error};

const TOKUDB_STATUS_QUERY: &str = "SHOW ENGINE TOKUDB STATUS";

pub async fn collect(conn: &mut Connection) -> Result<Vec<Metric>, Error> {
    let mut rows = conn.query(TOKUDB_STATUS_QUERY).await?;

    let mut metrics = vec![];
    while let Some(mut row) = rows.next().await? {
        let _temp = row.get_str();
        let key = row.get_str();
        let value = row.get_str();

        if let Ok(value) = value.parse::<f64>() {
            metrics.push(Metric::gauge(
                format!("mysql_engine_tokudb_{}", sanitize(key)),
                "Generic metric from SHOW ENGINE TOKUDB STATUS.",
                value,
            ));
        }
    }

    Ok(metrics)
}

fn sanitize(key: &str) -> String {
    let mut output = String::with_capacity(key.len());

    for ch in key.chars() {
        if ['>', ',', ':', '(', ')'].contains(&ch) {
            continue;
        }

        if [' ', '-'].contains(&ch) {
            output.push('_');
            continue;
        }

        if ['+', '/'].contains(&ch) {
            output.push_str("and");
            continue;
        }

        output.push(ch);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::mysql::assert_contains;
    use crate::sources::mysql::connection::mock;
    use event::tags;

    #[test]
    fn sanitize_metric() {
        for (input, want) in [
            (
                "loader: number of calls to loader->close() that failed",
                "loader_number_of_calls_to_loader_close_that_failed",
            ),
            (
                "ft: promotion: stopped anyway, after locking the child",
                "ft_promotion_stopped_anyway_after_locking_the_child",
            ),
            (
                "ft: basement nodes deserialized with fixed-keysize",
                "ft_basement_nodes_deserialized_with_fixed_keysize",
            ),
            (
                "memory: number of bytes used (requested + overhead)",
                "memory_number_of_bytes_used_requested_and_overhead",
            ),
            (
                "ft: uncompressed / compressed bytes written (overall)",
                "ft_uncompressed_and_compressed_bytes_written_overall",
            ),
        ] {
            let got = sanitize(input);
            assert_eq!(got, want);
        }
    }

    #[tokio::test]
    async fn smoke() {
        let mut conn = mock(|_| {
            (
                vec!["Type", "Name", "Status"],
                vec![
                    vec![
                        "TokuDB",
                        "indexer: number of calls to indexer->build() succeeded",
                        "1",
                    ],
                    vec![
                        "TokuDB",
                        "ft: promotion: stopped anyway, after locking the child",
                        "45316247",
                    ],
                    vec![
                        "TokuDB",
                        "memory: mallocator version",
                        "3.3.1-0-g9ef9d9e8c271cdf14f664b871a8f98c827714784",
                    ],
                    vec![
                        "TokuDB",
                        "filesystem: most recent disk full",
                        "Thu Jan  1 00:00:00 1970",
                    ],
                    vec![
                        "TokuDB",
                        "locktree: time spent ending the STO early (seconds)",
                        "9115.904484",
                    ],
                ],
            )
        })
        .await;

        let metrics = collect(&mut conn).await.unwrap();
        assert_contains(
            &metrics,
            vec![
                (tags!(), 1.0),
                (tags!(), 45316247.0),
                (tags!(), 9115.904484),
            ],
            vec![],
        );
    }
}
