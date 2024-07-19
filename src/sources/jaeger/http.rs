use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use configurable::Configurable;
use framework::tls::MaybeTlsListener;
use framework::tls::TlsConfig;
use framework::{Pipeline, ShutdownSignal};
use http::{Method, Request, Response, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
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

// https://www.jaegertracing.io/docs/1.31/apis/#thrift-over-http-stable
pub async fn serve(
    config: ThriftHttpConfig,
    shutdown: ShutdownSignal,
    output: Pipeline,
) -> crate::Result<()> {
    let output = Arc::new(Mutex::new(output));
    let listener = MaybeTlsListener::bind(&config.endpoint, &config.tls).await?;
    let service = service_fn(move |req: Request<Incoming>| {
        let output = Arc::clone(&output);
        async move { handle(output, req).await }
    });

    framework::http::serve(listener, service)
        .with_graceful_shutdown(shutdown)
        .await
}

async fn handle(
    output: Arc<Mutex<Pipeline>>,
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    if req.method() != Method::POST {
        return Ok::<_, hyper::Error>(
            Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .body(Full::default())
                .expect("build METHOD_NOT_ALLOWED should always success"),
        );
    }

    if req.uri().path() != "/api/traces" {
        return Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::default())
            .expect("build NOT_FOUND should always success"));
    }

    let data = req.into_body().collect().await?.to_bytes();
    match deserialize_binary_batch(data.to_vec()) {
        Ok(batch) => {
            if let Err(err) = output.lock().await.send(batch).await {
                error!(message = "Error sending batch", ?err);
                Ok(Response::builder()
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .body(Full::default())
                    .expect("build SERVER_UNAVAILABLE should always success"))
            } else {
                Ok(Response::new(Full::default()))
            }
        }
        Err(err) => Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Full::new(err.to_string().into()))
            .expect("build BAD_REQUEST should always success")),
    }
}
