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
    let service = make_service_fn(|_conn| async { Ok::<_, Error>(service_fn(handle)) });

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
    #[error("invalid seconds value: {0}")]
    Seconds(ParseIntError),
    #[error("invalid frequency value: {0}")]
    Frequency(ParseIntError),
    #[error("build profiler failed")]
    BuildProfiler(#[from] pprof::Error),
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
                report.flamegraph(&mut buf).unwrap();

                buf.into_inner().freeze()
            } else {
                // Generating protobuf content which can be processed with
                // `go tool pprof -svg xxx.pb`, which will generate a svg
                // too.
                //
                // Pyroscope will consume this kind of data too.
                let mut buf = BytesMut::new();
                let profile = report.pprof().unwrap();
                profile.encode(&mut buf).unwrap();

                buf.freeze()
            };

            Ok(Response::new(Body::from(buf)))
        }

        Err(err) => {
            error!(message = "Build report failed", ?err);

            let resp = Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
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
        crate::testing::test_generate_config::<Config>()
    }
}
