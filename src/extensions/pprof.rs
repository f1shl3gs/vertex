use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;

use framework::config::{ExtensionConfig, ExtensionContext, ExtensionDescription, GenerateConfig};
use framework::shutdown::ShutdownSignal;
use framework::Extension;
use futures::FutureExt;
use http::StatusCode;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PProfConfig {
    pub listen: SocketAddr,
}

impl Default for PProfConfig {
    fn default() -> Self {
        Self {
            listen: "0.0.0.0:10910".parse().unwrap(),
        }
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "pprof")]
impl ExtensionConfig for PProfConfig {
    async fn build(&self, ctx: ExtensionContext) -> crate::Result<Extension> {
        Ok(Box::pin(run(self.listen, ctx.shutdown)))
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
    let params: HashMap<String, String> = req
        .uri()
        .query()
        .map(|v| {
            url::form_urlencoded::parse(v.as_bytes())
                .into_owned()
                .collect()
        })
        .unwrap_or_else(HashMap::new);

    let seconds = match params.get("seconds") {
        Some(value) => value.parse().unwrap_or(30u64),
        _ => 30,
    };

    let guard = pprof::ProfilerGuard::new(100).unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(seconds)).await;

    match guard.report().build() {
        Ok(report) => {
            let file = std::fs::File::create("flamegraph.svg").unwrap();
            report.flamegraph(file).unwrap();

            Ok(Response::new(Body::empty()))
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

impl GenerateConfig for PProfConfig {
    fn generate_config() -> String {
        r#"
# Which address the pprof server will listen
listen: 0.0.0.0:10910
"#
        .into()
    }
}

inventory::submit! {
    ExtensionDescription::new::<PProfConfig>("pprof")
}
