mod docker;

use std::collections::HashMap;
use std::future::Future;

use futures_util::{StreamExt, TryStreamExt};
use tokio_util::codec::FramedRead;
use tokio_util::io::StreamReader;

use docker::{HostConfig, LogOutput, PortBinding};

pub struct Container {
    image: String,
    tag: String,

    args: Vec<String>,
    environments: Vec<String>,
    volumes: Vec<String>,
    ports: HashMap<String, Vec<PortBinding>>,
}

impl Container {
    pub fn new(image: impl Into<String>, tag: impl Into<String>) -> Self {
        Self {
            image: image.into(),
            tag: tag.into(),
            args: vec![],
            environments: vec![],
            volumes: vec![],
            ports: HashMap::new(),
        }
    }

    pub fn with_tcp(mut self, port: u16, publish: u16) -> Self {
        let key = format!("{port}/tcp");
        let pb = PortBinding {
            host_ip: "".to_string(),
            host_port: publish.to_string(),
        };

        self.ports.insert(key, vec![pb]);
        self
    }

    pub fn with_volume<S, T>(mut self, orig: S, dest: T) -> Self
    where
        S: Into<String>,
        T: Into<String>,
    {
        self.volumes
            .push(format!("{}:{}", orig.into(), dest.into()));
        self
    }

    pub fn with_env<S>(mut self, key: S, value: S) -> Self
    where
        S: Into<String>,
    {
        self.environments
            .push(format!("{}={}", key.into(), value.into()));
        self
    }

    pub fn args<S, T>(mut self, args: T) -> Self
    where
        S: Into<String>,
        T: IntoIterator<Item = S>,
    {
        self.args.extend(args.into_iter().map(|s| s.into()));
        self
    }

    pub async fn run<T>(self, f: impl Future<Output = T>) -> T {
        let client = docker::Client::default();

        client.pull(&self.image, &self.tag).await.unwrap();

        let options = docker::CreateOptions {
            image: format!("{}:{}", self.image, self.tag),
            env: self.environments,
            cmd: self.args,
            host_config: HostConfig {
                extra_hosts: vec!["host.docker.internal:host-gateway".into()],
                binds: self.volumes,
                port_bindings: self.ports,
            },
        };

        let id = client.create(options).await.unwrap();

        client.start(&id).await.unwrap();

        // tail logs
        let cid = id.clone();
        let mc = client.clone();
        tokio::spawn(async move {
            let reader = mc.tail_logs(&cid).await.unwrap();

            let mut reader = FramedRead::new(
                StreamReader::new(
                    Box::pin(reader.try_filter_map(|frame| async { Ok(frame.into_data().ok()) }))
                        .map_err(|err| {
                            if err.is_timeout() {
                                return std::io::Error::new(std::io::ErrorKind::TimedOut, err);
                            }

                            std::io::Error::new(std::io::ErrorKind::Other, err)
                        }),
                ),
                docker::NewlineLogOutputDecoder::default(),
            );

            while let Some(Ok(log)) = reader.next().await {
                match log {
                    LogOutput::Stdout(msg) => {
                        println!("stdout | {}", String::from_utf8_lossy(&msg))
                    }
                    LogOutput::Stderr(msg) => {
                        println!("stderr | {}", String::from_utf8_lossy(&msg))
                    }
                }
            }
        });

        let result = f.await;

        let _ = client.stop(&id).await;
        client.remove(&id).await.unwrap();

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::next_addr;
    use crate::wait::wait_for_tcp;

    #[tokio::test]
    async fn nginx() {
        let output = next_addr();

        Container::new("nginx", "1.21.3")
            .with_tcp(80, output.port())
            .run(async {
                wait_for_tcp(output).await;
            })
            .await;
    }
}
