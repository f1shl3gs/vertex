use std::convert::Infallible;

use framework::tls::MaybeTlsSettings;
use framework::{Pipeline, ShutdownSignal};
use futures_util::future::Shared;
use futures_util::FutureExt;
use http::{Request, Response};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Server};

use super::ThriftHttpConfig;

pub async fn serve(
    config: ThriftHttpConfig,
    shutdown: Shared<ShutdownSignal>,
    _output: Pipeline,
) -> framework::Result<()> {
    let tls = MaybeTlsSettings::from_config(&config.tls, true)?;
    let listener = tls.bind(&config.endpoint).await?;

    let service = make_service_fn(|_conn| async move {
        Ok::<_, Infallible>(service_fn(move |req: Request<Body>| async move {
            let (_header, body) = req.into_parts();
            let body = hyper::body::to_bytes(body).await.unwrap();

            // TODO: consume this field

            Ok::<_, Infallible>(Response::new(Body::empty()))
        }))
    });

    if let Err(err) = Server::builder(hyper::server::accept::from_stream(listener.accept_stream()))
        .serve(service)
        .with_graceful_shutdown(shutdown.map(|_| ()))
        .await
    {
        error!(message = "Jaeger HTTP server exit", ?err);

        Err(err.into())
    } else {
        Ok(())
    }
}
