use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use futures::FutureExt;
use hyper::{
    Body, Request, Response, Server,
    service::{make_service_fn, service_fn},
};
use serde::{Deserialize, Serialize};
use stream_cancel::Tripwire;
use crate::config::{ExtensionConfig, ExtensionContext};
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
        .with_graceful_shutdown(async move {
            shutdown.await;
            ()
        });
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

/// TODO: move this to a common mod
async fn tripwire_handler(closed: bool) {
    futures::future::poll_fn(|_| {
        if closed {
            std::task::Poll::Ready(())
        } else {
            std::task::Poll::Pending
        }
    })
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    async fn hello_handle(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
        Ok(Response::new(Body::from("Hello World")))
    }

    #[tokio::test]
    async fn start_server() {
        let addr = SocketAddr::from_str("127.0.0.1:9000").unwrap();
        let service = make_service_fn(|_conn| async {
            Ok::<_, Infallible>(service_fn(hello_handle))
        });

        // tokio::spawn(async move {
        //     let svr = Server::bind(&addr)
        //         .serve(service);
//
        //     println!("start");
        //     if let Err(e) = svr.await {
        //         eprintln!("server error: {}", e)
        //     }
//
        //     println!("done");
        // });

        let server = Server::bind(&addr)
            .serve(service);

        if let Err(err) = server.await {
            println!("err: {}", err);
        }
    }

    #[tokio::test]
    async fn test_run() {
        let addr = SocketAddr::from_str("127.0.0.1:9000").unwrap();
        let task = Box::pin(run(addr));

        let h = tokio::spawn(task);

        let f = h.await.unwrap();
    }
}