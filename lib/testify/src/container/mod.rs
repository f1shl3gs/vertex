mod docker;

use std::collections::HashMap;
use std::net::SocketAddr;

use docker::{HostConfig, LogOutput, PortBinding};
use futures::{StreamExt, TryStreamExt};
use tokio_util::codec::FramedRead;
use tokio_util::io::StreamReader;
use tracing::info;

use super::wait::wait_for_tcp;

pub struct Container {
    image: String,
    tag: String,

    args: Vec<String>,
    environments: Vec<String>,
    volumes: Vec<String>,
    ports: HashMap<String, Vec<PortBinding>>,
    stdout: bool,
    stderr: bool,
    memory_limit: usize,
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
            stdout: false,
            stderr: false,
            memory_limit: 0,
        }
    }

    pub fn with_tcp(mut self, port: u16, host_port: u16) -> Self {
        let key = format!("{port}/tcp");
        let pb = PortBinding {
            host_ip: None,
            host_port: Some(host_port.to_string()),
        };

        self.ports.insert(key, vec![pb]);
        self
    }

    pub fn with_udp(mut self, port: u16, host_port: u16) -> Self {
        let key = format!("{port}/udp");
        let pb = PortBinding {
            host_ip: None,
            host_port: Some(host_port.to_string()),
        };

        self.ports.insert(key, vec![pb]);
        self
    }

    pub fn with_volume<S>(mut self, orig: S, dest: &str) -> Self
    where
        S: std::fmt::Display,
    {
        self.volumes.push(format!("{orig}:{dest}"));
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

    pub fn tail_logs(mut self, stdout: bool, stderr: bool) -> Self {
        self.stdout = stdout;
        self.stderr = stderr;
        self
    }

    pub fn with_memory_limit(mut self, memory_limit: usize) -> Self {
        self.memory_limit = memory_limit;
        self
    }

    pub async fn run<T>(self, f: impl Future<Output = T>) -> T {
        let client = docker::Client::default();

        client.pull(&self.image, &self.tag).await.unwrap();

        let exposed_ports = if self.ports.is_empty() {
            None
        } else {
            Some(
                self.ports
                    .keys()
                    .map(|k| (k.to_string(), Default::default()))
                    .collect(),
            )
        };

        let options = docker::CreateOptions {
            image: format!("{}:{}", self.image, self.tag),
            env: self.environments,
            cmd: self.args,
            exposed_ports,
            host_config: HostConfig {
                extra_hosts: vec!["host.docker.internal:host-gateway".into()],
                binds: self.volumes,
                port_bindings: self.ports.clone(),
                memory: self.memory_limit,
            },
        };

        let id = client.create(options).await.unwrap();

        // tail logs
        if self.stdout || self.stderr {
            let cid = id.clone();
            let mc = client.clone();
            let stdout = self.stdout;
            let stderr = self.stderr;

            tokio::spawn(async move {
                let reader = mc.tail_logs(&cid, stdout, stderr).await.unwrap();

                let mut reader = FramedRead::new(
                    StreamReader::new(
                        Box::pin(
                            reader.try_filter_map(|frame| async { Ok(frame.into_data().ok()) }),
                        )
                        .map_err(|err| {
                            if err.is_timeout() {
                                return std::io::Error::new(std::io::ErrorKind::TimedOut, err);
                            }

                            std::io::Error::other(err)
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
        }

        client.start(&id).await.unwrap();

        if !self.ports.is_empty() {
            let ip = client.inspect_ip_address(&id).await.unwrap();
            for key in self.ports.keys() {
                if let Some(ps) = key.strip_suffix("/tcp") {
                    let port = ps.parse::<u16>().unwrap();
                    let addr = SocketAddr::new(ip, port);

                    info!("wait for tcp {}", addr);

                    wait_for_tcp(SocketAddr::from((ip, port))).await;
                }
            }
        }

        let result = f.await;

        let _ = client.stop(&id).await;
        client.remove(&id).await.unwrap();

        result
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use http_body_util::Full;
    use hyper::Uri;
    use hyper::body::Bytes;
    use hyper_util::client::legacy::connect::HttpConnector;
    use hyper_util::rt::TokioExecutor;

    use super::*;
    use crate::next_addr;

    #[tokio::test]
    async fn nginx() {
        let output = next_addr();

        Container::new("nginx", "1.27.4")
            .with_tcp(80, output.port())
            .tail_logs(true, true)
            .run(async {
                let client: hyper_util::client::legacy::Client<HttpConnector, Full<Bytes>> =
                    hyper_util::client::legacy::Client::builder(TokioExecutor::default())
                        .build(HttpConnector::new());

                let uri = Uri::from_str(&format!("http://{output}")).unwrap();
                let resp = client.get(uri).await.unwrap();

                assert_eq!(resp.status(), 200);
            })
            .await;
    }
}
