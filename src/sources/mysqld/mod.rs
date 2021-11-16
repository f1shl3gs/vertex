mod global_status;
mod global_variables;
mod slave_status;
mod info_schema_innodb_cmp;
mod info_schema_innodb_cmpmem;
mod info_schema_query_response_time;
#[cfg(test)]
mod tests;

use chrono::Utc;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use sqlx::mysql::{MySqlConnectOptions, MySqlSslMode};
use sqlx::MySqlPool;
use snafu::Snafu;
use event::{Event, Metric, tags};

use crate::sources::Source;
use crate::tls::TlsConfig;
use crate::config::{
    GenerateConfig, SourceDescription, default_false, default_true,
    SourceConfig, SourceContext, DataType, ticker_from_duration, default_interval,
    deserialize_duration, serialize_duration,
};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("query execute failed, query: {}, err: {}", query, source))]
    QueryFailed { source: sqlx::Error, query: &'static str }
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

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MysqldConfig {
    // Since 5.1, Collect from SHOW GLOBAL STATUS (Enabled by default)
    #[serde(default = "default_true")]
    global_status: bool,
    // Since 5.1, Collect from SHOW GLOBAL VARIABLES (Enabled by default)
    #[serde(default = "default_true")]
    global_variables: bool,
    // Since 5.1, Collect from SHOW SLAVE STATUS (Enabled by default)
    #[serde(default = "default_true")]
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
    #[serde(deserialize_with = "deserialize_duration", serialize_with = "serialize_duration")]
    interval: chrono::Duration,
}

fn default_host() -> String {
    "localhost".to_string()
}

fn default_port() -> u16 {
    3306
}

fn default_info_schema() -> InfoSchemaConfig {
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

        options
    }
}

impl GenerateConfig for MysqldConfig {
    fn generate_config() -> Value {
        serde_yaml::to_value(
            Self {
                global_status: default_true(),
                global_variables: default_true(),
                slave_status: default_true(),
                auto_increment_columns: default_false(),
                binlog_size: default_false(),
                info_schema: InfoSchemaConfig {
                    innodb_cmp: default_true(),
                    innodb_cmpmem: default_true(),
                    query_response_time: default_true(),
                },
                host: default_host(),
                port: default_port(),
                username: Some("foo".to_string()),
                password: Some("some_password".to_string()),
                ssl: None,
                interval: default_interval(),
            }
        ).unwrap()
    }
}

inventory::submit! {
    SourceDescription::new::<MysqldConfig>("mysqld")
}

#[async_trait::async_trait]
#[typetag::serde(name = "mysqld")]
impl SourceConfig for MysqldConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        let mut ticker = ticker_from_duration(self.interval).unwrap()
            .take_until(ctx.shutdown);
        let options = self.connect_options();
        let mut output = ctx.out;
        let instance = format!("{}:{}", self.host, self.port);

        Ok(Box::pin(async move {
            let pool = MySqlPool::connect_lazy_with(options);
            let instance = instance.as_str();

            while ticker.next().await.is_some() {
                let mut tasks = vec![];

                let p = pool.clone();
                tasks.push(tokio::spawn(async move {
                    global_status::gather(&p).await
                }));

                let p = pool.clone();
                tasks.push(tokio::spawn(async move {
                    global_variables::gather(&p).await
                }));

                let p = pool.clone();
                tasks.push(tokio::spawn(async move {
                    info_schema_innodb_cmp::gather(&p).await
                }));

                // When `try_join_all` works with `JoinHandle`, the behavior does not match
                // the docs. See: https://github.com/rust-lang/futures-rs/issues/2167
                let metrics = match futures::future::try_join_all(tasks).await {
                    Err(err) => {
                        warn!(
                            message = "Staring scrape tasks failed",
                            %err
                        );

                        vec![
                            Metric::gauge_with_tags(
                                "mysql_up",
                                "Whether the MySQL server is up.",
                                0,
                                tags!(
                                    "instance" => instance
                                ),
                            )
                        ]
                    },
                    Ok(results) => {
                        let merged = results.iter()
                            .fold(Ok(vec![]), | acc, part| {
                                match (acc, part) {
                                    (Ok(mut acc), Ok(part)) => {
                                        acc.extend_from_slice(part);
                                        Ok(acc)
                                    },
                                    (Ok(_), Err(err)) => Err(err),
                                    (Err(err), _) => Err(err),
                                }
                            });

                        match merged {
                            Ok(mut metrics) => {
                                metrics.push(Metric::gauge_with_tags(
                                    "mysql_up",
                                    "Whether the MySQL server is up.",
                                    1,
                                    tags!(
                                        "instance" => instance
                                    ),
                                ));

                                metrics
                            },
                            Err(err) => {
                                warn!(
                                    message = "Scrape metrics failed",
                                    %err
                                );

                                vec![
                                    Metric::gauge_with_tags(
                                        "mysql_up",
                                        "Whether the MySQL server is up.",
                                        0,
                                        tags!(
                                            "instance" => instance
                                        ),
                                    )
                                ]
                            }
                        }
                    }
                };

                let now = Utc::now();
                let mut stream = futures::stream::iter(metrics)
                    .map(|mut m| {
                        m.timestamp = Some(now);
                        Event::Metric(m)
                    })
                    .map(Ok);

                output.send_all(&mut stream).await;
            }

            Ok(())
        }))
    }

    fn output_type(&self) -> DataType {
        DataType::Metric
    }

    fn source_type(&self) -> &'static str {
        "mysqld"
    }
}

pub fn valid_name(s: &str) -> String {
    s.chars()
        .map(|x| {
            if x.is_alphanumeric() {
                x
            } else {
                '_'
            }
        })
        .collect::<String>()
        .to_lowercase()
}
