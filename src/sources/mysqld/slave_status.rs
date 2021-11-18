use std::collections::BTreeMap;
use chrono::NaiveDateTime;
use sqlx::{Column, FromRow, MySqlPool, Row};
use sqlx::mysql::MySqlRow;
use event::Metric;
use regex::Regex;

use crate::sources::mysqld::Error;

#[derive(Default, Debug)]
struct Record {
    master_uuid: String,
    master_host: String,
    channel_name: String,
    // MySQL & Percona
    connection_name: String, // MariaDB

    values: BTreeMap<String, f64>,
}

impl<'r> FromRow<'r, MySqlRow> for Record {
    fn from_row(row: &'r MySqlRow) -> Result<Self, sqlx::Error> {
        let mut record: Record = Default::default();
        for column in row.columns() {
            let name = column.name();
            let value = match row.try_get::<'r, String, _>(name) {
                Ok(value) => value,
                _ => continue
            };

            match name {
                "Master_UUID" => record.master_uuid = value,
                "Master_Host" => record.master_host = value,
                // MySQL & Percona
                "Channel_Name" => record.channel_name = value,
                // MariaDB
                "Connection_name" => record.connection_name = value,
                _ => {
                    match row.try_get::<'r, String, _>(name) {
                        Ok(value) => {
                            if let Some(fv) = parse_status(&value) {
                                record.values.insert(name.to_lowercase(), fv);
                            }
                        },
                        _ => {}
                    }
                }
            }
        }

        Ok(record)
    }
}

pub async fn gather(pool: &MySqlPool) -> Result<Vec<Metric>, Error> {
    let mut record = None;

    // Try the both syntax for MySQL/Percona and MariaDB
    'outer: for query in ["SHOW ALL SLAVES STATUS", "SHOW SLAVE STATUS"] {
        match sqlx::query_as::<_, Record>(query).fetch_one(pool).await {
            // MySQL/Percona
            Err(err) => {
                // Leverage lock-free SHOW SLAVE STATUS by guessing the right suffix
                for suffix in [" NONBLOCKING", " NOLOCK", ""] {
                    let query = format!("{}{}", query, suffix);

                    match sqlx::query_as::<_, Record>(&query).fetch_one(pool).await {
                        Ok(r) => {
                            record = Some(r);
                            break 'outer;
                        },
                        _ => continue
                    }
                }
            }
            // MariaDB
            Ok(r) => {
                record = Some(r);
            }
        }
    }

    if record.is_none() {
        return Err(Error::QuerySlaveStatusFailed)
    }

    println!("{:?}", record);

    Ok(vec![])
}

lazy_static! {
    static ref LOG_RE: Regex = Regex::new(r#".+\.(\d+)$"#).unwrap();
}

fn parse_status(val: &str) -> Option<f64> {
    let text = val.to_lowercase();
    match text.as_str() {
        "yes" | "on" => return Some(1.0),
        "no" | "off" | "disabled" => return Some(0.0),
        // SHOW SLAVE STATUS Slave_IO_Running can return "Connecting" which is a non-running state
        "connecting" => return Some(0.0),
        // SHOW GLOBAL STATUS like 'wsrep_cluster_status' can return "Primary" or "non-Primary"/"Disconnected"
        "primary" => return Some(1.0),
        "non-primary" | "disconnected" => return Some(0.0),
        _ => {}
    }

    // e.g. Jan 02 15:04:05 2006 MST
    if let Ok(date) = NaiveDateTime::parse_from_str(&text, "%b %d %X %Y %Z") {
        return Some(date.timestamp() as f64);
    }

    // e.g. 2006-01-02 15:04:05
    if let Ok(date) = NaiveDateTime::parse_from_str(&text, "%F %X") {
        return Some(date.timestamp() as f64);
    }

    // NOTE: cannot find anything about this
    if let Some(capture) = LOG_RE.captures(&text) {
        if capture.len() > 1 {
            let c = capture.get(0).unwrap();
            return c.as_str().parse().ok();
        }
    }

    text.parse().ok()
}

#[cfg(test)]
mod tests {
    use sqlx::mysql::{MySqlConnectOptions, MySqlSslMode};
    use crate::sources::mysqld::test_utils::setup_and_run;
    use super::*;

    #[tokio::test]
    async fn test_local_gather() {
        let opt = MySqlConnectOptions::default()
            .host("127.0.0.1")
            .port(9151)
            .username("root")
            .password("password")
            .ssl_mode(MySqlSslMode::Disabled);
        let pool = MySqlPool::connect_with(opt).await.unwrap();

        let results = gather(&pool).await.unwrap();
    }

    #[test]
    fn test_parse_status() {
        for (input, want) in [
            ("yes", Some(1.0)),
            ("on", Some(1.0)),
            ("no", Some(0.0)),
            ("off", Some(0.0)),
            ("Connecting", Some(0.0)),
            ("Primary", Some(1.0)),
            ("non-Primary", Some(0.0)),
            ("disconnected", Some(0.0)),
            ("2006-01-02 15:04:05", Some(1136214245.0)),
            ("Jan 02 15:04:05 2006 MST", Some(1136214245.0)),
            ("Mar 05 03:00:45 2009 CST", Some(1236193245.0 + 28800.0)),
            ("xfdaf.123", None),
            ("xfdaf.123f", None),
            ("Something_Unexpect", None),
            ("100", Some(100.0)),
            ("123.4", Some(123.4))
        ] {
            assert_eq!(
                parse_status(input),
                want,
                "input: {}, want: {:?}", input, want,
            );
        }
    }

    #[tokio::test]
    async fn test_gather() {
        async fn test(pool: MySqlPool) {
            let result = gather(&pool).await.unwrap();
            println!("{:#?}", result);
        }

        setup_and_run(test).await;
    }
}