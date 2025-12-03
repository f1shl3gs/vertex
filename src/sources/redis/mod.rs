mod config;
mod connection;
mod info;
mod latency;
mod sentinel;
mod slowlog;

use std::net::SocketAddr;
use std::time::Duration;
use std::time::Instant;

use chrono::Utc;
use configurable::{Configurable, configurable_component};
use connection::{Connection, Error as ClientError};
use event::Metric;
use framework::config::{OutputType, SourceConfig, SourceContext, default_interval};
use framework::{Pipeline, ShutdownSignal, Source};
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Invalid stats line of {0}")]
    InvalidStatsLine(&'static str),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Client(#[from] ClientError),
}

impl From<std::num::ParseIntError> for Error {
    fn from(err: std::num::ParseIntError) -> Self {
        Self::Parse(err.to_string())
    }
}

impl From<std::num::ParseFloatError> for Error {
    fn from(err: std::num::ParseFloatError) -> Self {
        Self::Parse(err.to_string())
    }
}

#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
struct Auth {
    /// Username to use for authentication
    #[serde(default)]
    username: Option<String>,

    /// Password of the Redis instance to scrape
    password: String,
}

#[configurable_component(source, name = "redis")]
#[serde(deny_unknown_fields)]
struct Config {
    /// Redis addresses to request for.
    #[configurable(required, format = "ip-address")]
    endpoints: Vec<SocketAddr>,

    /// Duration between each scrape.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    /// Authenticate the connection for Redis server, which is protected by
    /// `requirepass` option or ACL.
    #[serde(default)]
    auth: Option<Auth>,

    /// The assigned name is displayed in the output of CLIENT LIST so that it
    /// is possible to identify the client that performed a given connection.
    ///
    /// https://redis.io/docs/latest/commands/client-setname/
    #[serde(default)]
    client_name: Option<String>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "redis")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let endpoints = self.endpoints.clone();
        let auth = self.auth.clone();
        let client_name = self.client_name.clone();
        let interval = self.interval;

        let output = cx.output;
        let shutdown = cx.shutdown;

        Ok(Box::pin(async move {
            let mut tasks = JoinSet::from_iter(endpoints.into_iter().map(|endpoint| {
                run(
                    endpoint,
                    interval,
                    auth.clone(),
                    client_name.clone(),
                    output.clone(),
                    shutdown.clone(),
                )
            }));

            while tasks.join_next().await.is_some() {}

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::metric()]
    }
}

async fn run(
    endpoint: SocketAddr,
    interval: Duration,
    auth: Option<Auth>,
    client_name: Option<String>,
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let start = crate::common::calculate_start(&endpoint, interval);
    let mut ticker = tokio::time::interval_at(start.into(), interval);

    let mut errors = 0;
    let mut scraped = 0;
    let mut last_err = false;
    let target_value = endpoint.to_string();
    loop {
        tokio::select! {
            _ = ticker.tick() => {},
            _ = &mut shutdown => break,
        }

        scraped += 1;
        let start = Instant::now();
        let result = collect(endpoint, auth.as_ref(), client_name.as_ref()).await;
        let elapsed = start.elapsed();

        let mut metrics = match result {
            Ok(metrics) => metrics,
            Err(_err) => {
                last_err = true;
                errors += 1;

                Vec::with_capacity(4)
            }
        };

        metrics.extend([
            Metric::sum(
                "redis_scrape_errors_total",
                "Errors in requests to the exporter",
                errors,
            ),
            Metric::sum("redis_scrapes_total", "Total number of scrapes", scraped),
            Metric::gauge(
                "redis_last_scrape_duration_seconds",
                "Duration in seconds since last scraping request",
                elapsed,
            ),
            Metric::gauge(
                "redis_last_scrape_error",
                "The last scrape error status.",
                last_err,
            ),
        ]);

        let timestamp = Utc::now();
        metrics.iter_mut().for_each(|metric| {
            if !metric.name().starts_with("redis") {
                metric.set_name(format!("redis_{}", metric.name()));
            }

            metric.timestamp = Some(timestamp);
            metric.tags_mut().insert("target", &target_value);
        });

        if let Err(_err) = output.send(metrics).await {
            break;
        }
    }

    Ok(())
}

async fn collect(
    address: SocketAddr,
    auth: Option<&Auth>,
    client_name: Option<&String>,
) -> Result<Vec<Metric>, Error> {
    let mut conn = Connection::connect(&address).await?;

    if let Some(auth) = auth {
        match &auth.username {
            Some(username) => {
                conn.execute::<()>(&["auth", username, &auth.password])
                    .await?
            }
            None => conn.execute::<()>(&["auth", &auth.password]).await?,
        }
    }

    if let Some(name) = &client_name {
        conn.execute::<String>(&["client", "setname", name]).await?;
    }

    let mut metrics = config::collect(&mut conn).await?;

    if let Ok(partial) = info::collect(&mut conn).await {
        metrics.extend(partial);
    }

    // latency
    if let Ok(partial) = latency::collect(&mut conn).await {
        metrics.extend(partial);
    }

    if let Ok(partial) = slowlog::collect(&mut conn).await {
        metrics.extend(partial);
    }

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }
}

#[cfg(all(test, feature = "redis-integration-tests"))]
mod integration_tests {
    use testify::container::Container;
    use testify::next_addr;

    use super::*;
    use crate::testing::trace_init;

    const REDIS_PORT: u16 = 6379;

    async fn write_testdata(conn: &mut Connection) {
        for i in 0..100 {
            let key = format!("key_{i}");
            let value = format!("value_{i}");
            let resp = conn
                .execute::<String>(&["set", &key, &value])
                .await
                .unwrap();
            assert_eq!(resp, "OK")
        }
    }

    async fn run(password: Option<&str>, image: &str, tag: &str) {
        trace_init();

        let service_addr = next_addr();
        let mut container = Container::new(image, tag).with_tcp(REDIS_PORT, service_addr.port());
        if let Some(password) = password {
            container = container.args(["--requirepass", password]);
        }

        let metrics = container
            .run(async move {
                let mut conn = Connection::connect(&service_addr).await.unwrap();
                if let Some(password) = password {
                    conn.execute::<String>(&["auth", password]).await.unwrap();
                }
                write_testdata(&mut conn).await;

                let auth = password.map(|password| Auth {
                    username: None,
                    password: password.into(),
                });

                collect(service_addr, auth.as_ref(), None).await.unwrap()
            })
            .await;

        assert!(!metrics.is_empty());
    }

    #[tokio::test]
    async fn with_auth_v5() {
        run(Some("password"), "redis", "5.0-alpine").await;
    }

    #[tokio::test]
    async fn without_auth_v5() {
        run(None, "redis", "5.0-alpine").await;
    }

    #[tokio::test]
    async fn with_auth_v6() {
        run(Some("password"), "redis", "6.0-alpine").await;
    }

    #[tokio::test]
    async fn without_auth_v6() {
        run(None, "redis", "6.0-alpine").await;
    }

    #[tokio::test]
    async fn with_auth_v7() {
        run(Some("password"), "redis", "7.0-alpine").await;
    }

    #[tokio::test]
    async fn without_auth_v7() {
        run(None, "redis", "7.0-alpine").await;
    }
}
