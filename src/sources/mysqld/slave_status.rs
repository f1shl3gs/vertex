use std::collections::BTreeMap;

use chrono::NaiveDateTime;
use event::{tags, Metric};
use once_cell::sync::Lazy;
use regex::Regex;
use sqlx::mysql::MySqlRow;
use sqlx::{Column, FromRow, MySqlPool, Row, ValueRef};

use crate::sources::mysqld::MysqlError;

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

        for (index, column) in row.columns().iter().enumerate() {
            let name = column.name();

            match name {
                "Master_UUID" => record.master_uuid = row.try_get::<'r, String, _>(index)?,
                "Master_Host" => record.master_host = row.try_get::<'r, String, _>(index)?,
                // MySQL & Percona
                "Channel_Name" => record.channel_name = row.try_get::<'r, String, _>(index)?,
                // MariaDB
                "Connection_name" => {
                    record.connection_name = row.try_get::<'r, String, _>(index)?
                }
                _ => {
                    // TODO: this implement is ugly, it could be simple and clear if we can
                    //   access the value of `raw`'s value([u8]), but it is defined with "pub(crate)"
                    //      /// Implementation of [`ValueRef`] for MySQL.
                    //      #[derive(Clone)]
                    //      pub struct MySqlValueRef<'r> {
                    //          pub(crate) value: Option<&'r [u8]>,
                    //          pub(crate) row: Option<&'r Bytes>,
                    //          pub(crate) type_info: MySqlTypeInfo,
                    //          pub(crate) format: MySqlValueFormat,
                    //      }

                    let raw = row.try_get_raw(index)?;
                    if raw.is_null() {
                        continue;
                    }

                    if let Ok(text) = row.try_get::<'r, &str, _>(index) {
                        if let Some(fv) = parse_status(text) {
                            record.values.insert(name.to_lowercase(), fv);
                        }

                        continue;
                    }

                    if let Ok(v) = row.try_get::<'r, u32, _>(index) {
                        record.values.insert(name.to_lowercase(), v as f64);
                        continue;
                    }

                    debug!(message = "unknown column type from slave status", name,);
                }
            }
        }

        Ok(record)
    }
}

pub async fn gather(pool: &MySqlPool) -> Result<Vec<Metric>, MysqlError> {
    let mut record = None;

    // Try the both syntax for MySQL/Percona and MariaDB
    'outer: for query in ["SHOW ALL SLAVES STATUS", "SHOW SLAVE STATUS"] {
        match sqlx::query_as::<_, Record>(query).fetch_one(pool).await {
            // MySQL/Percona
            Err(_err) => {
                // Leverage lock-free SHOW SLAVE STATUS by guessing the right suffix
                for suffix in [" NONBLOCKING", " NOLOCK", ""] {
                    let query = format!("{}{}", query, suffix);

                    match sqlx::query_as::<_, Record>(&query).fetch_one(pool).await {
                        Ok(r) => {
                            record = Some(r);
                            break 'outer;
                        }
                        _ => continue,
                    }
                }
            }
            // MariaDB
            Ok(r) => {
                record = Some(r);
            }
        }
    }

    match record {
        Some(record) => {
            let mut metrics = Vec::with_capacity(record.values.len());

            for (k, v) in record.values {
                metrics.push(Metric::gauge_with_tags(
                    format!("mysql_slave_status_{}", k),
                    "Generic metric from SHOW SLAVE STATUS",
                    v,
                    tags!(
                        "master_host" => &record.master_host,
                        "master_uuid" => &record.master_uuid,
                        "channel_name" => &record.channel_name,
                        "connection_name" => &record.connection_name
                    ),
                ));
            }

            Ok(metrics)
        }

        // Replication is not enabled
        None => Ok(vec![]),
    }
}

static LOG_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r".+\.(\d+)$").unwrap());

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
    use super::*;

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
            ("123.4", Some(123.4)),
        ] {
            assert_eq!(
                parse_status(input),
                want,
                "input: {}, want: {:?}",
                input,
                want,
            );
        }
    }
}
