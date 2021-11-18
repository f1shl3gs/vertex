use sqlx::MySqlPool;
use event::Metric;

use crate::sources::mysqld::Error;


pub async fn gather(pool: &MySqlPool) -> Result<Vec<Metric>, Error> {
    // Try the both syntax for MySQL/Percona and MariaDB
    for query in ["SHOW ALL SLAVES STATUS", "SHOW SLAVE STATUS"] {
        match sqlx::query(query).execute(pool).await {
            // MySQL/Percona
            Err(err) => {
                // Leverage lock-free SHOW SLAVE STATUS by guessing the right suffix
                for suffix in [" NONBLOCKING", " NOLOCK", ""] {

                }

                warn!(
                    message = "query failed",
                    query,
                    %err,
                );

                println!("{}, {:?}", query, err);
            },
            // MariaDB
            _ => break
        }
    }

    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use crate::sources::mysqld::test_utils::setup_and_run;
    use super::*;

    #[tokio::test]
    async fn test_gather() {
        async fn test(pool: MySqlPool) {
            let result = gather(&pool).await.unwrap();
            println!("{:#?}", result);
        }

        setup_and_run(test).await;
    }
}