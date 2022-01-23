use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use event::{BatchNotifier, BatchStatus, BatchStatusReceiver, Event};
use futures::TryFutureExt;
use futures_util::FutureExt;
use http::{HeaderMap, Request, Response, StatusCode, Uri};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Server};

use super::error::ErrorMessage;
use crate::config::SourceContext;
use crate::pipeline::Pipeline;
use crate::sources::utils::http::auth::HttpSourceAuth;
use crate::sources::utils::http::HttpSourceAuthConfig;
use crate::sources::Source;
use crate::tls::{MaybeTlsSettings, TlsConfig};

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
        strict_path: bool,
        tls: &Option<TlsConfig>,
        auth: &Option<HttpSourceAuthConfig>,
        ctx: SourceContext,
        acknowledgements: bool,
    ) -> crate::Result<Source> {
        let tls = MaybeTlsSettings::from_config(tls, true)?;
        let path = path.to_owned();
        let shutdown = ctx.shutdown;
        let mut output = ctx.output;
        let auth = HttpSourceAuth::try_from(auth.as_ref())?;
        let acknowledgements = ctx.globals.acknowledgements;
        let listener = tls.bind(&address).await?;
        let inner = Arc::new(Inner {
            path: path.to_string(),
            auth,
            output
        });

        let service = make_service_fn(move |_conn| {
            counter!("http_source_connection_total", 1);
            let inner = Arc::clone(&inner);

            async move {
                Ok::<_, crate::Error>(service_fn(move |req: Request<Body>| async move {
                    counter!("http_source_request_total", 1);
                    let inner = Arc::clone(&inner);

                    if req.uri().path() != inner.path {
                        let resp = Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .body(Body::empty())
                            .unwrap();

                        return Ok::<_, crate::Error>(resp);
                    }

                    // authorization
                    let (parts, body) = req.into_parts();
                    let uri = &parts.uri;
                    let headers = &parts.headers;
                    if !inner.auth.validate(&headers.get("authorization")) {
                        let resp = Response::builder()
                            .status(StatusCode::UNAUTHORIZED)
                            .body(Body::empty())
                            .unwrap();
                        return Ok::<_, crate::Error>(resp);
                    }

                    let body = hyper::body::to_bytes(body).await?;
                    let events = self.build_events(uri, headers, body);
                    let ack = acknowledgements;
                    // let resp = handle_request(events, acknowledgements, &mut inner.output).await;
                    let resp = ok_resp(None);

                    return Ok::<_, crate::Error>(resp);
                }))
            }
        });

        Ok(Box::pin(async move {
            let path = path.as_str();

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

    async fn serve(&mut self, req: Request<Body>) -> Result<Response<Body>, crate::Error> {
        todo!()
    }
}

struct Inner {
    path: String,
    auth: HttpSourceAuth,
    output: Pipeline
}

impl Inner {
    async fn serve(&mut self, req: Request<Body>) -> Result<Response<Body>, crate::Error> {
        todo!()
    }
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
                .send_all(&mut futures::stream::iter(events))
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
                .and_then(|r| handle_batch_status(receiver))
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
    let body = body.map_or(Body::empty(), |s| Body::from(s));

    Response::builder()
        .status(http::StatusCode::OK)
        .body(body)
        .unwrap()
}
