use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::time::Duration;

use configurable::configurable_component;
use event::{Metric, tags};
use framework::config::{Output, SourceConfig, SourceContext, default_interval};
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::{Error, Source};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[configurable_component(source, name = "zookeeper")]
#[serde(deny_unknown_fields)]
struct Config {
    /// The endpoints to connect to.
    #[configurable(required)]
    target: SocketAddr,

    /// Duration between each scrape.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "zookeeper")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        Ok(Box::pin(run(
            self.target,
            self.interval,
            cx.output,
            cx.shutdown,
        )))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::metrics()]
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

async fn run(
    target: SocketAddr,
    interval: Duration,
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let mut ticker = tokio::time::interval(interval);

    loop {
        tokio::select! {
            biased;

            _ = &mut shutdown => break,
            _ = ticker.tick() => {}
        }

        let mut metrics = match fetch_stats(target).await {
            Ok((version, state, peer_state, stats)) => {
                let mut metrics = Vec::with_capacity(stats.len() + 2);
                metrics.extend([
                    Metric::gauge_with_tags(
                        "zk_up",
                        "",
                        1,
                        tags!(
                            "instance" => target.to_string(),
                        ),
                    ),
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
                    metrics.push(Metric::gauge_with_tags(
                        key.as_str(),
                        format!("{} value of mntr", key),
                        value,
                        tags!(
                            "instance" => target.to_string()
                        ),
                    ));
                }

                metrics
            }
            Err(err) => {
                warn!(
                    message = "Fetch zookeeper stats failed",
                    %err
                );

                vec![Metric::gauge_with_tags(
                    "zk_up",
                    "",
                    0,
                    tags!(
                        "instance" => target.to_string()
                    ),
                )]
            }
        };

        let now = chrono::Utc::now();
        metrics.iter_mut().for_each(|m| m.timestamp = Some(now));

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

fn parse_version(input: &str) -> String {
    let input = input.strip_prefix("zk_version").unwrap_or(input);
    let version = input.trim_start().split(',').next().unwrap_or("");

    version.to_string()
}

async fn fetch_stats(
    addr: SocketAddr,
) -> Result<(String, String, String, BTreeMap<String, f64>), Error> {
    let socket = TcpStream::connect(addr).await?;
    let (reader, mut writer) = tokio::io::split(socket);

    // Write `mntr`
    writer.write_all(b"mntr\n").await?;

    let reader = tokio::io::BufReader::new(reader);
    let mut lines = reader.lines();

    let mut version = String::new();
    let mut server_state = String::new();
    let mut peer_state = String::new();
    let mut stats = BTreeMap::new();
    while let Some(line) = lines.next_line().await? {
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

    use super::fetch_stats;
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

                let (version, state, _peer_state, stats) = fetch_stats(service_addr).await.unwrap();
                assert_eq!(version, "3.6.2--803c7f1a12f85978cb049af5e4ef23bd8b688715");
                assert_eq!(state, "standalone");
                assert!(*stats.get("zk_uptime").unwrap() > 0.0);
            })
            .await;
    }
}
