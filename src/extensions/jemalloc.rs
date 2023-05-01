#[cfg(not(feature = "jemalloc"))]
compile_error!("jemalloc-extension requires feature `jemalloc`");

use std::net::SocketAddr;
use std::num::ParseIntError;
use std::time::Duration;

use configurable::configurable_component;
use framework::config::{ExtensionConfig, ExtensionContext};
use framework::shutdown::ShutdownSignal;
use framework::Extension;
use futures::FutureExt;
use http::Request;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Response, Server, StatusCode};
use tikv_jemalloc_ctl::{stats, Access, AsName};

const DEFAULT_PROFILE_SECONDS: u64 = 30;

const OUTPUT: &str = "profile.out";
const PROF_ACTIVE: &[u8] = b"prof.active\0";
const PROF_DUMP: &[u8] = b"prof.dump\0";
const PROFILE_OUTPUT: &[u8] = b"profile.out\0";

fn default_listen() -> SocketAddr {
    "0.0.0.0:10911".parse().unwrap()
}

/// This extension integration `jemalloc-ctl`, you can dump the profiling
/// data and analyze it with `jeprof`.
#[configurable_component(extension, name = "jemalloc")]
#[derive(Debug)]
#[serde(deny_unknown_fields)]
struct Config {
    #[serde(default = "default_listen")]
    #[configurable(required)]
    pub listen: SocketAddr,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen: default_listen(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("MALLOC_CONF is not set, {source}")]
    EnvNotSet { source: std::env::VarError },

    #[error("MALLOC_CONF is set but prof is not enabled")]
    ProfileNotEnabled,
}

#[async_trait::async_trait]
#[typetag::serde(name = "jemalloc")]
impl ExtensionConfig for Config {
    async fn build(&self, ctx: ExtensionContext) -> crate::Result<Extension> {
        match std::env::var("MALLOC_CONF") {
            Ok(value) => {
                if !value.contains("prof:true") {
                    return Err(BuildError::ProfileNotEnabled.into());
                }
            }
            Err(err) => return Err(BuildError::EnvNotSet { source: err }.into()),
        }

        Ok(Box::pin(run(self.listen, ctx.shutdown)))
    }
}

async fn run(addr: SocketAddr, shutdown: ShutdownSignal) -> Result<(), ()> {
    let service = make_service_fn(|_conn| async { Ok::<_, Error>(service_fn(handler)) });

    let server = Server::bind(&addr)
        .serve(service)
        .with_graceful_shutdown(shutdown.map(|_| ()));
    if let Err(err) = server.await {
        warn!(
            message = "jemalloc profile server running failed",
            %err
        );
    }

    Ok(())
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("invalid seconds {0}")]
    Seconds(#[from] ParseIntError),
    #[error("jemalloc ctl error: {0}")]
    Jemalloc(#[from] tikv_jemalloc_ctl::Error),
}

async fn handler(req: Request<Body>) -> Result<Response<Body>, Error> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/stats") => {
            let allocated = stats::allocated::read()?;
            let active = stats::active::read()?;
            let metadata = stats::metadata::read()?;
            let resident = stats::resident::read()?;
            let mapped = stats::mapped::read()?;
            let retained = stats::retained::read()?;

            let body = format!(
                "allocated: {}\nactive: {}\nmetadata: {}\nresident: {}\nmapped: {}\nretained: {}\n",
                allocated, active, metadata, resident, mapped, retained
            );
            Ok(Response::new(Body::from(body)))
        }

        (&Method::GET, "/profile") => profiling(req).await,

        _ => {
            let mut resp = Response::new(Body::empty());
            *resp.status_mut() = StatusCode::NOT_FOUND;
            Ok(resp)
        }
    }
}

async fn profiling(req: Request<Body>) -> Result<Response<Body>, Error> {
    let mut seconds = DEFAULT_PROFILE_SECONDS;

    if let Some(query) = req.uri().query() {
        seconds = url::form_urlencoded::parse(query.as_ref())
            .into_iter()
            .find(|(k, _v)| k == "seconds")
            .map(|(_, v)| v.parse().map_err(Error::Seconds))
            .transpose()?
            .unwrap_or(DEFAULT_PROFILE_SECONDS);
    }

    set_prof_active(true);
    info!(message = "Starting jemalloc profile", seconds);
    tokio::time::sleep(Duration::from_secs(seconds)).await;
    set_prof_active(false);

    dump_profile();
    let data = std::fs::read(OUTPUT).expect("Read dumped profile failed");

    Ok(Response::new(Body::from(data)))
}

fn set_prof_active(active: bool) {
    let name = PROF_ACTIVE.name();
    name.write(active).expect("Should succeed to set profile");
}

fn dump_profile() {
    let name = PROF_DUMP.name();
    name.write(PROFILE_OUTPUT)
        .expect("Should succeed to dump profile")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }
}
