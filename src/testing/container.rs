use std::collections::HashMap;
use std::io::{BufRead, BufReader, ErrorKind, Read};
use std::process::{Command, Stdio};

use serde::Deserialize;

pub struct ContainerBuilder {
    image: String,
    args: Vec<String>,
    environments: Vec<(String, String)>,
    ports: Vec<u16>,
    volumes: Vec<(String, String)>,
}

impl ContainerBuilder {
    pub fn new(image: impl Into<String>) -> Self {
        Self {
            image: image.into(),
            args: vec![],
            environments: vec![],
            ports: vec![],
            volumes: vec![],
        }
    }

    pub fn with_volume<S>(self, orig: S, dest: S) -> Self
    where
        S: Into<String>,
    {
        let volumes = self
            .volumes
            .into_iter()
            .chain([(orig.into(), dest.into())])
            .collect();

        Self { volumes, ..self }
    }

    pub fn port(self, port: u16) -> Self {
        let mut ports = self.ports.clone();
        ports.push(port);

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
        .chain([self.image])
        .chain(self.args);

        let output = Command::new("docker").args(args).output()?;
        let id = String::from_utf8(output.stdout)
            .map(|id| id.trim().to_string())
            .map_err(|err| std::io::Error::new(ErrorKind::Other, err))?;

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

    pub fn get_host_port(&self, internal: u16) -> Option<String> {
        #[derive(Deserialize)]
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
        let ports = ports.remove(&key)??;
        let port = ports.first()?;

        Some(format!("{}:{}", port.host, port.port))
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
