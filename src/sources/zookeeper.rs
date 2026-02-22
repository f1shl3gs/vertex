use std::collections::BTreeMap;
use std::io::BufRead;
use std::net::SocketAddr;
use std::time::Duration;
use std::time::Instant;

use bytes::{Bytes, BytesMut};
use configurable::configurable_component;
use event::{Metric, tags};
use framework::config::{OutputType, SourceConfig, SourceContext, default_interval};
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::{Error, Source};
use tokio::net::TcpStream;
use tokio::task::JoinSet;

#[configurable_component(source, name = "zookeeper")]
struct Config {
    /// The endpoints to connect to.
    #[configurable(required)]
    endpoints: Vec<SocketAddr>,

    /// Duration between each scrape.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "zookeeper")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let endpoints = self.endpoints.clone();
        let interval = self.interval;
        let output = cx.output.clone();
        let shutdown = cx.shutdown.clone();

        Ok(Box::pin(async move {
            let mut tasks = JoinSet::from_iter(
                endpoints
                    .into_iter()
                    .map(|endpoint| run(endpoint, interval, output.clone(), shutdown.clone())),
            );

            while tasks.join_next().await.is_some() {}

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::metric()]
    }
}

async fn run(
    target: SocketAddr,
    interval: Duration,
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let mut scrapes = 0;
    let mut errors = 0;
    let target_value = target.to_string();

    let start = crate::common::calculate_start(&target, interval);
    let mut ticker = tokio::time::interval_at(start.into(), interval);

    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            _ = ticker.tick() => {}
        }

        scrapes += 1;
        let start = Instant::now();
        let (mut metrics, last_err) = match collect(target).await {
            Ok(metrics) => (metrics, 0),
            Err(err) => {
                warn!(message = "failed to collect metrics", ?err);
                errors += 1;

                (vec![], 1)
            }
        };
        let elapsed = start.elapsed();

        metrics.extend([
            Metric::gauge_with_tags(
                "zk_scrape_total",
                "",
                scrapes,
                tags!("instance" => target.to_string()),
            ),
            Metric::gauge_with_tags(
                "zk_scrape_errors",
                "",
                errors,
                tags!("instance" => target.to_string()),
            ),
            Metric::gauge_with_tags(
                "zk_last_scrape_error",
                "",
                last_err,
                tags!("instance" => target.to_string()),
            ),
            Metric::gauge_with_tags(
                "zk_last_scrape_duration_seconds",
                "",
                elapsed,
                tags!("instance" => target.to_string()),
            ),
        ]);

        let now = chrono::Utc::now();
        metrics.iter_mut().for_each(|metric| {
            metric.timestamp = Some(now);
            metric.tags_mut().insert("target", &target_value);
        });

        if let Err(err) = output.send(metrics).await {
            error!(
                message = "Error sending zookeeper metrics",
                %err
            );

            return Err(());
        }
    }

    Ok(())
}

async fn collect(target: SocketAddr) -> Result<Vec<Metric>, Error> {
    let mut metrics = Vec::new();

    match request(target, "mntr\n").await {
        Ok(resp) => {
            let (version, state, peer_state, stats) = parse_mntr(resp)?;
            metrics.reserve(stats.len() + 2);

            metrics.extend([
                Metric::gauge_with_tags(
                    "zk_version",
                    "",
                    1,
                    tags!(
                        "version" => version,
                        "instance" => target.to_string()
                    ),
                ),
                Metric::gauge_with_tags(
                    "zk_server_state",
                    "",
                    1,
                    tags!(
                        "state" => state,
                        "instance" => target.to_string()
                    ),
                ),
                Metric::gauge_with_tags(
                    "zk_peer_state",
                    "",
                    1,
                    tags!(
                        "state" => peer_state,
                        "instance" => target.to_string(),
                    ),
                ),
            ]);

            for (key, value) in stats {
                let desc = format!("{key} value of mntr");
                metrics.push(Metric::gauge_with_tags(
                    key,
                    desc,
                    value,
                    tags!(
                        "instance" => target.to_string()
                    ),
                ));
            }
        }
        Err(err) => {
            warn!(
                message = "fetch mntr stats failed",
                %err
            );
        }
    }

    match request(target, "ruok\n").await {
        Ok(resp) => metrics.push(Metric::gauge_with_tags(
            "zk_ok",
            "Is ZooKeeper currently OK",
            resp.as_ref() == b"imok",
            tags!(
                "instance" => target.to_string()
            ),
        )),
        Err(err) => {
            warn!(
                message = "fetch ruok stats failed",
                %err
            );
        }
    }

    Ok(metrics)
}

fn parse_version(input: &str) -> String {
    let input = input.strip_prefix("zk_version").unwrap_or(input);
    let version = input.trim_start().split(',').next().unwrap_or("");

    version.to_string()
}

async fn request(addr: SocketAddr, cmd: &str) -> Result<Bytes, Error> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut socket = TcpStream::connect(addr).await?;

    socket.write_all(cmd.as_bytes()).await?;

    let mut buf = BytesMut::with_capacity(128);
    loop {
        let cnt = socket.read_buf(&mut buf).await?;
        if cnt == 0 {
            break;
        }
    }

    Ok(buf.freeze())
}

fn parse_mntr(input: Bytes) -> Result<(String, String, String, BTreeMap<String, f64>), Error> {
    let mut lines = input.lines();

    let mut version = String::new();
    let mut server_state = String::new();
    let mut peer_state = String::new();
    let mut stats = BTreeMap::new();
    while let Some(Ok(line)) = lines.next() {
        let (key, value) = match line.split_once('\t') {
            Some(pair) => pair,
            None => {
                warn!(message = "split mntr line failed", line);
                continue;
            }
        };

        if key == "zk_version" {
            version = parse_version(&line);
            continue;
        }

        if key == "zk_server_state" {
            server_state = value.to_string();
            continue;
        }

        if key == "zk_peer_state" {
            peer_state = value.to_string();
            continue;
        }

        if let Ok(v) = value.parse::<f64>() {
            stats.insert(key.to_string(), v);
        } else {
            warn!(message = "parse mntr value failed", line);
        }
    }

    Ok((version, server_state, peer_state, stats))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }

    #[test]
    fn test_parse_version() {
        let input = "zk_version	3.7.0-e3704b390a6697bfdf4b0bef79e3da7a4f6bac4b, built on 2021-03-17 09:46 UTC";
        let v = parse_version(input);
        assert_eq!(v, "3.7.0-e3704b390a6697bfdf4b0bef79e3da7a4f6bac4b")
    }
}

#[cfg(all(test, feature = "zookeeper-integration-tests"))]
mod integration_tests {
    use std::time::Duration;

    use testify::container::Container;
    use testify::next_addr;

    use super::*;
    use crate::testing::trace_init;

    #[tokio::test]
    async fn test_fetch_stats() {
        trace_init();

        let service_addr = next_addr();

        Container::new("zookeeper", "3.6.2")
            .with_tcp(2181, service_addr.port())
            .with_env("ZOO_4LW_COMMANDS_WHITELIST", "*")
            .run(async move {
                tokio::time::sleep(Duration::from_secs(5)).await;

                let resp = request(service_addr, "mntr\n").await.unwrap();
                let (version, state, _peer_state, stats) = parse_mntr(resp).unwrap();

                assert_eq!(version, "3.6.2--803c7f1a12f85978cb049af5e4ef23bd8b688715");
                assert_eq!(state, "standalone");
                assert!(*stats.get("zk_uptime").unwrap() > 0.0);
            })
            .await;
    }
}
