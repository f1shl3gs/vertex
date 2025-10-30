use std::collections::{BTreeMap, HashMap};
use std::fmt::Display;
use std::path::PathBuf;
use std::time::Duration;

use bytes::Bytes;
use configurable::configurable_component;
use framework::config::{ExtensionConfig, ExtensionContext};
use framework::observe::{Endpoint, Observer};
use framework::{Extension, ShutdownSignal};
use futures::StreamExt;
use http::{Method, Request};
use http_body_util::{BodyExt, Full};
use hyper_unix::UnixConnector;
use hyper_util::rt::TokioExecutor;
use regex::Regex;
use serde::Deserialize;
use tokio::sync::watch::{Sender, channel};
use value::{Value, value};

fn default_path() -> PathBuf {
    PathBuf::from("/var/run/docker.sock")
}

fn default_timeout() -> Duration {
    Duration::from_secs(5)
}

#[configurable_component(extension, name = "docker")]
struct Config {
    /// The absolute path of docker socket
    #[serde(default = "default_path")]
    path: PathBuf,

    /// The list of container image names to exclude
    #[serde(default)]
    exclude_images: Vec<String>,

    /// Max amount of time to wait for a response form Docker API
    #[serde(default = "default_timeout", with = "humanize::duration::serde")]
    timeout: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "docker")]
impl ExtensionConfig for Config {
    async fn build(&self, cx: ExtensionContext) -> crate::Result<Extension> {
        let exclude_images = self
            .exclude_images
            .iter()
            .map(|pattern| Regex::new(pattern))
            .collect::<Result<Vec<_>, _>>()?;

        let connector = UnixConnector::new(self.path.clone());
        let client = hyper_util::client::legacy::Builder::new(TokioExecutor::new())
            .build::<_, Full<Bytes>>(connector);

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

type Client = hyper_util::client::legacy::Client<UnixConnector, Full<Bytes>>;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Container {
    id: String,
    image: String,
}

#[derive(Debug)]
enum Error {
    UnexpectedStatusCode(http::StatusCode),

    Hyper(hyper::Error),

    Client(hyper_util::client::legacy::Error),

    Deserialize(serde_json::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::UnexpectedStatusCode(code) => write!(f, "unexpected status code: {code}"),
            Error::Hyper(err) => err.fmt(f),
            Error::Client(err) => err.fmt(f),
            Error::Deserialize(err) => err.fmt(f),
        }
    }
}

/// Watch events
///
/// https://docs.docker.com/reference/api/engine/version/v1.51/#tag/System/operation/SystemEvents
async fn watch(client: Client, tx: Sender<()>, mut shutdown: ShutdownSignal) {
    async fn watch_inner(client: &Client, tx: &Sender<()>, shutdown: &mut ShutdownSignal) {
        let req = Request::builder()
            .method(Method::GET)
            // filters={"type":["container"],"event":["destroy","die","pause","rename","stop","start","unpause","update"]}
            .uri("http://localhost/events?filters=%7B%22type%22%3A%5B%22container%22%5D%2C%22event%22%3A%5B%22destroy%22%2C%22die%22%2C%22pause%22%2C%22rename%22%2C%22stop%22%2C%22start%22%2C%22unpause%22%2C%22update%22%5D%7D")
            .body(Full::<Bytes>::default())
            .unwrap();

        let resp = match client.request(req).await {
            Ok(resp) => resp,
            Err(err) => {
                warn!(message = "watch events failed", ?err);
                return;
            }
        };

        let (parts, incoming) = resp.into_parts();
        if !parts.status.is_success() {
            let data = incoming.collect().await.unwrap().to_bytes();

            warn!(
                message = "unexpected response status code",
                code = %parts.status,
                body = %String::from_utf8_lossy(&data)
            );

            return;
        }

        let mut stream = incoming.into_data_stream().take_until(shutdown);
        while let Some(Ok(_data)) = stream.next().await {
            if let Err(_err) = tx.send(()) {
                break;
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
    let containers = fetch::<Vec<Container>>(
        client,
        "http://localhost/containers/json?filters=%7B%22status%22%3A%5B%22running%22%5D%7D"
            .to_string(),
    )
    .await?;

    let mut endpoints = Vec::with_capacity(containers.len());
    for container in containers {
        if exclude_images
            .iter()
            .any(|re| re.is_match(&container.image))
        {
            continue;
        }

        let inspect = fetch::<Inspect>(
            client,
            format!("http://localhost/containers/{}/json", &container.id),
        )
        .await?;

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
            let details = value!({
                "name": name.clone(),
                "image": image.clone(),
                "tag": tag.clone(),
                "command": command.clone(),
                "hostname": hostname.clone(),
                "container_id": container_id.clone(),
                "transport": proto.to_string(),
                "labels": labels.clone(),
            });

            endpoints.push(Endpoint {
                id,
                typ: "docker".to_string(),
                target: format!("{host}:{port}"),
                details,
            });
        }
    }

    Ok(endpoints)
}

#[derive(Deserialize)]
struct Empty {}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct InspectConfig {
    hostname: String,
    image: String,

    #[serde(default)]
    cmd: Option<Vec<String>>,
    labels: HashMap<String, String>,
    exposed_ports: HashMap<String, Empty>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Network {
    #[serde(rename = "IPAddress")]
    ip_address: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct NetworkSettings {
    networks: HashMap<String, Network>,
    // ports: HashMap<String, Option<Vec<Port>>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Inspect {
    name: String,
    config: InspectConfig,
    network_settings: NetworkSettings,
}

async fn fetch<T: serde::de::DeserializeOwned>(client: &Client, uri: String) -> Result<T, Error> {
    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Full::<Bytes>::default())
        .unwrap();

    let resp = client.request(req).await.map_err(Error::Client)?;
    let (parts, incoming) = resp.into_parts();
    if !parts.status.is_success() {
        return Err(Error::UnexpectedStatusCode(parts.status));
    }

    let body = incoming.collect().await.map_err(Error::Hyper)?.to_bytes();

    serde_json::from_slice::<T>(&body).map_err(Error::Deserialize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }
}
