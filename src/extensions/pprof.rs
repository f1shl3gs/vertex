use std::fmt::Debug;
use std::net::SocketAddr;
use std::num::ParseIntError;

use bytes::{BufMut, BytesMut};
use configurable::configurable_component;
use framework::config::{ExtensionConfig, ExtensionContext};
use framework::shutdown::ShutdownSignal;
use framework::Extension;
use futures::FutureExt;
use http::StatusCode;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use pprof::protos::Message;

const DEFAULT_PROFILE_SECONDS: u64 = 30;
const DEFAULT_FREQUENCY: u32 = 1000;

#[configurable_component(extension, name = "pprof")]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Which address the pprof server will listen
    #[configurable(required)]
    pub listen: SocketAddr,
}

#[async_trait::async_trait]
#[typetag::serde(name = "pprof")]
impl ExtensionConfig for Config {
    async fn build(&self, cx: ExtensionContext) -> crate::Result<Extension> {
        Ok(Box::pin(run(self.listen, cx.shutdown)))
    }
}

async fn run(addr: SocketAddr, shutdown: ShutdownSignal) -> Result<(), ()> {
    let service = make_service_fn(|_conn| async {
        use std::convert::Infallible;

        Ok::<_, Infallible>(service_fn(|req: Request<Body>| async {
            let resp = match handle(req).await {
                Ok(resp) => resp,
                Err(err) => {
                    let (code, msg) = match err {
                        Error::Seconds(_) | Error::Frequency(_) => (400, err.to_string()),
                        err => (500, err.to_string()),
                    };

                    Response::builder()
                        .status(code)
                        .body(Body::from(msg))
                        .unwrap()
                }
            };

            Ok::<_, Infallible>(resp)
        }))
    });

    let server = Server::bind(&addr)
        .serve(service)
        .with_graceful_shutdown(shutdown.map(|_| ()));
    if let Err(e) = server.await {
        error!("pprof serve failed: {}", e)
    }

    Ok(())
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("parse seconds failed: {0}")]
    Seconds(ParseIntError),
    #[error("parse frequency failed: {0}")]
    Frequency(ParseIntError),
    #[error("build profiler failed, {0}")]
    BuildProfiler(#[from] pprof::Error),
    // pprof doesn't re-export EncodeError, so Protobuf(EncodeError) might
    // broken building, cause our prost and pprof's prost is conflict.
    #[error("encode to protobuf data failed, {0}")]
    Protobuf(String),
}

async fn handle(req: Request<Body>) -> Result<Response<Body>, Error> {
    let mut seconds = DEFAULT_PROFILE_SECONDS;
    let mut frequency = DEFAULT_FREQUENCY;
    let mut flamegraph = false;

    if let Some(query) = req.uri().query() {
        url::form_urlencoded::parse(query.as_bytes())
            .into_iter()
            .try_for_each::<_, Result<(), Error>>(|(k, v)| {
                match k.as_ref() {
                    "seconds" => seconds = v.parse().map_err(Error::Seconds)?,
                    "frequency" => frequency = v.parse().map_err(Error::Frequency)?,
                    "flamegraph" => flamegraph = v == "true",
                    _ => {}
                }

                Ok(())
            })?;
    }

    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(frequency as i32)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .map_err(Error::BuildProfiler)?;

    tokio::time::sleep(std::time::Duration::from_secs(seconds)).await;

    match guard.report().build() {
        Ok(report) => {
            let buf = if flamegraph {
                let mut buf = BytesMut::new().writer();
                report.flamegraph(&mut buf)?;

                buf.into_inner().freeze()
            } else {
                // Generating protobuf content which can be processed with
                // `go tool pprof -svg xxx.pb`, which will generate a svg
                // too.
                //
                // Pyroscope will consume this kind of data too.
                let mut buf = BytesMut::new();
                let profile = report.pprof()?;
                profile
                    .encode(&mut buf)
                    .map_err(|err| Error::Protobuf(err.to_string()))?;

                buf.freeze()
            };

            Ok(Response::new(Body::from(buf)))
        }

        Err(err) => {
            error!(message = "Build report failed", ?err);

            let resp = Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .expect("should build 500 response");

            Ok(resp)
        }
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
