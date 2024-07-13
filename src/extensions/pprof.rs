use std::fmt::Debug;
use std::net::SocketAddr;
use std::num::ParseIntError;

use bytes::{BufMut, Bytes, BytesMut};
use configurable::configurable_component;
use framework::config::{ExtensionConfig, ExtensionContext};
use framework::shutdown::ShutdownSignal;
use framework::Extension;
use http::StatusCode;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::{service::service_fn, Request, Response};
use hyper_util::rt::TokioIo;
use pprof::protos::Message;
use tokio::net::TcpListener;

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
        let listener = TcpListener::bind(self.listen).await?;

        Ok(Box::pin(run(listener, cx.shutdown)))
    }
}

async fn run(listener: TcpListener, mut shutdown: ShutdownSignal) -> Result<(), ()> {
    let service = service_fn(move |req: Request<Incoming>| async {
        let resp = handle(req).await.unwrap_or_else(|err| {
            let (code, msg) = match err {
                Error::Seconds(_) | Error::Frequency(_) => (400, err.to_string()),
                err => (500, err.to_string()),
            };

            Response::builder()
                .status(code)
                .body(Full::<Bytes>::new(msg.into()))
                .unwrap()
        });

        Ok::<_, hyper::Error>(resp)
    });

    loop {
        let conn = tokio::select! {
            _ = &mut shutdown => break,
            result = listener.accept() => match result {
                Ok((conn, _peer)) => TokioIo::new(conn),
                Err(err) => {
                    error!(
                        message = "accept new connection failed",
                        %err
                    );

                    continue
                }
            }
        };

        tokio::spawn(async move {
            if let Err(err) = http1::Builder::new().serve_connection(conn, service).await {
                error!(message = "handle http connection failed", ?err);
            }
        });
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

async fn handle(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Error> {
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
            let data = if flamegraph {
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

            Ok(Response::new(Full::new(data)))
        }

        Err(err) => {
            error!(message = "Build report failed", ?err);

            let resp = Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::default())
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
