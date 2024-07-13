use std::net::SocketAddr;

use bytes::Bytes;
use futures::channel::mpsc;
use futures_util::SinkExt;
use http::{Request, Response, StatusCode};
use http_body_util::{BodyExt, Empty};
use hyper::body::{Body, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use serde::Deserialize;
use tokio::net::TcpListener;
use tripwire::{Trigger, Tripwire};

use crate::config::{SinkConfig, SinkContext};

pub fn load_sink<T>(config: &str) -> crate::Result<(T, SinkContext)>
where
    for<'a> T: Deserialize<'a> + SinkConfig,
{
    let config = serde_yaml::from_str(config)?;
    let cx = SinkContext::new_test();

    Ok((config, cx))
}

pub fn build_test_server(
    addr: SocketAddr,
) -> (
    mpsc::Receiver<(http::request::Parts, Bytes)>,
    Trigger,
    impl std::future::Future<Output = Result<(), ()>>,
) {
    build_test_server_generic(addr, || Response::new(Empty::<Bytes>::new()))
}

pub fn build_test_server_status(
    addr: SocketAddr,
    status: StatusCode,
) -> (
    mpsc::Receiver<(http::request::Parts, Bytes)>,
    Trigger,
    impl std::future::Future<Output = Result<(), ()>>,
) {
    build_test_server_generic(addr, move || {
        Response::builder()
            .status(status)
            .body(Empty::<Bytes>::new())
            .unwrap_or_else(|_| unreachable!())
    })
}

pub fn build_test_server_generic<B>(
    addr: SocketAddr,
    responder: impl Fn() -> Response<B> + Clone + Send + Sync + 'static,
) -> (
    mpsc::Receiver<(http::request::Parts, Bytes)>,
    Trigger,
    impl std::future::Future<Output = Result<(), ()>>,
)
where
    B: Body + Send + Sync + 'static,
    <B as Body>::Data: Send + Sync,
    <B as Body>::Error: std::error::Error + Send + Sync,
{
    let (tx, rx) = mpsc::channel(100);
    let (trigger, mut tripwire) = Tripwire::new();

    let server = async move {
        let listener = TcpListener::bind(addr).await.unwrap();

        loop {
            let conn = tokio::select! {
                _ = &mut tripwire => return Ok(()),
                result = listener.accept() => match result {
                    Ok((conn, _addr)) => TokioIo::new(conn),
                    Err(err) => {
                        warn!(
                            message = "accept connection failed",
                            ?err
                        );

                        continue;
                    }
                }
            };

            let responder = responder.clone();
            let tx = tx.clone();
            let service = service_fn(move |req: Request<Incoming>| {
                let (parts, incoming) = req.into_parts();
                let resp = responder();
                let mut tx = tx.clone();
                if resp.status().is_success() {
                    tokio::spawn(async move {
                        let data = incoming.collect().await.unwrap().to_bytes();
                        tx.send((parts, data)).await.unwrap();
                        info!(message = "send response success");
                    });
                }

                async move { Ok::<Response<B>, B::Error>(resp) }
            });

            let mut tripwire = tripwire.clone();
            tokio::spawn(async move {
                let conn = http1::Builder::new().serve_connection(conn, service);
                tokio::pin!(conn);

                loop {
                    tokio::select! {
                        result = conn.as_mut() => {
                            if let Err(err) = result {
                                warn!(message = "failed to serve connection", ?err);
                            }

                            break;
                        },
                        _ = &mut tripwire => {
                            conn.as_mut().graceful_shutdown();
                        }
                    }
                }
            });
        }
    };

    (rx, trigger, server)
}
