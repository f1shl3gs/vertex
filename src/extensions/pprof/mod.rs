#[cfg(feature = "jemalloc")]
mod heap;
mod profile;

use std::fmt::Debug;
use std::net::SocketAddr;
use std::sync::LazyLock;

use bytes::Bytes;
use configurable::configurable_component;
use framework::Extension;
use framework::config::{ExtensionConfig, ExtensionContext, Resource};
use framework::shutdown::ShutdownSignal;
use http::{Method, StatusCode};
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::{Request, Response, service::service_fn};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

const DEFAULT_PROFILE_SECONDS: u64 = 30;
const DEFAULT_FREQUENCY: u32 = 1000;

#[configurable_component(extension, name = "pprof")]
#[serde(deny_unknown_fields)]
struct Config {
    /// Which address the pprof server will listen
    #[configurable(required)]
    listen: SocketAddr,
}

#[async_trait::async_trait]
#[typetag::serde(name = "pprof")]
impl ExtensionConfig for Config {
    async fn build(&self, cx: ExtensionContext) -> crate::Result<Extension> {
        let listener = TcpListener::bind(self.listen).await?;

        Ok(Box::pin(run(listener, cx.shutdown)))
    }

    fn resources(&self) -> Vec<Resource> {
        vec![Resource::tcp(self.listen)]
    }
}

async fn run(listener: TcpListener, shutdown: ShutdownSignal) -> Result<(), ()> {
    let service = service_fn(move |req: Request<Incoming>| async {
        let resp = handle(req).await.unwrap_or_else(|err| {
            let (code, msg) = match err {
                Error::ParseQuery { .. } => (400, err.to_string()),
                err => (500, err.to_string()),
            };

            Response::builder()
                .status(code)
                .body(Full::<Bytes>::new(msg.into()))
                .unwrap()
        });

        Ok::<_, hyper::Error>(resp)
    });

    framework::http::serve(listener.into(), service)
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(|_err| ())
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error("parse {key} failed, {err}")]
    ParseQuery { key: &'static str, err: String },
    #[error("build profiler failed, {0}")]
    BuildProfiler(#[from] pprof::Error),
    // pprof doesn't re-export EncodeError, so Protobuf(EncodeError) might
    // broken building, cause our prost and pprof's prost is conflict.
    #[error("encode to protobuf data failed, {0}")]
    Protobuf(String),
    #[cfg(feature = "jemalloc")]
    #[error(transparent)]
    Heap(heap::HeapProfileError),
}

#[cfg(feature = "jemalloc")]
impl From<heap::HeapProfileError> for Error {
    fn from(err: heap::HeapProfileError) -> Self {
        Error::Heap(err)
    }
}

static PROFILE_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
#[cfg(feature = "jemalloc")]
static HEAP_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

async fn handle(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Error> {
    if req.method() != Method::GET {
        let resp = Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Full::default())
            .unwrap();
        return Ok(resp);
    }

    match req.uri().path() {
        #[cfg(feature = "jemalloc")]
        "/debug/pprof/allocs" => heap::allocs(req).await,
        #[cfg(feature = "jemalloc")]
        "/debug/pprof/heap" => match HEAP_MUTEX.try_lock() {
            Ok(guard) => {
                let result = heap::profile(req).await;
                drop(guard);
                result
            }
            Err(_err) => {
                let resp = Response::builder()
                    .status(StatusCode::TOO_MANY_REQUESTS)
                    .body(Full::default())
                    .unwrap();

                Ok(resp)
            }
        },
        "/debug/pprof/profile" => match PROFILE_MUTEX.try_lock() {
            Ok(guard) => {
                let result = profile::handle(req).await;
                drop(guard);
                result
            }
            Err(_err) => {
                let resp = Response::builder()
                    .status(StatusCode::TOO_MANY_REQUESTS)
                    .body(Full::default())
                    .unwrap();

                Ok(resp)
            }
        },
        _ => {
            let resp = Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Full::default())
                .unwrap();

            Ok(resp)
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
