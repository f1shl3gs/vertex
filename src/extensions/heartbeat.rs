use std::collections::BTreeMap;
use std::fmt::Debug;
use std::net::Ipv4Addr;
use std::ops::Deref;
use std::time::Duration;

use async_trait::async_trait;
use chrono::SecondsFormat;
use configurable::configurable_component;
use framework::config::{ExtensionConfig, ExtensionContext, ProxyConfig, UriSerde};
use framework::http::HttpClient;
use framework::tls::{TlsConfig, TlsSettings};
use framework::{Extension, ShutdownSignal};
use http::Request;
use hyper::Body;
use nix::net::if_::InterfaceFlags;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::Serialize;
use tokio::select;

const fn default_interval() -> Duration {
    Duration::from_secs(60)
}

#[configurable_component(extension, name = "heartbeat")]
#[derive(Debug)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// POST some state of vertex to remote endpoint.
    /// then we can do much more, e.g. service discovery.
    #[configurable(required)]
    endpoint: UriSerde,

    tls: Option<TlsConfig>,

    /// Duration of each heartbeat sending.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    #[serde(default)]
    tags: BTreeMap<String, String>,
}

#[async_trait]
#[typetag::serde(name = "heartbeat")]
impl ExtensionConfig for Config {
    async fn build(&self, cx: ExtensionContext) -> framework::Result<Extension> {
        let mut status = VERTEX_STATUS.lock();

        status.uuid = get_uuid();
        status.uptime = chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, false);
        status.hostname = framework::hostname().unwrap();
        status.version = framework::get_version();
        status.lease = humanize::duration::duration(&(self.interval + Duration::from_secs(15)));
        status.os = sysinfo::os().unwrap_or_default();
        status.address = get_advertise_addr()?;
        status.kernel = sysinfo::kernel().unwrap_or_default();
        status.tags = self.tags.clone();

        let tls = TlsSettings::from_options(&self.tls)?;
        let client = HttpClient::new(tls, &ProxyConfig::default())?;

        Ok(Box::pin(run(
            self.endpoint.clone(),
            client,
            self.interval,
            cx.shutdown,
        )))
    }
}

#[derive(Debug, Serialize)]
struct Resource {
    name: String,
    component_type: String,
    address: String,
    port: u16,
}

#[derive(Default, Debug, Serialize)]
struct Status {
    uuid: String,
    hostname: String,
    version: String,
    address: String,
    lease: String, // 1 interval + 15 seconds
    os: String,
    uptime: String,
    tags: BTreeMap<String, String>,

    #[cfg(unix)]
    kernel: String,

    // component resources, for now it's address
    resources: Vec<Resource>,
}

static VERTEX_STATUS: Lazy<Mutex<Status>> = Lazy::new(Default::default);

pub fn report_config(config: &framework::config::Config) {
    let mut resources = vec![];

    for (key, ext) in &config.extensions {
        resources.extend(ext.resources().into_iter().filter_map(|r| match r {
            framework::config::Resource::Port(addr, _) => Some(Resource {
                name: key.to_string(),
                component_type: ext.component_name().to_string(),
                address: format!("{}", addr.ip()),
                port: addr.port(),
            }),

            _ => None,
        }));
    }

    for (key, source) in &config.sources {
        resources.extend(source.resources().into_iter().filter_map(|r| match r {
            framework::config::Resource::Port(addr, _) => Some(Resource {
                name: key.to_string(),
                component_type: source.component_name().to_string(),
                address: format!("{}", addr.ip()),
                port: addr.port(),
            }),

            _ => None,
        }));
    }

    for (key, sink) in &config.sinks {
        resources.extend(sink.resources(key).into_iter().filter_map(|r| match r {
            framework::config::Resource::Port(addr, _) => Some(Resource {
                name: key.to_string(),
                component_type: sink.component_name().to_string(),
                address: format!("{}", addr.ip()),
                port: addr.port(),
            }),

            _ => None,
        }));
    }

    VERTEX_STATUS.lock().resources = resources;
}

async fn run(
    endpoint: UriSerde,
    client: HttpClient,
    interval: Duration,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()> {
    loop {
        select! {
            _ = &mut shutdown => return Ok(()),
            _ = tokio::time::sleep(interval) => {}
        }

        let body = serde_json::to_string(VERTEX_STATUS.deref())
            .expect("status serialize should always success");

        let req = Request::post(&endpoint.uri).body(Body::from(body)).unwrap();

        match client.send(req).await {
            Ok(resp) => {
                if resp.status().as_u16() >= 300 {
                    warn!(message = "unexpected status code", code = ?resp.status());
                    continue;
                }

                debug!(message = "upload status successful")
            }
            Err(err) => {
                warn!(
                    message = "upload vertex status failed",
                    endpoint = endpoint.uri.to_string(),
                    ?err
                );

                continue;
            }
        }
    }
}

fn get_uuid() -> String {
    match sysinfo::machine_id() {
        Ok(id) => id,
        Err(err) => {
            if err.kind() != std::io::ErrorKind::NotFound {
                warn!(message = "cannot find /etc/machine-id, use random", ?err)
            }

            uuid::Uuid::new_v4().to_string()
        }
    }
}

fn get_advertise_addr() -> std::io::Result<String> {
    let ifaddrs = nix::ifaddrs::getifaddrs()?;

    let addrs = ifaddrs
        .filter_map(|addr| {
            if addr.flags.intersects(InterfaceFlags::IFF_LOOPBACK)
                || !addr.flags.intersects(InterfaceFlags::IFF_RUNNING)
            {
                return None;
            }

            let sockaddr = addr.address?.as_sockaddr_in()?.ip();
            let ne = u32::from_be(sockaddr);
            let ip = Ipv4Addr::new(
                (ne & 0xFF) as u8,
                ((ne >> 8) & 0xFF) as u8,
                ((ne >> 16) & 0xFF) as u8,
                (ne >> 24) as u8,
            );

            Some(ip.to_string())
        })
        .collect::<Vec<_>>();

    Ok(if addrs.is_empty() {
        "".to_string()
    } else {
        addrs[0].to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }
}
