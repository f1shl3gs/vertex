use std::net::SocketAddr;

use bytes::Bytes;
use configurable::Configurable;
use event::{AddBatchNotifier, BatchNotifier, BatchStatus, Events};
use framework::tls::MaybeTlsListener;
use framework::tls::TlsConfig;
use framework::{Pipeline, ShutdownSignal};
use http::{Method, Request, Response, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;
use jaeger::agent::deserialize_binary_batch;
use serde::{Deserialize, Serialize};

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
    acknowledgements: bool,
) -> crate::Result<()> {
    let listener = MaybeTlsListener::bind(&config.endpoint, config.tls.as_ref()).await?;

    let service = service_fn(move |req: Request<Incoming>| {
        let output = output.clone();

        async move { handle(req, output, acknowledgements).await }
    });

    framework::http::serve(listener, service)
        .with_graceful_shutdown(shutdown)
        .await
}

async fn handle(
    req: Request<Incoming>,
    mut output: Pipeline,
    acknowledgements: bool,
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
            let mut events: Events = batch.into();

            let (batch, receiver) = BatchNotifier::maybe_new_with_receiver(acknowledgements);
            if let Some(batch) = batch {
                events.add_batch_notifier(batch);
            }

            if let Err(err) = output.send(events).await {
                error!(message = "Error sending batch", ?err);

                let status = if let Some(receiver) = receiver {
                    match receiver.await {
                        BatchStatus::Delivered => StatusCode::OK,
                        BatchStatus::Errored => StatusCode::INTERNAL_SERVER_ERROR,
                        BatchStatus::Failed => StatusCode::SERVICE_UNAVAILABLE,
                    }
                } else {
                    StatusCode::OK
                };

                Ok(Response::builder()
                    .status(status)
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
