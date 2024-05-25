use std::collections::BTreeMap;
use std::fmt::Debug;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
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

        status.uuid.clone_from(&UUID);
        status.uptime = chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, false);
        status.hostname = hostname::get().expect("get hostname failed");
        status.version = crate::get_version();
        status.lease = humanize::duration::duration(&(self.interval + Duration::from_secs(15)));
        status.os = sysinfo::os().unwrap_or_default();
        status.address = get_advertise_addr()?.to_string();
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

fn get_advertise_addr() -> std::io::Result<IpAddr> {
    let mut advertised = None;

    unsafe {
        let mut addrs = std::mem::MaybeUninit::<*mut libc::ifaddrs>::uninit();
        let ret = libc::getifaddrs(addrs.as_mut_ptr());
        if ret == -1 {
            panic!("{}", std::io::Error::last_os_error());
        }

        let base = addrs.assume_init();
        let mut next = addrs.assume_init();

        while let Some(addr) = next.as_ref() {
            if addr.ifa_flags & libc::IFF_LOOPBACK as libc::c_uint != 0 {
                next = addr.ifa_next;
                continue;
            }

            if addr.ifa_flags & libc::IFF_RUNNING as libc::c_uint == 0 {
                next = addr.ifa_next;
                continue;
            }

            if addr.ifa_addr.is_null() {
                next = addr.ifa_next;
                continue;
            }

            match (*addr.ifa_addr).sa_family as libc::c_int {
                libc::AF_INET => {
                    let sockaddr: libc::sockaddr_in =
                        std::ptr::read_unaligned(addr.ifa_addr as *const _);
                    let ip = Ipv4Addr::from(sockaddr.sin_addr.s_addr.to_ne_bytes());
                    advertised = Some(IpAddr::V4(ip));
                    break;
                }
                libc::AF_INET6 => {
                    let sockaddr: libc::sockaddr_in6 =
                        std::ptr::read_unaligned(addr.ifa_addr as *const _);
                    let ip = Ipv6Addr::from(sockaddr.sin6_addr.s6_addr);
                    advertised = Some(IpAddr::V6(ip));
                    break;
                }
                _ => {
                    next = addr.ifa_next;
                    continue;
                }
            }
        }

        libc::freeifaddrs(base);
    }

    match advertised {
        Some(addr) => Ok(addr),
        None => Err(std::io::Error::new(
            std::io::ErrorKind::AddrNotAvailable,
            "cannot find a valid addr",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }
}
