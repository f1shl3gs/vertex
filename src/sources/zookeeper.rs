use std::borrow::Cow;
use std::collections::BTreeMap;
use std::time::Duration;

use configurable::configurable_component;
use event::{tags, Metric, INSTANCE_KEY};
use framework::config::{default_interval, DataType, Output, SourceConfig, SourceContext};
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::{Error, Source};
use futures::StreamExt;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_stream::wrappers::IntervalStream;

#[configurable_component(source, name = "zookeeper")]
#[derive(Clone, Debug)]
#[serde(deny_unknown_fields)]
struct ZookeeperConfig {
    /// The endpoints to connect to.
    #[configurable(required)]
    endpoint: String,

    /// Duration between each scrape.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,
}

struct ZookeeperSource {
    endpoint: String,
}

impl ZookeeperSource {
    fn from(conf: &ZookeeperConfig) -> Self {
        Self {
            endpoint: conf.endpoint.clone(),
        }
    }

    async fn run(
        self,
        interval: std::time::Duration,
        mut output: Pipeline,
        shutdown: ShutdownSignal,
    ) -> Result<(), ()> {
        let interval = tokio::time::interval(interval);
        let mut ticker = IntervalStream::new(interval).take_until(shutdown);

        let endpoint = Cow::from(self.endpoint.clone());
        while let Some(_ts) = ticker.next().await {
            let mut metrics = match fetch_stats(endpoint.as_ref()).await {
                Ok((version, state, peer_state, stats)) => {
                    let mut metrics = Vec::with_capacity(stats.len() + 2);
                    metrics.extend_from_slice(&[
                        Metric::gauge_with_tags(
                            "zk_up",
                            "",
                            1,
                            tags!(
                                INSTANCE_KEY => endpoint.clone()
                            ),
                        ),
                        Metric::gauge_with_tags(
                            "zk_version",
                            "",
                            1,
                            tags!(
                                "version" => version,
                                INSTANCE_KEY => endpoint.clone()
                            ),
                        ),
                        Metric::gauge_with_tags(
                            "zk_server_state",
                            "",
                            1,
                            tags!(
                                "state" => state,
                                INSTANCE_KEY => endpoint.clone()
                            ),
                        ),
                        Metric::gauge_with_tags(
                            "zk_peer_state",
                            "",
                            1,
                            tags!(
                                "state" => peer_state,
                                INSTANCE_KEY => endpoint.clone(),
                            ),
                        ),
                    ]);

                    for (key, value) in stats {
                        metrics.push(Metric::gauge_with_tags(
                            key.as_str(),
                            format!("{} value of mntr", key),
                            value,
                            tags!(
                                INSTANCE_KEY => endpoint.clone()
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
                            INSTANCE_KEY => endpoint.clone()
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
}

#[async_trait::async_trait]
#[typetag::serde(name = "zookeeper")]
impl SourceConfig for ZookeeperConfig {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let source = ZookeeperSource::from(self);
        Ok(Box::pin(source.run(self.interval, cx.output, cx.shutdown)))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }
}

fn parse_version(input: &str) -> String {
    let input = input.strip_prefix("zk_version").unwrap();
    let version = input.trim_start().split(',').next().unwrap_or("");

    version.to_string()
}

async fn fetch_stats(addr: &str) -> Result<(String, String, String, BTreeMap<String, f64>), Error> {
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
                warn!("split mntr line failed, parsing: \"{}\"", line);
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
            warn!("parse mntr value failed, parsing: {}", line);
        }
    }

    Ok((version, server_state, peer_state, stats))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<ZookeeperConfig>()
    }

    #[test]
    fn test_parse_version() {
        let input = "zk_version	3.7.0-e3704b390a6697bfdf4b0bef79e3da7a4f6bac4b, built on 2021-03-17 09:46 UTC";
        let v = parse_version(input);
        assert_eq!(v, "3.7.0-e3704b390a6697bfdf4b0bef79e3da7a4f6bac4b")
    }
}

#[cfg(all(test, feature = "integration-tests-zookeeper"))]
mod integration_tests {
    use super::fetch_stats;
    use testcontainers::images::zookeeper::Zookeeper;
    use testcontainers::RunnableImage;

    #[tokio::test]
    async fn test_fetch_stats() {
        let client = testcontainers::clients::Cli::default();
        let image = RunnableImage::from(Zookeeper::default())
            // .with_env_var(("ALLOW_ANONYMOUS_LOGIN", "yes"))
            .with_env_var(("ZOO_4LW_COMMANDS_WHITELIST", "*"));

        let service = client.run(image);
        let host_port = service.get_host_port_ipv4(2181);
        let addr = format!("127.0.0.1:{}", host_port);

        let (version, state, _peer_state, stats) = fetch_stats(addr.as_str()).await.unwrap();
        assert_eq!(version, "3.6.2--803c7f1a12f85978cb049af5e4ef23bd8b688715");
        assert_eq!(state, "standalone");
        assert!(*stats.get("zk_uptime").unwrap() > 0.0);
    }
}
