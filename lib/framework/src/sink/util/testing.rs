use std::net::SocketAddr;

use bytes::Bytes;
use futures::channel::mpsc;
use futures_util::{SinkExt, TryFutureExt};
use http::{Request, Response, StatusCode};
use hyper::body::HttpBody;
use hyper::service::make_service_fn;
use hyper::{Body, Server};
use serde::Deserialize;
use tower::service_fn;
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
    build_test_server_generic(addr, || Response::new(Body::empty()))
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
            .body(Body::empty())
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
    B: HttpBody + Send + Sync + 'static,
    <B as HttpBody>::Data: Send + Sync,
    <B as HttpBody>::Error: std::error::Error + Send + Sync,
{
    let (tx, rx) = mpsc::channel(100);
    let service = make_service_fn(move |_| {
        let responder = responder.clone();
        let tx = tx.clone();

        async move {
            let responder = responder.clone();
            Ok::<_, crate::Error>(service_fn(move |req: Request<Body>| {
                let responder = responder.clone();
                let mut tx = tx.clone();

                async move {
                    let (parts, body) = req.into_parts();
                    let resp = responder();
                    if resp.status().is_success() {
                        tokio::spawn(async move {
                            let bytes = hyper::body::to_bytes(body).await.unwrap();
                            tx.send((parts, bytes)).await.unwrap();
                        });
                    }

                    Ok::<_, crate::Error>(resp)
                }
            }))
        }
    });

    let (trigger, tripwire) = Tripwire::new();
    let server = Server::bind(&addr)
        .serve(service)
        .with_graceful_shutdown(tripwire)
        .map_err(|err| panic!("Server error: {}", err));

    (rx, trigger, server)
}
