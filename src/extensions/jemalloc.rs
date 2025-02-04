#[cfg(not(feature = "jemalloc"))]
compile_error!("jemalloc-extension requires feature `jemalloc`");

use std::env::temp_dir;
use std::net::SocketAddr;
use std::num::ParseIntError;
use std::sync::Mutex;
use std::time::Duration;

use bytes::Bytes;
use chrono::Utc;
use configurable::configurable_component;
use framework::config::{ExtensionConfig, ExtensionContext};
use framework::shutdown::ShutdownSignal;
use framework::tls::MaybeTlsListener;
use framework::Extension;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tikv_jemalloc_ctl::stats;
use tokio::net::TcpListener;

const DEFAULT_PROFILE_SECONDS: u64 = 30;

// C string should end with a '\0'.
const PROF_ACTIVE: &[u8] = b"prof.active\0";
const PROF_DUMP: &[u8] = b"prof.dump\0";

static PROFILE_MUTEX: Mutex<()> = Mutex::new(());

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
    async fn build(&self, cx: ExtensionContext) -> crate::Result<Extension> {
        match std::env::var("MALLOC_CONF") {
            Ok(value) => {
                if !value.contains("prof:true") {
                    return Err("MALLOC_CONF is set, but \"prof\" is not enabled".into());
                }
            }
            Err(_err) => return Err("MALLOC_CONF is not set".into()),
        }

        let listener = TcpListener::bind(self.listen).await?;
        let shutdown = cx.shutdown;

        Ok(Box::pin(
            framework::http::serve(listener.into(), service_fn(handler))
                .with_graceful_shutdown(shutdown)
                .await
                .map_err(|_err| ()),
        ))
    }
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

async fn handler(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Error> {
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

            Ok(Response::new(Full::new(Bytes::from(body))))
        }

        (&Method::GET, "/profile") => match PROFILE_MUTEX.try_lock() {
            Ok(_guard) => profiling(req).await,
            Err(_err) => {
                let resp = Response::builder()
                    .status(StatusCode::TOO_MANY_REQUESTS)
                    .body(Full::new(Bytes::from("Already in profiling")))
                    .unwrap();

                Ok(resp)
            }
        },

        _ => {
            let resp = Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::from("Not found")))
                .unwrap();

            Ok(resp)
        }
    }
}

async fn profiling(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Error> {
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

    dump_profile().map(|data| Response::new(Full::from(data)))
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
        crate::testing::generate_config::<Config>()
    }
}
