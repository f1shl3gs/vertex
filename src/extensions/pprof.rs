use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;

use futures::FutureExt;
use serde::{Deserialize, Serialize};
use hyper::{
    Body, Request, Response, Server,
    service::{make_service_fn, service_fn},
};

use crate::config::{ExtensionConfig, ExtensionContext, ExtensionDescription, GenerateConfig};
use crate::extensions::Extension;
use crate::shutdown::ShutdownSignal;


#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PProfConfig {
    pub listen: SocketAddr,
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
    let service = make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(handle))
    });

    let server = Server::bind(&addr)
        .serve(service)
        .with_graceful_shutdown(shutdown.map(|_| ()));
    if let Err(e) = server.await {
        error!("pprof serve failed: {}", e)
    }

    Ok(())
}

async fn handle(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let params: HashMap<String, String> = _req
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
        _ => 30
    };

    let guard = pprof::ProfilerGuard::new(100).unwrap();

    tokio::time::sleep(std::time::Duration::from_secs(seconds)).await;

    match guard.report().build() {
        Ok(report) => {
            let file = std::fs::File::create("flamegraph.svg").unwrap();
            report.flamegraph(file).unwrap();
        }

        Err(_) => {}
    }

    Ok(Response::new(Body::from(vec![])))
}

inventory::submit! {
    ExtensionDescription::new::<PProfConfig>("pprof")
}

impl GenerateConfig for PProfConfig {
    fn generate_config() -> String {
        r##"
listen: 0.0.0.0:10910
        "##.to_string()
    }
}
