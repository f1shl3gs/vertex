use std::net::SocketAddr;
use std::sync::Arc;

use configurable::Configurable;
use framework::tls::MaybeTlsListener;
use framework::tls::TlsConfig;
use framework::{Pipeline, ShutdownSignal};
use futures_util::FutureExt;
use http::{Method, Request, Response, StatusCode};
use hyper::body::to_bytes;
use hyper::server::accept::from_stream;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Server};
use jaeger::agent::deserialize_binary_batch;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

fn default_thrift_http_endpoint() -> SocketAddr {
    SocketAddr::new([0, 0, 0, 0].into(), 14268)
}

/// In some cases it is not feasible to deploy Jaeger Agent next to the application,
/// for example, when the application code is running as AWS Lambda function.
/// In these scenarios the Jaeger Clients can be configured to submit spans directly
/// to the Collectors over HTTP/HTTPS.
///
/// See https://www.jaegertracing.io/docs/1.31/apis/#thrift-over-http-stable
#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ThriftHttpConfig {
    #[serde(default = "default_thrift_http_endpoint")]
    #[configurable(required)]
    pub endpoint: SocketAddr,

    #[serde(default)]
    tls: Option<TlsConfig>,
}

pub async fn serve(
    config: ThriftHttpConfig,
    shutdown: ShutdownSignal,
    output: Pipeline,
) -> crate::Result<()> {
    let output = Arc::new(Mutex::new(output));
    let listener = MaybeTlsListener::bind(&config.endpoint, &config.tls).await?;

    // https://www.jaegertracing.io/docs/1.31/apis/#thrift-over-http-stable
    let service = make_service_fn(|_conn| {
        let output = Arc::clone(&output);

        let svc = service_fn(move |req| handle(Arc::clone(&output), req));

        async move { Ok::<_, hyper::Error>(svc) }
    });

    if let Err(err) = Server::builder(from_stream(listener.accept_stream()))
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

async fn handle(
    output: Arc<Mutex<Pipeline>>,
    req: Request<Body>,
) -> Result<Response<Body>, hyper::Error> {
    if req.method() != Method::POST {
        return Ok::<_, hyper::Error>(
            Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .body(Body::empty())
                .expect("build METHOD_NOT_ALLOWED should always success"),
        );
    }

    if req.uri().path() != "/api/traces" {
        return Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .expect("build NOT_FOUND should always success"));
    }

    let buf = to_bytes(req.into_body()).await?;
    match deserialize_binary_batch(buf.to_vec()) {
        Ok(batch) => {
            if let Err(err) = output.lock().await.send(batch).await {
                error!(message = "Error sending batch", ?err);
                Ok(Response::builder()
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .body(Body::empty())
                    .expect("build SERVER_UNAVAILABLE should always success"))
            } else {
                Ok(Response::new(Body::empty()))
            }
        }
        Err(err) => Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(err.to_string()))
            .expect("build BAD_REQUEST should always success")),
    }
}
