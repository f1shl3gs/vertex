use std::convert::Infallible;
use std::net::SocketAddr;

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

#[configurable_component(extension, name = "pprof")]
#[derive(Debug)]
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
    let service = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });

    let server = Server::bind(&addr)
        .serve(service)
        .with_graceful_shutdown(shutdown.map(|_| ()));
    if let Err(e) = server.await {
        error!("pprof serve failed: {}", e)
    }

    Ok(())
}

async fn handle(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let mut seconds = DEFAULT_PROFILE_SECONDS;
    let mut flamegraph = false;

    if let Some(query) = req.uri().query() {
        url::form_urlencoded::parse(query.as_bytes())
            .into_owned()
            .for_each(|(k, v)| {
                if k == "seconds" {
                    seconds = v.parse().unwrap_or(DEFAULT_PROFILE_SECONDS);
                } else if k == "flamegraph" && v == "true" {
                    flamegraph = true;
                }
            });
    }

    let guard = pprof::ProfilerGuard::new(100).unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(seconds)).await;

    match guard.report().build() {
        Ok(report) => {
            let buf = if flamegraph {
                let mut buf = BytesMut::with_capacity(4 * 1024).writer();
                report.flamegraph(&mut buf).unwrap();

                buf.into_inner().freeze()
            } else {
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
