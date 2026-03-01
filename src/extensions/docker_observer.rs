use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::time::Duration;

use configurable::configurable_component;
use docker::containers::ListContainersOptions;
use docker::system::EventsOptions;
use docker::{Client, Error};
use framework::config::{ExtensionConfig, ExtensionContext};
use framework::observe::{Endpoint, Observer};
use framework::{Extension, ShutdownSignal};
use futures::StreamExt;
use regex::Regex;
use tokio::sync::watch::{Sender, channel};
use value::{Value, value};

fn default_path() -> PathBuf {
    PathBuf::from("/var/run/docker.sock")
}

fn default_timeout() -> Duration {
    Duration::from_secs(5)
}

#[configurable_component(extension, name = "docker_observer")]
struct Config {
    /// The absolute path of docker socket
    #[serde(default = "default_path")]
    path: PathBuf,

    /// A list of filters whose matching images are to be excluded. Supports literals and regex
    #[serde(default)]
    exclude_images: Vec<String>,

    /// Max amount of time to wait for a response form Docker API
    #[serde(default = "default_timeout", with = "humanize::duration::serde")]
    timeout: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "docker_observer")]
impl ExtensionConfig for Config {
    async fn build(&self, cx: ExtensionContext) -> crate::Result<Extension> {
        let exclude_images = self
            .exclude_images
            .iter()
            .map(|pattern| Regex::new(pattern))
            .collect::<Result<Vec<_>, _>>()?;
        let client = Client::new(self.path.clone());

        Ok(Box::pin(run(
            client,
            Duration::from_secs(60 * 60),
            self.timeout,
            exclude_images,
            Observer::register(cx.key),
            cx.shutdown,
        )))
    }
}

async fn run(
    client: Client,
    interval: Duration,
    timeout: Duration,
    exclude_images: Vec<Regex>,
    observer: Observer,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let (tx, mut rx) = channel::<()>(());
    tokio::spawn(watch(client.clone(), tx, shutdown.clone()));

    let mut ticker = tokio::time::interval(interval);
    loop {
        tokio::select! {
            _ = ticker.tick() => {}
            _ = &mut shutdown => break,
            _ = rx.changed() => {
                debug!(message = "events detected, refreshing");
            }
        }

        match tokio::time::timeout(timeout, list(&client, &exclude_images)).await {
            Ok(Ok(endpoints)) => {
                if let Err(err) = observer.publish(endpoints) {
                    warn!(message = "publish endpoints failed", ?err);
                }
            }
            Ok(Err(err)) => {
                warn!(message = "list containers failed", ?err);
            }
            Err(_err) => {
                warn!(message = "list containers timeout", ?timeout);
            }
        }
    }

    Ok(())
}

/// Watch events
///
/// https://docs.docker.com/reference/api/engine/version/v1.51/#tag/System/operation/SystemEvents
async fn watch(client: Client, tx: Sender<()>, mut shutdown: ShutdownSignal) {
    async fn watch_inner(client: &Client, tx: &Sender<()>, shutdown: &mut ShutdownSignal) {
        let mut filters = HashMap::new();
        filters.insert("type", vec!["container"]);
        filters.insert(
            "event",
            vec![
                "destroy", "die", "pause", "rename", "stop", "start", "unpause", "update",
            ],
        );
        let opts = EventsOptions {
            filters: Some(filters),
            ..Default::default()
        };

        match client.events(opts).await {
            Ok(stream) => {
                let mut stream = stream.take_until(shutdown);
                while let Some(Ok(_data)) = stream.next().await {
                    if let Err(_err) = tx.send(()) {
                        break;
                    }
                }
            }
            Err(err) => {
                warn!(message = "watch events failed", ?err);
            }
        }
    }

    loop {
        watch_inner(&client, &tx, &mut shutdown).await;

        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(10)) => continue,
            _ = &mut shutdown => break,
        }
    }
}

async fn list(client: &Client, exclude_images: &[Regex]) -> Result<Vec<Endpoint>, Error> {
    let mut filters = HashMap::new();
    filters.insert("status", vec!["running"]);
    let opts = ListContainersOptions {
        filters: Some(filters),
        ..Default::default()
    };

    let containers = match client.list_containers(opts).await {
        Ok(containers) => containers
            .into_iter()
            .filter(|c| !exclude_images.iter().any(|m| m.is_match(&c.image)))
            .collect::<Vec<_>>(),
        Err(err) => {
            return Err(err);
        }
    };

    let mut endpoints = Vec::with_capacity(containers.len());
    for container in containers {
        let inspect = match client.inspect_container(&container.id).await {
            Ok(inspect) => {
                if inspect.config.exposed_ports.is_empty() {
                    continue;
                }

                inspect
            }
            Err(err) => {
                warn!(
                    message = "inspect container failed",
                    id = container.id,
                    ?err
                );
                continue;
            }
        };

        if inspect.config.exposed_ports.is_empty() {
            continue;
        }

        let name = match inspect.name.strip_prefix('/') {
            Some(name) => name.to_string(),
            None => inspect.name,
        };
        let (image, tag) = match inspect.config.image.split_once(':') {
            None => (inspect.config.image, String::new()),
            Some((image, tag)) => (image.to_string(), tag.to_string()),
        };
        let command = inspect.config.cmd.unwrap_or_default().join(" ");
        let container_id = container.id;
        let hostname = inspect.config.hostname;
        let host = inspect
            .network_settings
            .networks
            .into_iter()
            .next()
            .map(|(_, network)| network.ip_address)
            .unwrap_or("127.0.0.1".to_string());

        let labels = inspect
            .config
            .labels
            .into_iter()
            .map(|(k, v)| (k, Value::from(v)))
            .collect::<BTreeMap<String, Value>>();

        for (exposed, _) in inspect.config.exposed_ports {
            let Some((port, proto)) = exposed.split_once('/') else {
                continue;
            };

            let id = format!("{}:{}", container_id, exposed);
            let port = port.parse::<u16>().unwrap_or_default();

            let details = value!({
                "name": name.clone(),
                "image": image.clone(),
                "tag": tag.clone(),
                "command": command.clone(),
                "hostname": hostname.clone(),
                "container_id": container_id.clone(),
                "transport": proto.to_string(),
                "labels": labels.clone(),
                "port": port,
            });

            endpoints.push(Endpoint {
                id,
                typ: "docker".into(),
                target: format!("{host}:{port}"),
                details,
            });
        }
    }

    Ok(endpoints)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }
}
