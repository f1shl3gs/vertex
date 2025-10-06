use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use bytes::Bytes;
use configurable::configurable_component;
use event::{Metric, tags};
use framework::config::{OutputType, SourceConfig, SourceContext, default_interval};
use framework::{Pipeline, ShutdownSignal, Source};
use http::{Method, Request};
use http_body_util::{BodyExt, Full};
use hyper_unix::UnixConnector;
use hyper_util::rt::TokioExecutor;
use serde::Deserialize;

fn default_endpoint() -> PathBuf {
    PathBuf::from("/run/podman/podman.sock")
}

const fn default_timeout() -> Duration {
    Duration::from_secs(5)
}

fn default_api_version() -> String {
    "v3.2.0".to_string()
}

#[configurable_component(source, name = "podman")]
struct Config {
    /// Address to reach the desired Podman daemon.
    #[serde(default = "default_endpoint")]
    endpoint: PathBuf,

    /// API version of the Podman
    #[serde(default = "default_api_version")]
    api_version: String,

    /// The maximum amount of time to wait for Podman API responses
    #[serde(default = "default_timeout", with = "humanize::duration::serde")]
    timeout: Duration,

    /// The interval at which to gather container stats.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "podman")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let client = Client::new(self.endpoint.clone(), self.api_version.clone());
        let interval = self.interval;
        let timeout = self.timeout;
        let shutdown = cx.shutdown;
        let output = cx.output;

        Ok(Box::pin(run(client, interval, timeout, shutdown, output)))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::metric()]
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

async fn run(
    client: Client,
    interval: Duration,
    timeout: Duration,
    mut shutdown: ShutdownSignal,
    mut output: Pipeline,
) -> Result<(), ()> {
    let mut ticker = tokio::time::interval(interval);

    loop {
        tokio::select! {
            _ = ticker.tick() => {},
            _ = &mut shutdown => break,
        }

        let start = Instant::now();
        let result = tokio::time::timeout(timeout, gather(&client)).await;
        let elapsed = start.elapsed();

        match result {
            Ok(Ok(metrics)) => {
                if let Err(_err) = output.send(metrics).await {
                    break;
                }
            }
            Ok(Err(err)) => {
                warn!(message = "scrape metrics failed", ?err, ?elapsed);
            }
            Err(_) => {
                // timeout
                debug!(message = "scrape timed out", ?elapsed);
            }
        }
    }

    Ok(())
}

async fn gather(client: &Client) -> Result<Vec<Metric>, Error> {
    let containers = client.list_container().await?;

    let ids = containers.iter().map(|c| c.id.as_str()).collect::<Vec<_>>();

    let stats = client.stats(&ids).await?;
    let mut metrics = Vec::with_capacity(stats.len() * 16);
    for stat in stats {
        let tags = tags!(
            "id" => stat.container_id.clone(),
            // "image" =>
            "name" => stat.name.clone(),
        );

        metrics.extend([
            Metric::sum_with_tags(
                "podman_container_cpu_system_seconds_total",
                "System CPU usage",
                stat.cpu_system_nano as f64 / 1_000_000_000.0,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "podman_container_cpu_seconds_total",
                "Total CPU time consumed",
                stat.cpu_nano as f64 / 1_000_000_000.0,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "podman_container_cpu_percent",
                "Percent of CPU used by the container",
                stat.cpu,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "podman_container_memory_limit",
                "Memory limit of the container",
                stat.mem_limit,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "podman_container_memory_usage",
                "Memory usage of the container",
                stat.mem_usage,
                tags.clone(),
            ),
            Metric::gauge_with_tags(
                "podman_container_memory_percent",
                "Percentage of memory used",
                stat.mem_perc,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "podman_container_blockio_read_bytes",
                "Number of bytes transferred from the disk by the container",
                stat.block_input,
                tags.clone(),
            ),
            Metric::sum_with_tags(
                "podman_container_blockio_write_bytes",
                "Number of bytes transferred from the disk by the container",
                stat.block_output,
                tags,
            ),
        ]);

        // per cpu usage
        if let Some(per_cpu) = stat.per_cpu {
            for (core, usage) in per_cpu.into_iter().enumerate() {
                metrics.push(Metric::sum_with_tags(
                    "podman_container_percpu_seconds_total",
                    "Total CPU time consumed per CPU-core",
                    usage,
                    tags!(
                        "core" => core,
                        "id" => stat.container_id.clone(),
                        // "image" =>
                        "name" => stat.name.clone(),
                    ),
                ));
            }
        }

        // network
        for (interface, network) in stat.network {
            let tags = tags!(
                "id" => stat.container_id.clone(),
                // "image" =>
                "name" => stat.name.clone(),
                "interface" => interface,
            );

            metrics.extend([
                Metric::sum_with_tags(
                    "podman_container_network_receive_bytes",
                    "Network receive bytes",
                    network.rx_bytes,
                    tags.clone(),
                ),
                Metric::sum_with_tags(
                    "podman_container_network_receive_dropped_bytes",
                    "Network receive dropped bytes",
                    network.rx_dropped,
                    tags.clone(),
                ),
                Metric::sum_with_tags(
                    "podman_container_network_receive_errors",
                    "Network receive errors",
                    network.rx_errors,
                    tags.clone(),
                ),
                Metric::sum_with_tags(
                    "podman_container_network_receive_packets",
                    "Network receive packets",
                    network.rx_packets,
                    tags.clone(),
                ),
                Metric::sum_with_tags(
                    "podman_container_network_sent_bytes",
                    "Network sent bytes",
                    network.tx_bytes,
                    tags.clone(),
                ),
                Metric::sum_with_tags(
                    "podman_container_network_sent_dropped_bytes",
                    "Network sent dropped bytes",
                    network.tx_dropped,
                    tags.clone(),
                ),
                Metric::sum_with_tags(
                    "podman_container_network_sent_errors",
                    "Network sent errors",
                    network.tx_errors,
                    tags.clone(),
                ),
                Metric::sum_with_tags(
                    "podman_container_network_sent_packets",
                    "Network sent packets",
                    network.tx_packets,
                    tags.clone(),
                ),
            ]);
        }
    }

    Ok(metrics)
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("unexpected response status {0}")]
    UnexpectedStatus(http::StatusCode),

    #[error(transparent)]
    Hyper(hyper::Error),

    #[error(transparent)]
    Request(hyper_util::client::legacy::Error),

    #[error(transparent)]
    Deserialize(serde_json::Error),

    #[error(transparent)]
    Api(StatsError),
}

impl From<hyper_util::client::legacy::Error> for Error {
    fn from(err: hyper_util::client::legacy::Error) -> Self {
        Self::Request(err)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Container {
    id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Network {
    rx_bytes: u64,
    rx_dropped: u64,
    rx_errors: u64,
    rx_packets: u64,
    tx_bytes: u64,
    tx_dropped: u64,
    tx_errors: u64,
    tx_packets: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Stat {
    name: String,
    #[serde(rename = "ContainerID")]
    container_id: String,

    // #[serde(rename = "AvgCPU")]
    // avg_cpu: f64,
    #[serde(rename = "CPU")]
    cpu: f64,
    #[serde(rename = "CPUNano")]
    cpu_nano: u64,
    #[serde(rename = "CPUSystemNano")]
    cpu_system_nano: u64,
    #[serde(rename = "PerCPU")]
    per_cpu: Option<Vec<u64>>,

    mem_limit: u64,
    mem_perc: f64,
    mem_usage: u64,

    network: HashMap<String, Network>,

    block_input: u64,
    block_output: u64,
    // #[serde(rename = "UpTime")]
    // uptime: u64,
}

#[derive(Debug, Deserialize)]
struct StatsError {
    cause: String,
    message: String,
    response: i64,
}

impl Display for StatsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.response, self.cause, self.message)
    }
}

impl std::error::Error for StatsError {}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct StatsResp {
    error: Option<StatsError>,
    stats: Vec<Stat>,
}

struct Client {
    http: hyper_util::client::legacy::Client<UnixConnector, Full<Bytes>>,
    version: String,
}

impl Client {
    fn new(endpoint: PathBuf, version: String) -> Self {
        let connector = UnixConnector::new(endpoint);
        let http = hyper_util::client::legacy::Builder::new(TokioExecutor::new())
            .build::<_, Full<Bytes>>(connector);

        Self { http, version }
    }

    /// https://docs.podman.io/en/latest/_static/api.html?version=v5.6#tag/containers/operation/ContainerListLibpod
    async fn list_container(&self) -> Result<Vec<Container>, Error> {
        let req = Request::builder()
            .method(Method::GET)
            // filters={"status":["running"]}
            .uri(format!("http://d/{}/libpod/containers/json?filters=%7B%22status%22%3A%5B%22running%22%5D%7D", self.version))
            .body(Full::<Bytes>::default())
            .unwrap();

        let resp = self.http.request(req).await?;
        let (parts, incoming) = resp.into_parts();
        if !parts.status.is_success() {
            return Err(Error::UnexpectedStatus(parts.status));
        }

        let body = incoming.collect().await.map_err(Error::Hyper)?.to_bytes();

        serde_json::from_slice(&body).map_err(Error::Deserialize)
    }

    /// https://docs.podman.io/en/latest/_static/api.html?version=v5.6#tag/containers/operation/ContainersStatsAllLibpod
    async fn stats(&self, ids: &[&str]) -> Result<Vec<Stat>, Error> {
        let containers = ids
            .iter()
            .map(|id| format!("containers={}", id))
            .collect::<Vec<_>>()
            .join("&");

        let req = Request::builder()
            .method(Method::GET)
            .uri(format!(
                "http://d/{}/libpod/containers/stats?stream=false&{}",
                self.version, containers
            ))
            .body(Full::<Bytes>::default())
            .unwrap();

        let resp = self.http.request(req).await?;
        let (parts, incoming) = resp.into_parts();
        if !parts.status.is_success() {
            return Err(Error::UnexpectedStatus(parts.status));
        }

        let body = incoming.collect().await.map_err(Error::Hyper)?.to_bytes();

        let resp = serde_json::from_slice::<StatsResp>(&body).map_err(Error::Deserialize)?;
        if let Some(err) = resp.error {
            return Err(Error::Api(err));
        }

        Ok(resp.stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }
}
