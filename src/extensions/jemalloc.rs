#[cfg(not(feature = "jemalloc"))]
compile_error!("jemalloc-extension requires feature `jemalloc`");

use std::env::temp_dir;
use std::net::SocketAddr;
use std::num::ParseIntError;
use std::sync::OnceLock;
use std::time::Duration;

use chrono::Utc;
use configurable::configurable_component;
use framework::config::{ExtensionConfig, ExtensionContext};
use framework::shutdown::ShutdownSignal;
use framework::Extension;
use futures::FutureExt;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use tikv_jemalloc_ctl::stats;
use tokio::sync::Mutex;

const DEFAULT_PROFILE_SECONDS: u64 = 30;

// C string should end with a '\0'.
const PROF_ACTIVE: &[u8] = b"prof.active\0";
const PROF_DUMP: &[u8] = b"prof.dump\0";

static PROFILE_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

fn default_listen() -> SocketAddr {
    "0.0.0.0:10911".parse().unwrap()
}

/// This extension integration `jemalloc-ctl`, you can dump the profiling data and
/// analyze it with `jeprof`. Environment MALLOC_CONF="prof:true" must be set before
/// you start profiling.
///
/// Once you start Vertex, you can get profile by running
/// `wget http://127.0.0.0:10911/profile` to get profile data, then you can analyze
/// it by `jeprof --show_bytes --svg <path_to_binary> ./profile > ./profile.svg`.
#[configurable_component(extension, name = "jemalloc")]
#[serde(deny_unknown_fields)]
struct Config {
    #[serde(default = "default_listen")]
    #[configurable(required)]
    listen: SocketAddr,
}

#[async_trait::async_trait]
#[typetag::serde(name = "jemalloc")]
impl ExtensionConfig for Config {
    async fn build(&self, ctx: ExtensionContext) -> crate::Result<Extension> {
        match std::env::var("MALLOC_CONF") {
            Ok(value) => {
                if !value.contains("prof:true") {
                    return Err("MALLOC_CONF is set, but \"prof\" is not enabled".into());
                }
            }
            Err(_err) => return Err("MALLOC_CONF is not set".into()),
        }

        Ok(Box::pin(run(self.listen, ctx.shutdown)))
    }
}

async fn run(addr: SocketAddr, shutdown: ShutdownSignal) -> Result<(), ()> {
    let service = make_service_fn(|_conn| async { Ok::<_, Error>(service_fn(handler)) });
    let server = Server::bind(&addr)
        .serve(service)
        .with_graceful_shutdown(shutdown.map(|_| ()));

    info!(message = "start http server", ?addr);
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
    #[error("read profile data failed, {0}")]
    ReadProfile(#[from] std::io::Error),
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

        (&Method::GET, "/profile") => {
            let mutex = PROFILE_MUTEX.get_or_init(Default::default);
            match mutex.try_lock() {
                Ok(_guard) => profiling(req).await,
                Err(_err) => {
                    let mut resp = Response::new(Body::from("Already in Profiling"));
                    *resp.status_mut() = StatusCode::TOO_MANY_REQUESTS;
                    Ok(resp)
                }
            }
        }

        _ => {
            let mut resp = Response::new(Body::empty());
            *resp.status_mut() = StatusCode::NOT_FOUND;
            Ok(resp)
        }
    }
}

async fn profiling(req: Request<Body>) -> Result<Response<Body>, Error> {
    let seconds = match req.uri().query() {
        Some(value) => url::form_urlencoded::parse(value.as_ref())
            .into_iter()
            .find(|(k, _v)| k == "seconds")
            .map(|(_, v)| v.parse().map_err(Error::Seconds))
            .transpose()?
            .unwrap_or(DEFAULT_PROFILE_SECONDS),
        None => DEFAULT_PROFILE_SECONDS,
    };

    set_prof_active(true)?;
    info!(message = "starting jemalloc profile", seconds);
    tokio::time::sleep(Duration::from_secs(seconds)).await;
    set_prof_active(false)?;

    dump_profile().map(|data| Response::new(Body::from(data)))
}

fn set_prof_active(active: bool) -> Result<(), Error> {
    unsafe {
        tikv_jemalloc_ctl::raw::update(PROF_ACTIVE, active)?;
    }

    Ok(())
}

fn dump_profile() -> Result<Vec<u8>, Error> {
    // random string is not a good option
    let filename = format!("vertex_mem_{}.prof", Utc::now().timestamp());

    let path = temp_dir().join(filename);
    let mut bytes = std::ffi::CString::new(path.to_str().unwrap())
        .expect("build CString")
        .into_bytes_with_nul();
    unsafe {
        let ptr = bytes.as_mut_ptr() as *mut libc::c_char;
        tikv_jemalloc_ctl::raw::write(PROF_DUMP, ptr)?;
    }

    std::fs::read(path).map_err(Error::ReadProfile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }
}
