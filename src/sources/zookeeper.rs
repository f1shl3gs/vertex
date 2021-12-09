// mntr

use std::collections::BTreeMap;

use futures::{SinkExt, StreamExt};
use event::{tags, Metric};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_stream::wrappers::IntervalStream;

use crate::pipeline::Pipeline;
use crate::shutdown::ShutdownSignal;
use crate::sources::Source;
use crate::Error;
use crate::config::{
    DataType, SourceConfig, SourceContext, deserialize_duration,
    serialize_duration, default_interval, GenerateConfig, SourceDescription
};


#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ZookeeperConfig {
    endpoint: String,

    #[serde(deserialize_with = "deserialize_duration", serialize_with = "serialize_duration")]
    #[serde(default = "default_interval")]
    interval: chrono::Duration,
}

impl GenerateConfig for ZookeeperConfig {
    fn generate_config() -> Value {
        serde_yaml::to_value(Self {
            endpoint: "127.0.0.1:9092".to_string(),
            interval: default_interval(),
        }).unwrap()
    }
}

inventory::submit! {
    SourceDescription::new::<ZookeeperConfig>("zookeeper")
}

struct ZookeeperSource {
    endpoint: String,
}

impl ZookeeperSource {
    fn from(conf: &ZookeeperConfig) -> Self {
        Self {
            endpoint: conf.endpoint.clone()
        }
    }

    async fn run(
        self,
        interval: std::time::Duration,
        mut output: Pipeline,
        shutdown: ShutdownSignal,
    ) -> Result<(), ()> {
        let interval = tokio::time::interval(interval);
        let mut ticker = IntervalStream::new(interval)
            .take_until(shutdown);

        let endpoint = self.endpoint.as_str();
        while let Some(_) = ticker.next().await {
            match fetch_stats(endpoint).await {
                Ok((version, state, stats)) => {
                    let mut metrics = Vec::with_capacity(stats.len() + 2);
                    metrics.extend_from_slice(&[
                        Metric::gauge_with_tags(
                            "zk_up",
                            "",
                            1,
                            tags!(
                                "instance" => endpoint
                            ),
                        ),
                        Metric::gauge_with_tags(
                            "zk_version",
                            "",
                            1,
                            tags!(
                                "version" => version,
                                "instance" => endpoint
                            ),
                        ),
                        Metric::gauge_with_tags(
                            "zk_server_state",
                            "",
                            1,
                            tags!(
                                "state" => state,
                                "instance" => endpoint
                            ),
                        )
                    ]);

                    for (key, value) in stats {
                        metrics.push(Metric::gauge_with_tags(
                            key.as_str(),
                            format!("{} value of mntr", key),
                            value,
                            tags!(
                                "instance" => endpoint
                            ),
                        ).into());
                    }

                    let now = chrono::Utc::now();
                    let mut stream = futures::stream::iter(metrics)
                        .map(|mut m| {
                            m.timestamp = Some(now);
                            Ok(m.into())
                        });
                    output.send_all(&mut stream).await;
                }
                Err(err) => {
                    output.send(Metric::gauge_with_tags(
                        "zk_up",
                        "",
                        0,
                        tags!(
                            "instance" => endpoint
                        ),
                    ).into()).await;
                }
            }
        }

        Ok(())
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "zookeeper")]
impl SourceConfig for ZookeeperConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        let source = ZookeeperSource::from(self);
        Ok(Box::pin(source.run(
            self.interval.to_std().unwrap(),
            ctx.out,
            ctx.shutdown,
        )))
    }

    fn output_type(&self) -> DataType {
        DataType::Metric
    }

    fn source_type(&self) -> &'static str {
        "zookeeper"
    }
}

fn parse_version(input: &str) -> String {
    let input = input.strip_prefix("zk_version").unwrap();
    let version = input.trim_start()
        .split(',')
        .nth(0)
        .unwrap_or("");

    version.to_string()
}

fn parse_server_state(line: &str) -> String {
    todo!()
}

fn parse_peer_state(line: &str) -> String {
    todo!()
}

async fn fetch_stats(addr: &str) -> Result<(String, String, BTreeMap<String, f64>), Error> {
    let socket = TcpStream::connect(addr).await?;
    let (reader, mut writer) = tokio::io::split(socket);

    // Write `mntr`
    writer.write_all(b"mntr\n").await?;

    let reader = tokio::io::BufReader::new(reader);
    let mut lines = reader.lines();

    let mut version = String::new();
    let mut server_state = String::new();
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
            continue;
        }

        if let Ok(v) = value.parse::<f64>() {
            stats.insert(key.to_string(), v);
        } else {
            warn!("parse mntr value failed, parsing: {}", line);
        }
    }

    Ok((version, server_state, stats))
}

#[cfg(test)]
mod tests {
    use super::parse_version;

    #[test]
    fn test_parse_version() {
        let input = "zk_version	3.7.0-e3704b390a6697bfdf4b0bef79e3da7a4f6bac4b, built on 2021-03-17 09:46 UTC";
        let v = parse_version(input);
        assert_eq!(v, "3.7.0-e3704b390a6697bfdf4b0bef79e3da7a4f6bac4b")
    }
}

#[cfg(all(test, feature = "integration-tests-zookeeper"))]
mod integration_tests {
    use testcontainers::Docker;
    use zk::Zookeeper;
    use super::fetch_stats;

    mod zk {
        use std::collections::HashMap;
        use testcontainers::{Container, Docker, Image, WaitForMessage};

        const CONTAINER_IDENTIFIER: &str = "bitnami/zookeeper";
        const DEFAULT_TAG: &str = "3.7.0";

        #[derive(Debug, Default, Clone)]
        pub struct ZookeeperArgs;

        impl IntoIterator for ZookeeperArgs {
            type Item = String;
            type IntoIter = ::std::vec::IntoIter<String>;

            fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
                vec![].into_iter()
            }
        }

        #[derive(Debug)]
        pub struct Zookeeper {
            tag: String,
            arguments: ZookeeperArgs,
            envs: HashMap<String, String>,
        }

        impl Default for Zookeeper {
            fn default() -> Self {
                Zookeeper {
                    tag: DEFAULT_TAG.to_string(),
                    arguments: ZookeeperArgs {},
                    envs: Default::default(),
                }
            }
        }

        impl Image for Zookeeper {
            type Args = ZookeeperArgs;
            type EnvVars = HashMap<String, String>;
            type Volumes = HashMap<String, String>;
            type EntryPoint = std::convert::Infallible;
            fn descriptor(&self) -> String {
                format!("{}:{}", CONTAINER_IDENTIFIER, &self.tag)
            }

            fn wait_until_ready<D: Docker>(&self, container: &Container<'_, D, Self>) {
                container
                    .logs()
                    .stdout
                    .wait_for_message("The list of known four letter word commands is")
                    .unwrap();
            }

            fn args(&self) -> <Self as Image>::Args {
                self.arguments.clone()
            }

            fn env_vars(&self) -> Self::EnvVars {
                self.envs.clone()
            }

            fn volumes(&self) -> Self::Volumes {
                HashMap::new()
            }

            fn with_args(self, arguments: <Self as Image>::Args) -> Self {
                Zookeeper { arguments, ..self }
            }
        }

        impl Zookeeper {
            pub fn with_tag(self, tag_str: &str) -> Self {
                Zookeeper {
                    tag: tag_str.to_string(),
                    ..self
                }
            }

            pub fn with_env(self, key: &str, value: &str) -> Self {
                let mut envs = self.envs.clone();
                envs.insert(key.to_string(), value.to_string());

                Zookeeper {
                    envs,
                    ..self
                }
            }
        }
    }

    #[tokio::test]
    async fn test_fetch_stats() {
        let docker = testcontainers::clients::Cli::default();
        let image = Zookeeper::default()
            .with_env("ALLOW_ANONYMOUS_LOGIN", "yes");

        let service = docker.run(image);
        let host_port = service.get_host_port(2181).unwrap();
        let addr = format!("127.0.0.1:{}", host_port);

        let (version, state, stats) = fetch_stats(addr.as_str()).await.unwrap();
        assert_eq!(version, "3.7.0-e3704b390a6697bfdf4b0bef79e3da7a4f6bac4b");
        assert_eq!(state, "standalone");
        assert_eq!(*stats.get("zk_uptime").unwrap() > 0.0, true);
    }
}