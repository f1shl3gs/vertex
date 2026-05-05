mod binlog_size;
mod connection;
mod engine;
mod global;
mod heartbeat;
mod information;
mod performance;
mod slave;
mod sys_user_summary;
mod user;

use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Add;
use std::time::{Duration, Instant};

use configurable::{Configurable, configurable_component};
use event::Metric;
use framework::config::{OutputType, SourceConfig, SourceContext, default_interval};
use framework::{Pipeline, ShutdownSignal, Source};
use futures::stream::{FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use connection::Connection;

#[derive(Debug, Error)]
pub enum Error {
    #[error("no data")]
    NoData,

    #[error(transparent)]
    Mysql(#[from] connection::Error),

    #[error("resolve failed")]
    Resolve,

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("invalid row data")]
    InvalidData,
}

fn default_user() -> String {
    "root".to_string()
}

#[derive(Configurable, Clone, Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum AuthConfig {
    NativePassword {
        /// Username used to connect to MySQL instance
        #[serde(default = "default_user")]
        user: String,

        /// Password used to connect to MySQL instance
        #[serde(default)]
        password: Option<String>,
    },
}

#[derive(Configurable, Clone, Serialize, Deserialize, Debug)]
struct CollectConfig {
    #[serde(default)]
    global: global::Config,

    /// Since 5.1, collect the current size of all registered binlog files
    #[serde(default)]
    binlog_size: bool,

    /// Since 5.7, Collect per user metrics from sys.x$user_summary.
    ///
    /// See https://dev.mysql.com/doc/refman/5.7/en/sys-user-summary.html for details
    #[serde(default)]
    sys_user_summary: bool,

    #[serde(default)]
    user: Option<user::Config>,

    #[serde(default)]
    engine: Option<engine::Config>,

    #[serde(default)]
    heartbeat: Option<heartbeat::Config>,

    #[serde(default)]
    info_schema: information::Config,

    #[serde(default)]
    perf_schema: performance::Config,

    #[serde(default)]
    slave: slave::Config,
}

#[configurable_component(source, name = "mysql")]
#[serde(deny_unknown_fields)]
struct Config {
    /// Endpoint to the MySQL/MariaDB instances
    endpoints: Vec<String>,

    /// Authentication used for connecting to MySQL instance
    auth: AuthConfig,

    /// Duration between each scrape.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    // tls: Option<TlsConfig>,
    #[serde(flatten, default)]
    collect: CollectConfig,
}

#[async_trait::async_trait]
#[typetag::serde(name = "mysql")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let interval = self.interval;
        let output = cx.output;
        let shutdown = cx.shutdown;
        let auth = match &self.auth {
            AuthConfig::NativePassword { user, password } => connection::AuthConfig {
                username: user.clone(),
                password: password.clone(),
            },
        };

        let mut endpoints = self.endpoints.clone();
        let collect_config = self.collect.clone();

        Ok(Box::pin(async move {
            endpoints.dedup();

            let mut tasks = FuturesUnordered::from_iter(endpoints.into_iter().map(|endpoint| {
                run(
                    endpoint,
                    &auth,
                    interval,
                    &collect_config,
                    shutdown.clone(),
                    output.clone(),
                )
            }));

            while tasks.next().await.is_none() {}

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::metric()]
    }
}

async fn run(
    mut endpoint: String,
    auth: &connection::AuthConfig,
    interval: Duration,
    config: &CollectConfig,
    mut shutdown: ShutdownSignal,
    mut output: Pipeline,
) -> Result<(), ()> {
    let (host, port) = match endpoint.split_once(':') {
        Some((host, port)) => {
            let port = port.parse::<u16>().map_err(|_| ())?;
            endpoint.truncate(host.len());

            (endpoint, port)
        }
        None => (endpoint, 3306),
    };

    let instance = format!("{}:{}", host, port);
    let offset = calculate_offset(&instance, interval);
    let mut ticker = tokio::time::interval_at(tokio::time::Instant::now().add(offset), interval);
    let timeout = interval / 3 * 2;

    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            _ = ticker.tick() => {}
        }

        let start = Instant::now();
        let result = tokio::time::timeout(timeout, collect(&instance, auth, config)).await;
        let elapsed = start.elapsed();

        let health = [
            Metric::gauge(
                "mysql_up",
                "Whether the MySQL server is up.",
                matches!(result, Ok(Ok(_))),
            ),
            Metric::gauge("mysql_scrape_duration_seconds", "", elapsed),
        ];

        let metrics = match result {
            Ok(Ok(mut metrics)) => {
                metrics.extend(health);
                metrics
            }
            Ok(Err(_err)) => health.to_vec(),
            Err(_) => {
                warn!(message = "collecting metrics timeout", instance, ?elapsed,);

                health.to_vec()
            }
        };

        if let Err(_err) = output.send(metrics).await {
            break;
        }
    }

    Ok(())
}

fn calculate_offset(key: &str, interval: Duration) -> Duration {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    let hash = hasher.finish();

    let ms = hash % (interval.as_millis() as u64);
    Duration::from_millis(ms)
}

async fn collect(
    endpoint: &str,
    auth: &connection::AuthConfig,
    conf: &CollectConfig,
) -> Result<Vec<Metric>, Error> {
    let mut conn = match connect(endpoint, auth).await {
        Ok(conn) => conn,
        Err(err) => {
            warn!(
                message = "connect to MySQL server failed",
                endpoint,
                %err
            );

            return Err(err);
        }
    };

    let version = conn.version();

    let mut metrics = match global::collect(&mut conn, &conf.global).await {
        Ok(partial) => partial,
        Err(err) => {
            warn!(message = "collecting global failed", %err);
            return Err(err.into());
        }
    };

    if conf.binlog_size && version >= 5.1 {
        match binlog_size::collect(&mut conn).await {
            Ok(partial) => metrics.extend(partial),
            Err(err) => {
                warn!(message = "collecting binlog size failed", %err);

                return Err(err);
            }
        }
    }

    if let Some(conf) = &conf.user {
        match user::collect(&mut conn, conf).await {
            Ok(partial) => metrics.extend(partial),
            Err(err) => {
                warn!(message = "collecting users failed", %err);
                return Err(err.into());
            }
        }
    }

    if let Some(conf) = &conf.engine {
        match engine::collect(&mut conn, conf).await {
            Ok(partial) => metrics.extend(partial),
            Err(err) => {
                warn!(message = "collecting engine failed", %err);
                return Err(err.into());
            }
        }
    }

    if conf.sys_user_summary && version >= 5.7 {
        match sys_user_summary::collect(&mut conn).await {
            Ok(partial) => metrics.extend(partial),
            Err(err) => {
                warn!(message = "collecting system users summary failed", %err);
                return Err(err.into());
            }
        }
    }

    if let Some(conf) = &conf.heartbeat
        && version >= 5.1
    {
        match heartbeat::collect(&mut conn, conf).await {
            Ok(partial) => metrics.extend(partial),
            Err(err) => {
                warn!(message = "collecting heartbeat failed", %err);
                return Err(err.into());
            }
        }
    }

    match slave::collect(&mut conn, &conf.slave).await {
        Ok(partial) => metrics.extend(partial),
        Err(err) => {
            warn!(message = "collecting slave failed", %err);
            return Err(err.into());
        }
    }

    match information::collect(&mut conn, &conf.info_schema).await {
        Ok(partial) => metrics.extend(partial),
        Err(err) => {
            warn!(message = "collecting info schemas failed", %err);
            return Err(err.into());
        }
    }

    match performance::collect(&mut conn, &conf.perf_schema).await {
        Ok(partial) => metrics.extend(partial),
        Err(err) => {
            warn!(message = "collecting performance schemas failed", %err);
            return Err(err.into());
        }
    }

    if let Err(err) = conn.close().await {
        warn!(message = "close mysql connection failed", %err);
        return Err(err.into());
    }

    Ok(metrics)
}

async fn connect(endpoint: &str, auth: &connection::AuthConfig) -> Result<Connection, Error> {
    let mut addrs = tokio::net::lookup_host(&endpoint).await?;
    let addr = addrs.next().ok_or(Error::Resolve)?;
    Connection::connect(addr, auth).await.map_err(Into::into)
}

pub fn sanitize(name: &str) -> String {
    let name = name.replace(|c: char| !c.is_ascii_alphanumeric() && c != '_', "_");

    name.to_lowercase()
}

#[cfg(test)]
pub fn assert_contains(
    metrics: &[Metric],
    gauges: Vec<(event::tags::Tags, f64)>,
    counters: Vec<(event::tags::Tags, f64)>,
) {
    use event::MetricValue;

    for (tags, value) in gauges {
        assert!(
            metrics
                .iter()
                .filter(|m| matches!(m.value(), MetricValue::Gauge(mv) if *mv == value))
                .any(|m| m.tags == tags),
            "want gauge {:?} {}",
            tags,
            value
        );
    }

    for (tags, value) in counters {
        assert!(
            metrics
                .iter()
                .filter(|m| matches!(m.value(), MetricValue::Sum(mv) if *mv == value))
                .any(|m| m.tags == tags),
            "want sum {:?} {}",
            tags,
            value
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }
}
