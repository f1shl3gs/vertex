mod global_status;
mod global_variables;
mod info_schema_innodb_cmp;
mod info_schema_innodb_cmpmem;
mod info_schema_query_response_time;
#[cfg(all(test, feature = "integration-tests-mysql"))]
mod integration_tests;
mod slave_status;

use std::borrow::Cow;
use std::time::{Duration, Instant};

use configurable::{configurable_component, Configurable};
use event::{Metric, INSTANCE_KEY};
use framework::config::{
    default_interval, default_true, DataType, Output, SourceConfig, SourceContext,
};
use framework::{tls::TlsConfig, Source};
use serde::{Deserialize, Serialize};
use sqlx::mysql::{MySqlConnectOptions, MySqlSslMode};
use sqlx::{ConnectOptions, MySql, MySqlPool, Pool};
use thiserror::Error;

const VERSION_QUERY: &str = "SELECT @@version";

#[derive(Debug, Error)]
pub enum MysqlError {
    #[error("query execute failed, query: {query}, err: {err}")]
    Query {
        err: sqlx::Error,
        query: &'static str,
    },
    #[error("parse mysql version failed, version: {0}")]
    ParseMysqlVersion(String),
    #[error("query slave status failed")]
    QuerySlaveStatus,
    #[error("task join failed, err: {0}")]
    TaskJoin(#[from] tokio::task::JoinError),
}

#[derive(Configurable, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InfoSchemaConfig {
    /// Since 5.5, Collect InnoDB compressed tables metrics from information_schema.innodb_cmp.
    #[serde(default = "default_true")]
    innodb_cmp: bool,
    /// Since 5.5, Collect InnoDB buffer pool compression metrics from information_schema.innodb_cmpmem.
    #[serde(default = "default_true")]
    innodb_cmpmem: bool,
    /// Since 5.5, Collect query response time distribution if query_response_time_stats is ON.
    #[serde(default = "default_true")]
    query_response_time: bool,
}

const fn default_global_status() -> bool {
    true
}

const fn default_global_variables() -> bool {
    true
}

const fn default_slave_status() -> bool {
    true
}

#[configurable_component(source, name = "mysqld")]
#[serde(deny_unknown_fields)]
struct MysqldConfig {
    /// Since 5.1, Collect from SHOW GLOBAL STATUS (Enabled by default)
    #[serde(default = "default_global_status")]
    global_status: bool,
    /// Since 5.1, Collect from SHOW GLOBAL VARIABLES (Enabled by default)
    #[serde(default = "default_global_variables")]
    global_variables: bool,
    /// Since 5.1, Collect from SHOW SLAVE STATUS (Enabled by default)
    #[serde(default = "default_slave_status")]
    slave_status: bool,

    /// Since 5.1, collect auto_increment columns and max values from information_schema.
    #[serde(default)]
    auto_increment_columns: bool,

    /// Since 5.1, collect the current size of all registered binlog files
    #[serde(default)]
    binlog_size: bool,

    #[serde(default = "default_info_schema")]
    info_schema: InfoSchemaConfig,

    /// IP address to MySQL server.
    #[serde(default = "default_host")]
    host: String,

    /// TCP port to MySQL server
    #[serde(default = "default_port")]
    port: u16,

    /// Username used to connect to MySQL instance
    #[serde(default)]
    username: Option<String>,

    /// Password used to connect to MySQL instance
    #[serde(default)]
    password: Option<String>,
    ssl: Option<TlsConfig>,

    /// Duration between each scrape.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,
}

fn default_host() -> String {
    "localhost".to_string()
}

const fn default_port() -> u16 {
    3306
}

const fn default_info_schema() -> InfoSchemaConfig {
    InfoSchemaConfig {
        innodb_cmp: true,
        innodb_cmpmem: true,
        query_response_time: true,
    }
}

impl MysqldConfig {
    fn connect_options(&self) -> MySqlConnectOptions {
        // TODO support ssl
        let mut options = MySqlConnectOptions::new()
            .host(self.host.as_str())
            .port(self.port)
            .ssl_mode(MySqlSslMode::Disabled);

        if let Some(username) = &self.username {
            options = options.username(username);
        }

        if let Some(password) = &self.password {
            options = options.password(password);
        }

        options.disable_statement_logging()
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "mysqld")]
impl SourceConfig for MysqldConfig {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let mut ticker = tokio::time::interval(self.interval);
        let options = self.connect_options();
        let instance = format!("{}:{}", self.host, self.port);
        let SourceContext {
            mut output,
            mut shutdown,
            ..
        } = cx;

        Ok(Box::pin(async move {
            let pool = MySqlPool::connect_lazy_with(options);
            let instance = Cow::from(instance);

            loop {
                tokio::select! {
                    biased;

                    _ = &mut shutdown => break,
                    _ = ticker.tick() => {}
                }

                let start = Instant::now();
                let result = gather(instance.as_ref(), pool.clone()).await;
                let elapsed = start.elapsed().as_secs_f64();
                let up = result.is_ok();

                let mut metrics = result.unwrap_or_default();
                metrics.extend_from_slice(&[
                    Metric::gauge("mysql_up", "Whether the MySQL server is up.", up),
                    Metric::gauge("mysql_scrape_duration_seconds", "", elapsed),
                ]);

                let now = chrono::Utc::now();
                metrics.iter_mut().for_each(|m| {
                    m.timestamp = Some(now);
                    m.insert_tag(INSTANCE_KEY, instance.clone());
                });

                if let Err(err) = output.send(metrics).await {
                    error!(
                        message = "Error sending mysqld metrics",
                        %err
                    );

                    return Err(());
                }
            }

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }
}

pub async fn gather(instance: &str, pool: Pool<MySql>) -> Result<Vec<Metric>, MysqlError> {
    let version = match get_mysql_version(&pool).await {
        Ok(v) => v,
        Err(err) => {
            warn!(
                message = "Get mysql version failed",
                instance,
                %err
            );

            return Err(err);
        }
    };

    let mut tasks = vec![];

    if version >= 5.1 {
        let p = pool.clone();
        tasks.push(tokio::spawn(async move { global_status::gather(&p).await }));
    }

    if version >= 5.1 {
        let p = pool.clone();
        tasks.push(tokio::spawn(
            async move { global_variables::gather(&p).await },
        ));
    }

    if version >= 5.5 {
        let p = pool.clone();
        tasks.push(tokio::spawn(async move {
            info_schema_innodb_cmp::gather(&p).await
        }));
    }

    if version >= 5.5 {
        let p = pool.clone();
        tasks.push(tokio::spawn(async move {
            info_schema_innodb_cmpmem::gather(&p).await
        }));
    }

    if version >= 5.5 {
        let p = pool.clone();
        tasks.push(tokio::spawn(async move {
            info_schema_query_response_time::gather(&p).await
        }));
    }

    if version >= 5.1 {
        let p = pool.clone();
        tasks.push(tokio::spawn(async move { slave_status::gather(&p).await }));
    }

    // When `try_join_all` works with `JoinHandle`, the behavior does not match
    // the docs. See: https://github.com/rust-lang/futures-rs/issues/2167
    let results = futures::future::try_join_all(tasks).await?;

    // NOTE:
    // `results.into_iter().collect()` would be awesome, BUT
    // the trait `FromIterator<Vec<event::Metric>>` is not implemented for `Vec<event::Metric>`

    let mut metrics = vec![];
    for partial in results {
        match partial {
            Ok(partial) => metrics.extend(partial),
            Err(err) => return Err(err),
        }
    }

    Ok(metrics)
}

pub fn valid_name(s: &str) -> String {
    s.chars()
        .map(|x| if x.is_alphanumeric() { x } else { '_' })
        .collect::<String>()
        .to_lowercase()
}

pub async fn get_mysql_version(pool: &MySqlPool) -> Result<f64, MysqlError> {
    let version = sqlx::query_scalar::<_, String>(VERSION_QUERY)
        .fetch_one(pool)
        .await
        .map_err(|err| MysqlError::Query {
            query: VERSION_QUERY,
            err,
        })?;

    let nums = version.split('.').collect::<Vec<_>>();
    if nums.len() < 2 {
        return Err(MysqlError::ParseMysqlVersion(version));
    }

    let version = match (nums[0].parse::<f64>(), nums[1].parse::<f64>()) {
        (Ok(major), Ok(mut minor)) => {
            loop {
                minor /= 10.0;
                if minor < 1.0 {
                    break;
                }
            }

            major + minor
        }
        _ => {
            // If we can't match/parse the version, set it some big value that matches all versions.
            1000.0
        }
    };

    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<MysqldConfig>()
    }
}
