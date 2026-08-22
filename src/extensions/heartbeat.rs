//! This extension sends a heartbeat signal via POST to an HTTP endpoint
//! on a regular interval. This is useful to keep track of existing Vertex
//! instances in a large deployment.

use std::time::Duration;

use bytes::Bytes;
use chrono::{DateTime, Utc};
use configurable::configurable_component;
use framework::Extension;
use framework::config::{ExtensionConfig, ExtensionContext};
use framework::http::{Auth, HttpClient};
use framework::tls::TlsConfig;
use http::{Method, Request, Uri};
use http_body_util::{BodyExt, Full};
use serde::Serialize;

const fn default_interval() -> Duration {
    Duration::from_secs(60)
}

#[configurable_component(extension, name = "heartbeat")]
struct Config {
    /// Unique identifier to submit for this instance
    ///
    /// Linux: /etc/machine-id -> random
    #[serde(default)]
    id: String,

    /// Interval for sending heartbeat messages
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    /// URL of heartbeat endpoint
    endpoint: String,

    /// Optional TLS settings
    tls: Option<TlsConfig>,

    auth: Option<Auth>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "heartbeat")]
impl ExtensionConfig for Config {
    async fn build(&self, cx: ExtensionContext) -> crate::Result<Extension> {
        let id = self.id.trim();
        let id = if !id.is_empty() {
            id.to_string()
        } else {
            std::fs::read_to_string("/etc/machine-id")?
        };

        let client = HttpClient::new(self.tls.as_ref(), &cx.proxy)?;

        let start_time = read_btime()? + (read_start_ticks()? / 100);
        let heartbeat = Heartbeat {
            id,
            start_time: DateTime::from_timestamp(start_time, 0).ok_or("invalid start time")?,
        };

        let auth = self.auth.clone();
        let endpoint = self.endpoint.parse::<Uri>()?;
        let mut ticker = tokio::time::interval(self.interval);
        let mut shutdown = cx.shutdown;

        Ok(Box::pin(async move {
            loop {
                tokio::select! {
                    _ = &mut shutdown => break,
                    _ = ticker.tick() => {}
                }

                send_heartbeat(&client, &endpoint, auth.as_ref(), &heartbeat).await;
            }

            Ok(())
        }))
    }
}

#[derive(Serialize)]
struct Heartbeat {
    id: String,
    start_time: DateTime<Utc>,
}

fn read_start_ticks() -> std::io::Result<i64> {
    let content = std::fs::read_to_string("/proc/self/stat")?;
    let start_ticks = content
        .split_whitespace()
        .nth(21)
        .ok_or(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "no stat",
        ))?;

    start_ticks.parse::<i64>().map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "failed to parse start ticks",
        )
    })
}

fn read_btime() -> std::io::Result<i64> {
    let content = std::fs::read_to_string("/proc/stat")?;
    for line in content.lines() {
        let Some(s) = line.strip_prefix("btime ") else {
            continue;
        };

        return s.parse::<i64>().map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "btime parse error")
        });
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "btime not found",
    ))
}

async fn send_heartbeat(
    client: &HttpClient,
    endpoint: &Uri,
    auth: Option<&Auth>,
    heartbeat: &Heartbeat,
) {
    let payload =
        Bytes::from(serde_json::to_vec(heartbeat).expect("failed to serialize heartbeat"));
    let mut req = match Request::builder()
        .method(Method::POST)
        .uri(endpoint)
        .body(Full::new(payload))
    {
        Ok(req) => req,
        Err(err) => {
            error!(message = "build heartbeat request failed", ?err);

            return;
        }
    };

    if let Some(auth) = auth {
        auth.apply(&mut req);
    }

    let resp = match client.send(req).await {
        Ok(resp) => resp,
        Err(err) => {
            warn!(message = "failed to send http request", ?err);
            return;
        }
    };

    if resp.status().is_success() {
        debug!(message = "send heartbeat success");
        return;
    }

    let (parts, incoming) = resp.into_parts();
    match incoming.collect().await {
        Ok(body) => {
            warn!(
                message = "send heartbeat failed",
                status = ?parts.status,
                body = std::str::from_utf8(body.to_bytes().as_ref())
                    .unwrap_or("failed to decode response")
            );
        }
        Err(err) => {
            warn!(
                message = "failed to read response from server",
                status = ?parts.status,
                ?err
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }
}
