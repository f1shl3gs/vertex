use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use event::{BatchNotifier, BatchStatus, BatchStatusReceiver, Event};
use futures::TryFutureExt;
use futures_util::FutureExt;
use http::{HeaderMap, Method, Request, Response, StatusCode, Uri};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Server};

use super::auth::HttpSourceAuth;
use super::error::ErrorMessage;
use super::HttpSourceAuthConfig;
use crate::config::SourceContext;
use crate::pipeline::Pipeline;
use crate::tls::{MaybeTlsSettings, TlsConfig};
use crate::Source;

#[async_trait::async_trait]
pub trait HttpSource: Clone + Send + Sync + 'static {
    fn build_events(
        &self,
        uri: &Uri,
        headers: &HeaderMap,
        body: Bytes,
    ) -> Result<Vec<Event>, ErrorMessage>;

    async fn run(
        self,
        address: SocketAddr,
        method: http::Method,
        path: &str,
        tls: &Option<TlsConfig>,
        auth: &Option<HttpSourceAuthConfig>,
        cx: SourceContext,
        acknowledgements: bool,
    ) -> crate::Result<Source> {
        let tls = MaybeTlsSettings::from_config(tls, true)?;
        let path = path.to_owned();
        let auth = HttpSourceAuth::try_from(auth.as_ref())?;
        let listener = tls.bind(&address).await?;
        let acknowledgements = cx.acknowledgements() || acknowledgements;
        let shutdown = cx.shutdown;
        let output = cx.output;
        let inner = Arc::new(Inner {
            method,
            path: path.to_string(),
            auth,
            output,
        });

        // TODO: metrics

        // TODO: nested closure is pretty tricky, re-work is needed
        let service = make_service_fn(move |_conn| {
            let inner = Arc::clone(&inner);
            let builder = self.clone();

            async move {
                Ok::<_, crate::Error>(service_fn(move |req: Request<Body>| {
                    let inner = Arc::clone(&inner);
                    let mut output = inner.output.clone();
                    let builder = builder.clone();

                    async move {
                        let (parts, body) = req.into_parts();
                        let uri = &parts.uri;
                        let method = parts.method;

                        if method != inner.method {
                            let resp = Response::builder()
                                .status(StatusCode::METHOD_NOT_ALLOWED)
                                .body(Body::empty())
                                .unwrap();
                            return Ok::<_, crate::Error>(resp);
                        }

                        if uri.path() != inner.path {
                            let resp = Response::builder()
                                .status(StatusCode::NOT_FOUND)
                                .body(Body::empty())
                                .unwrap();
                            return Ok::<_, crate::Error>(resp);
                        }

                        // authorization
                        let headers = &parts.headers;
                        if !inner.auth.validate(headers.get("authorization")) {
                            let resp = Response::builder()
                                .status(StatusCode::UNAUTHORIZED)
                                .body(Body::empty())
                                .unwrap();
                            return Ok::<_, crate::Error>(resp);
                        }

                        let body = hyper::body::to_bytes(body).await?;
                        let events = builder.build_events(uri, headers, body);
                        let resp = handle_request(events, acknowledgements, &mut output).await;

                        Ok::<_, crate::Error>(resp)
                    }
                }))
            }
        });

        Ok(Box::pin(async move {
            if let Err(err) =
                Server::builder(hyper::server::accept::from_stream(listener.accept_stream()))
                    .serve(service)
                    .with_graceful_shutdown(shutdown.map(|_| ()))
                    .await
            {
                error!(message = "Http source server start failed", ?err);

                return Err(());
            }

            Ok(())
        }))
    }
}

struct Inner {
    method: Method,
    path: String,
    auth: HttpSourceAuth,
    output: Pipeline,
}

async fn handle_request(
    events: Result<Vec<Event>, ErrorMessage>,
    acknowledgements: bool,
    output: &mut Pipeline,
) -> Response<Body> {
    match events {
        Ok(mut events) => {
            let receiver = BatchNotifier::maybe_apply_to_events(acknowledgements, &mut events);

            match output
                .send_batch(events)
                .map_err(move |err| {
                    // can only fail if receiving end disconnected, so we are
                    // shutting down, probably not gracefully.
                    error!(message = "Failed to forward events, downstream is closed");

                    error!(
                        message = "Tried to send the following event",
                        %err
                    );

                    ErrorMessage::new(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to forward events",
                    )
                })
                .and_then(|_r| handle_batch_status(receiver))
                .await
            {
                Ok(resp) => resp,
                Err(err) => err.into(),
            }
        }

        Err(err) => err.into(),
    }
}

async fn handle_batch_status(
    receiver: Option<BatchStatusReceiver>,
) -> Result<Response<Body>, ErrorMessage> {
    match receiver {
        None => Ok(ok_resp(None)),
        Some(receiver) => match receiver.await {
            BatchStatus::Delivered => Ok(ok_resp(None)),
            BatchStatus::Errored => Err(ErrorMessage {
                code: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Error delivering contents to sink".to_string(),
            }),
            BatchStatus::Failed => Err(ErrorMessage {
                code: StatusCode::BAD_REQUEST,
                message: "Contents failed to deliver to sink".to_string(),
            }),
        },
    }
}

fn ok_resp(body: Option<String>) -> Response<Body> {
    let body = body.map_or(Body::empty(), Body::from);

    Response::builder()
        .status(http::StatusCode::OK)
        .body(body)
        .unwrap()
}
