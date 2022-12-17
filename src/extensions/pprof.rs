use std::convert::Infallible;
use std::net::SocketAddr;

use bytes::{BufMut, BytesMut};
use framework::config::{ExtensionConfig, ExtensionContext, ExtensionDescription, GenerateConfig};
use framework::shutdown::ShutdownSignal;
use framework::Extension;
use futures::FutureExt;
use http::StatusCode;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use pprof::protos::Message;
use serde::{Deserialize, Serialize};

const DEFAULT_PROFILE_SECONDS: u64 = 30;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PprofConfig {
    pub listen: SocketAddr,
}

#[async_trait::async_trait]
#[typetag::serde(name = "pprof")]
impl ExtensionConfig for PprofConfig {
    async fn build(&self, cx: ExtensionContext) -> crate::Result<Extension> {
        Ok(Box::pin(run(self.listen, cx.shutdown)))
    }

    fn extension_type(&self) -> &'static str {
        "pprof"
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

impl GenerateConfig for PprofConfig {
    fn generate_config() -> String {
        r#"
# Which address the pprof server will listen
listen: 0.0.0.0:10910
"#
        .into()
    }
}

inventory::submit! {
    ExtensionDescription::new::<PprofConfig>("pprof")
}
