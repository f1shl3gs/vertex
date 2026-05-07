mod auto_increment;
mod clientstats;
mod innodb_cmp;
mod innodb_cmpmem;
mod innodb_metrics;
mod innodb_sys_tablespaces;
mod process_list;
mod query_response_time;
mod replica_host;
mod rocksdb_perf_context;
mod schemastats;
mod tables;
mod tablestats;
mod userstats;

use configurable::Configurable;
use event::Metric;
use framework::config::default_true;
use serde::{Deserialize, Serialize};

use super::connection::{Connection, Error, Flavor};
use super::sanitize;

#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Since 5.5, Collect InnoDB compressed tables metrics from information_schema.innodb_cmp.
    #[serde(default = "default_true")]
    innodb_cmp: bool,

    /// Since 5.5, Collect InnoDB buffer pool compression metrics from information_schema.innodb_cmpmem.
    #[serde(default = "default_true")]
    innodb_cmpmem: bool,

    /// Since 5.5, Collect metrics from information_schema.innodb_metrics
    #[serde(default)]
    innodb_metrics: bool,

    /// Since 5.7, Collect metrics from information_schema.innodb_sys_tablespaces
    #[serde(default)]
    innodb_sys_tablespaces: bool,

    /// Since 5.5, Collect query response time distribution if query_response_time_stats is ON.
    #[serde(default = "default_true")]
    query_response_time: bool,

    /// Since 5.5, If running with userstat=1, set to true to collect client statistics
    #[serde(default)]
    clientstats: bool,

    /// Since 5.1, Collect auto_increment columns and max values from information_schema
    #[serde(default)]
    auto_increment: bool,

    /// Collect current thread state counts from the information_schema.processlist
    #[serde(default)]
    process_list: Option<process_list::Config>,

    /// Since 5.6,  Collect metrics from information_schema.replica_host_status
    #[serde(default)]
    replica_host: bool,

    /// Since 5.6, Collect metrics from information_schema.ROCKSDB_PERF_CONTEXT
    #[serde(default)]
    rocksdb_perf_context: bool,

    /// Since 5.1, If running with userstat=1, set to true to collect schema statistics
    #[serde(default)]
    schemastats: bool,

    /// Since 5.1, Collect metrics from information_schema.tables
    #[serde(default)]
    tables: Vec<String>,

    /// Since 5.1, If running with userstat=1, set to true to collect table statistics
    #[serde(default)]
    tablestats: bool,

    /// Since 5.1, If running with userstat=1, set to true to collect user statistics
    #[serde(default)]
    userstats: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            innodb_cmp: true,
            innodb_cmpmem: true,
            query_response_time: true,
            innodb_metrics: false,
            innodb_sys_tablespaces: false,
            clientstats: false,
            auto_increment: false,
            process_list: None,
            replica_host: false,
            rocksdb_perf_context: false,
            schemastats: false,
            tables: vec![],
            tablestats: false,
            userstats: false,
        }
    }
}

const USERSTAT_CHECK_QUERY: &str =
    "SHOW GLOBAL VARIABLES WHERE Variable_Name='userstat' OR Variable_Name='userstat_running'";

async fn check_userstat(conn: &mut Connection) -> Result<bool, Error> {
    let mut rows = conn.query(USERSTAT_CHECK_QUERY).await?;

    let mut status = false;
    while let Some(mut row) = rows.next().await? {
        if status {
            // make sure this while will drain all incoming rows
            continue;
        }

        let name = row.get_str();
        let value = row.get_str();
        status = ["userstat", "userstat_running"].contains(&name) && value == "ON";
    }

    Ok(status)
}

pub async fn collect(conn: &mut Connection, conf: &Config) -> Result<Vec<Metric>, Error> {
    let version = conn.version();
    let mut metrics = Vec::new();

    if check_userstat(conn).await? {
        if conf.clientstats && version >= 5.5 {
            metrics.extend(clientstats::collect(conn).await?);
        }

        if conf.schemastats && version >= 5.1 {
            metrics.extend(schemastats::collect(conn).await?);
        }

        if conf.tablestats && version >= 5.1 {
            metrics.extend(tablestats::collect(conn).await?);
        }

        if conf.userstats && version >= 5.1 {
            metrics.extend(userstats::collect(conn).await?);
        }
    }

    if conf.auto_increment && version >= 5.1 {
        metrics.extend(auto_increment::collect(conn).await?);
    }

    if conf.innodb_cmp && version >= 5.5 {
        metrics.extend(innodb_cmp::collect(conn).await?);
    }

    if conf.innodb_cmpmem && version >= 5.5 {
        metrics.extend(innodb_cmpmem::collect(conn).await?);
    }

    if conf.innodb_metrics && version >= 5.5 {
        metrics.extend(innodb_metrics::collect(conn).await?);
    }

    if conf.innodb_sys_tablespaces && version >= 5.7 {
        metrics.extend(innodb_sys_tablespaces::collect(conn).await?);
    }

    if let Some(conf) = &conf.process_list
        && version >= 5.1
    {
        metrics.extend(process_list::collect(conn, conf).await?);
    }

    if conf.query_response_time && version >= 5.5 {
        metrics.extend(query_response_time::collect(conn).await?);
    }

    if conf.replica_host && version >= 5.6 {
        metrics.extend(replica_host::collect(conn).await?);
    }

    if conf.rocksdb_perf_context && version >= 5.6 {
        metrics.extend(rocksdb_perf_context::collect(conn).await?);
    }

    if !conf.tables.is_empty() && version >= 5.1 {
        metrics.extend(tables::collect(conn, &conf.tables).await?);
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::mysql::connection::mock;

    #[tokio::test]
    async fn userstats() {
        for (key, value, want) in [
            ("userstat", "ON", true),
            ("userstat", "OFF", false),
            ("xxxx", "ON", false),
        ] {
            let mut conn = mock(|_| (vec!["Variable_name", "Value"], vec![vec![key, value]])).await;

            let got = check_userstat(&mut conn).await.unwrap();
            assert_eq!(got, want);
        }
    }
}
