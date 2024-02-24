use std::collections::BTreeMap;
use std::fmt::Debug;
use std::ops::Deref;
use std::time::Duration;

use async_trait::async_trait;
use chrono::SecondsFormat;
use configurable::configurable_component;
use framework::config::{ExtensionConfig, ExtensionContext, ProxyConfig, UriSerde};
use framework::http::HttpClient;
use framework::tls::TlsConfig;
use framework::{Extension, ShutdownSignal};
use http::Request;
use hyper::Body;
use nix::net::if_::InterfaceFlags;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::Serialize;

static UUID: Lazy<String> = Lazy::new(|| uuid::Uuid::new_v4().to_string());

const fn default_interval() -> Duration {
    Duration::from_secs(60)
}

#[configurable_component(extension, name = "heartbeat")]
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
        let mut status = STATUS.lock();

        status.uuid = UUID.clone();
        status.uptime = chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, false);
        status.hostname = hostname::get().expect("get hostname failed");
        status.version = crate::get_version();
        status.lease = humanize::duration::duration(&(self.interval + Duration::from_secs(15)));
        status.os = sysinfo::os().unwrap_or_default();
        status.address = get_advertise_addr()?;
        status.kernel = sysinfo::kernel().unwrap_or_default();
        status.tags = self.tags.clone();

        let client = HttpClient::new(&self.tls, &ProxyConfig::default())?;

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

static STATUS: Lazy<Mutex<Status>> = Lazy::new(Default::default);

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

    STATUS.lock().resources = resources;
}

async fn run(
    endpoint: UriSerde,
    client: HttpClient,
    interval: Duration,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let mut ticker = tokio::time::interval(interval);

    loop {
        tokio::select! {
            _ = &mut shutdown => return Ok(()),
            _ = ticker.tick() => {}
        }

        let body = serde_json::to_string(STATUS.lock().deref())
            .expect("status serialize should always success");

        let req = Request::post(&endpoint.uri)
            .body(Body::from(body))
            .expect("should build POST request");

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
            Some(sockaddr.to_string())
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
