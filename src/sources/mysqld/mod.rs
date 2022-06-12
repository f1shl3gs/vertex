mod global_status;
mod global_variables;
mod info_schema_innodb_cmp;
mod info_schema_innodb_cmpmem;
mod info_schema_query_response_time;
mod slave_status;

#[cfg(all(test, feature = "integration-tests-mysql"))]
mod integration_tests;

use std::borrow::Cow;
use std::time::{Duration, Instant};

use event::{Metric, INSTANCE_KEY};
use framework::config::{
    default_false, default_interval, default_true, deserialize_duration, serialize_duration,
    ticker_from_duration, DataType, GenerateConfig, Output, SourceConfig, SourceContext,
    SourceDescription,
};
use framework::{tls::TlsConfig, Source};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use sqlx::mysql::{MySqlConnectOptions, MySqlSslMode};
use sqlx::{ConnectOptions, MySql, MySqlPool, Pool};

const VERSION_QUERY: &str = "SELECT @@version";

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("query execute failed, query: {}, err: {}", query, source))]
    Query {
        source: sqlx::Error,
        query: &'static str,
    },
    #[snafu(display("get server version failed, err: {}", source))]
    GetServerVersion { source: sqlx::Error },
    #[snafu(display("parse mysql version failed, version: {}", version))]
    ParseMysqlVersion { version: String },
    #[snafu(display("query slave status failed"))]
    QuerySlaveStatus,
    #[snafu(display("task join failed, err: {}", source))]
    TaskJoin { source: tokio::task::JoinError },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InfoSchemaConfig {
    // Since 5.5, Collect InnoDB compressed tables metrics from information_schema.innodb_cmp.
    #[serde(default = "default_true")]
    innodb_cmp: bool,
    // Since 5.5, Collect InnoDB buffer pool compression metrics from information_schema.innodb_cmpmem.
    #[serde(default = "default_true")]
    innodb_cmpmem: bool,
    // Since 5.5, Collect query response time distribution if query_response_time_stats is ON.
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

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MysqldConfig {
    // Since 5.1, Collect from SHOW GLOBAL STATUS (Enabled by default)
    #[serde(default = "default_global_status")]
    global_status: bool,
    // Since 5.1, Collect from SHOW GLOBAL VARIABLES (Enabled by default)
    #[serde(default = "default_global_variables")]
    global_variables: bool,
    // Since 5.1, Collect from SHOW SLAVE STATUS (Enabled by default)
    #[serde(default = "default_slave_status")]
    slave_status: bool,

    // Since 5.1, collect auto_increment columns and max values from information_schema.
    #[serde(default = "default_false")]
    auto_increment_columns: bool,
    // Since 5.1, collect the current size of all registered binlog files
    #[serde(default = "default_false")]
    binlog_size: bool,

    #[serde(default = "default_info_schema")]
    info_schema: InfoSchemaConfig,

    #[serde(default = "default_host")]
    host: String,
    #[serde(default = "default_port")]
    port: u16,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    password: Option<String>,
    ssl: Option<TlsConfig>,

    #[serde(default = "default_interval")]
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
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

        options.disable_statement_logging();

        options
    }
}

impl GenerateConfig for MysqldConfig {
    fn generate_config() -> String {
        format!(
            r#"
# IP address to Mysql server.
host: {}

#
port: {}

# The interval between scrapes.
#
# interval: 15s

# Username used to connect to MySQL instance
# username: user

# Password used to connect to MySQL instance
# password: some_password

# TLS options to connect to MySQL server.
# tls:
{}

##### Scrape Configuration #####

# Since 5.1, Collect from "SHOW GLOBAL STATUS"
global_status: {}

# Since 5.1, Collect from "SHOW GLOBAL VARIABLES"
global_variables: {}
"#,
            default_host(),
            default_port(),
            TlsConfig::generate_commented_with_indent(2),
            default_global_status(),
            default_global_variables(),
        )
    }
}

inventory::submit! {
    SourceDescription::new::<MysqldConfig>("mysqld")
}

#[async_trait::async_trait]
#[typetag::serde(name = "mysqld")]
impl SourceConfig for MysqldConfig {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let mut ticker = ticker_from_duration(self.interval).take_until(cx.shutdown);
        let options = self.connect_options();
        let mut output = cx.output;
        let instance = format!("{}:{}", self.host, self.port);

        Ok(Box::pin(async move {
            let pool = MySqlPool::connect_lazy_with(options);
            let instance = Cow::from(instance);

            while ticker.next().await.is_some() {
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

    fn source_type(&self) -> &'static str {
        "mysqld"
    }
}

pub async fn gather(instance: &str, pool: Pool<MySql>) -> Result<Vec<Metric>, Error> {
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
    let results = futures::future::try_join_all(tasks)
        .await
        .context(TaskJoinSnafu)?;

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

pub async fn get_mysql_version(pool: &MySqlPool) -> Result<f64, Error> {
    let version = sqlx::query_scalar::<_, String>(VERSION_QUERY)
        .fetch_one(pool)
        .await
        .context(QuerySnafu {
            query: VERSION_QUERY,
        })?;

    let nums = version.split('.').collect::<Vec<_>>();
    if nums.len() < 2 {
        return Err(Error::ParseMysqlVersion {
            version: version.clone(),
        });
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
