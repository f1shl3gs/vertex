use std::collections::HashMap;
use std::fmt::Display;
use std::io::{BufRead, BufReader, ErrorKind, Read};
use std::net::SocketAddr;
use std::process::{Command, Stdio};

use serde::Deserialize;

#[derive(Clone)]
enum Port {
    Tcp(u16),
    Udp(u16),
}

impl Display for Port {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Port::Tcp(port) => write!(f, "{}/tcp", port),
            Port::Udp(port) => write!(f, "{}/udp", port),
        }
    }
}

pub struct ContainerBuilder {
    image: String,
    extra_args: Vec<String>,
    args: Vec<String>,
    environments: Vec<(String, String)>,
    ports: Vec<Port>,
    volumes: Vec<(String, String)>,
}

impl ContainerBuilder {
    pub fn new(image: impl Into<String>) -> Self {
        Self {
            image: image.into(),
            extra_args: vec![],
            args: vec![],
            environments: vec![],
            ports: vec![],
            volumes: vec![],
        }
    }

    pub fn with_volume<S, T>(self, orig: S, dest: T) -> Self
    where
        S: Into<String>,
        T: Into<String>,
    {
        let volumes = self
            .volumes
            .into_iter()
            .chain([(orig.into(), dest.into())])
            .collect();

        Self { volumes, ..self }
    }

    pub fn with_port(self, port: u16) -> Self {
        let mut ports = self.ports.clone();
        ports.push(Port::Tcp(port));

        Self { ports, ..self }
    }

    pub fn with_udp_port(self, port: u16) -> Self {
        let mut ports = self.ports.clone();
        ports.push(Port::Udp(port));

        Self { ports, ..self }
    }

    pub fn with_env<S>(self, key: S, value: S) -> Self
    where
        S: Into<String>,
    {
        let environments = self
            .environments
            .into_iter()
            .chain([(key.into(), value.into())])
            .collect();

        Self {
            environments,
            ..self
        }
    }

    pub fn with_extra_args<S, T>(self, args: T) -> Self
    where
        S: Into<String>,
        T: IntoIterator<Item = S>,
    {
        let docker_args = args.into_iter().map(Into::into).collect();

        Self {
            extra_args: docker_args,
            ..self
        }
    }

    pub fn args<S, T>(self, args: T) -> Self
    where
        S: Into<String>,
        T: IntoIterator<Item = S>,
    {
        let args = args.into_iter().map(Into::into).collect();

        Self { args, ..self }
    }

    pub fn run(self) -> std::io::Result<Container> {
        let environments = self
            .environments
            .into_iter()
            .flat_map(|(key, value)| ["-e".to_string(), format!("{}={}", key, value)]);
        let ports = self
            .ports
            .into_iter()
            .flat_map(|port| ["-p".to_string(), port.to_string()]);
        let volumes = self
            .volumes
            .into_iter()
            .flat_map(|(orig, dest)| ["-v".to_string(), format!("{}:{}", orig, dest)]);

        let args = [
            "run".to_string(),
            "-d".to_string(), // daemon
            "--rm".to_string(),
        ]
        .into_iter()
        .chain(environments)
        .chain(ports)
        .chain(volumes)
        .chain(self.extra_args)
        .chain([self.image.clone()])
        .chain(self.args);

        let output = Command::new("docker").args(args).output()?;
        let id = String::from_utf8(output.stdout)
            .map(|id| id.trim().to_string())
            .map_err(|err| std::io::Error::new(ErrorKind::Other, err))?;

        let cid = id.clone();
        let image = self.image.clone();
        std::thread::spawn(move || {
            // This will be deprecated
            #[allow(clippy::zombie_processes)]
            let child = Command::new("docker")
                .args(["logs", "-t", "-f", cid.as_str()])
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();

            let mut output = BufReader::new(child.stdout.unwrap()).lines();
            while let Some(Ok(line)) = output.next() {
                println!("{} | {}", image, line);
            }
        });

        Ok(Container(id))
    }
}

pub enum WaitFor<'a> {
    Stdout(&'a str),
    Stderr(&'a str),
}

pub struct Container(String);

impl Container {
    pub fn wait(&self, wait: WaitFor) -> std::io::Result<()> {
        match wait {
            WaitFor::Stdout(msg) => {
                let child = Command::new("docker")
                    .args(["logs", "-f", self.0.as_str()])
                    .stdout(Stdio::piped())
                    .spawn()?;

                wait_for(child.stdout.unwrap(), msg)
            }
            WaitFor::Stderr(msg) => {
                let child = Command::new("docker")
                    .args(["logs", "-f", self.0.as_str()])
                    .stderr(Stdio::piped())
                    .spawn()?;

                wait_for(child.stderr.unwrap(), msg)
            }
        }
    }

    pub fn get_tcp_port(&self, internal: u16) -> SocketAddr {
        #[derive(Debug, Deserialize)]
        struct Port {
            #[serde(rename = "HostIp")]
            host: String,
            #[serde(rename = "HostPort")]
            port: String,
        }

        let output = Command::new("docker")
            .args([
                "inspect",
                self.0.as_str(),
                "-f",
                "{{json .NetworkSettings.Ports }}",
            ])
            .output()
            .unwrap();

        let mut ports: HashMap<String, Option<Vec<Port>>> =
            serde_json::from_slice(&output.stdout).unwrap();

        let key = format!("{}/tcp", internal);
        let ports = ports.remove(&key).unwrap().unwrap();
        let port = ports.first().unwrap();

        format!("{}:{}", port.host, port.port)
            .parse::<SocketAddr>()
            .unwrap()
    }

    pub fn get_udp_port(&self, internal: u16) -> SocketAddr {
        #[derive(Debug, Deserialize)]
        struct Port {
            #[serde(rename = "HostIp")]
            host: String,
            #[serde(rename = "HostPort")]
            port: String,
        }

        let output = Command::new("docker")
            .args([
                "inspect",
                self.0.as_str(),
                "-f",
                "{{json .NetworkSettings.Ports }}",
            ])
            .output()
            .unwrap();

        let mut ports: HashMap<String, Option<Vec<Port>>> =
            serde_json::from_slice(&output.stdout).unwrap();

        let key = format!("{}/udp", internal);
        let ports = ports.remove(&key).unwrap().unwrap();
        let port = ports.first().unwrap();

        format!("{}:{}", port.host, port.port)
            .parse::<SocketAddr>()
            .unwrap()
    }

    pub fn get_mapped_addr(&self, internal: u16) -> SocketAddr {
        #[derive(Debug, Deserialize)]
        struct Port {
            #[serde(rename = "HostIp")]
            host: String,
            #[serde(rename = "HostPort")]
            port: String,
        }

        let output = Command::new("docker")
            .args([
                "inspect",
                self.0.as_str(),
                "-f",
                "{{json .NetworkSettings.Ports }}",
            ])
            .output()
            .unwrap();

        let mut ports: HashMap<String, Option<Vec<Port>>> =
            serde_json::from_slice(&output.stdout).unwrap();

        // try tcp first
        if let Some(Some(ports)) = ports.remove(&format!("{}/tcp", internal)) {
            let port = ports.first().unwrap(); // first is ipv4

            return format!("{}:{}", port.host, port.port)
                .parse::<SocketAddr>()
                .unwrap();
        }

        panic!("Failed to get mapped address");
    }
}

impl Drop for Container {
    fn drop(&mut self) {
        Command::new("docker")
            .args(["container", "kill", &self.0])
            .output()
            .expect("container kill failed");
    }
}

fn wait_for<T: Read>(reader: T, msg: &'_ str) -> std::io::Result<()> {
    let stream = BufReader::new(reader).lines();

    for line in stream {
        if line?.contains(msg) {
            return Ok(());
        }
    }

    Err(ErrorKind::NotFound.into())
}
