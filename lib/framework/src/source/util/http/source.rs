use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use event::{BatchNotifier, BatchStatus, BatchStatusReceiver, Event};
use futures::TryFutureExt;
use http::header::AUTHORIZATION;
use http::{HeaderMap, Method, Request, Response, StatusCode, Uri};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;

use super::auth::HttpSourceAuth;
use super::error::ErrorMessage;
use super::HttpSourceAuthConfig;
use crate::config::SourceContext;
use crate::pipeline::Pipeline;
use crate::tls::{MaybeTlsListener, TlsConfig};
use crate::Source;

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
        method: Method,
        path: &str,
        tls: &Option<TlsConfig>,
        auth: &Option<HttpSourceAuthConfig>,
        cx: SourceContext,
    ) -> crate::Result<Source> {
        let auth = HttpSourceAuth::try_from(auth.as_ref())?;
        let mut listener = MaybeTlsListener::bind(&address, tls).await?;
        let acknowledgements = cx.acknowledgements();
        let mut shutdown = cx.shutdown;
        let output = cx.output;
        let builder = self.clone();
        let inner = Arc::new(Inner {
            method,
            path: path.to_string(),
            auth,
        });

        Ok(Box::pin(async move {
            loop {
                let conn = tokio::select! {
                    _ = &mut shutdown => break,
                    result = listener.accept() => match result {
                        Ok(conn) => TokioIo::new(conn),
                        Err(err) => {
                            warn!(
                                message = "accept connection failed",
                                %err
                            );

                            continue
                        }
                    }
                };

                let inner = inner.clone();
                let builder = builder.clone();
                let output = output.clone();
                let service = service_fn(move |req: Request<Incoming>| {
                    let (parts, incoming) = req.into_parts();
                    let builder = builder.clone();
                    let inner = inner.clone();
                    let mut output = output.clone();

                    async move {
                        if !inner.auth.validate(parts.headers.get(AUTHORIZATION)) {
                            let resp = Response::builder()
                                .status(StatusCode::UNAUTHORIZED)
                                .body(Full::default())
                                .unwrap();
                            return Ok(resp);
                        }

                        if inner.method != parts.method {
                            let resp = Response::builder()
                                .status(StatusCode::METHOD_NOT_ALLOWED)
                                .body(Full::default())
                                .unwrap();
                            return Ok(resp);
                        }

                        if inner.path != parts.uri.path() {
                            let resp = Response::builder()
                                .status(StatusCode::NOT_FOUND)
                                .body(Full::default())
                                .unwrap();
                            return Ok(resp);
                        }

                        let body = incoming.collect().await?.to_bytes();
                        let data = builder.build_events(&parts.uri, &parts.headers, body);
                        let resp = handle_request(data, acknowledgements, &mut output).await;

                        Ok::<Response<Full<Bytes>>, hyper::Error>(resp)
                    }
                });

                tokio::spawn(async move {
                    if let Err(err) = http1::Builder::new().serve_connection(conn, service).await {
                        warn!(message = "handle http connection failed", %err);
                    }
                });
            }

            Ok(())
        }))
    }
}

struct Inner {
    method: Method,
    path: String,
    auth: HttpSourceAuth,
}

async fn handle_request(
    events: Result<Vec<Event>, ErrorMessage>,
    acknowledgements: bool,
    output: &mut Pipeline,
) -> Response<Full<Bytes>> {
    match events {
        Ok(mut events) => {
            let receiver = BatchNotifier::maybe_apply_to(acknowledgements, &mut events);
            let result = output
                .send_batch(events)
                .map_err(move |err| {
                    // can only fail if receiving end disconnected, so we are
                    // shutting down, probably not gracefully.
                    error!(
                        message = "Failed to forward events, downstream is closed",
                        %err
                    );

                    ErrorMessage::new(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to forward events",
                    )
                })
                .and_then(|_r| handle_batch_status(receiver))
                .await;

            match result {
                Ok(resp) => resp,
                Err(err) => err.into(),
            }
        }

        Err(err) => err.into(),
    }
}

async fn handle_batch_status(
    receiver: Option<BatchStatusReceiver>,
) -> Result<Response<Full<Bytes>>, ErrorMessage> {
    match receiver {
        None => Ok(ok_resp()),
        Some(receiver) => match receiver.await {
            BatchStatus::Delivered => Ok(ok_resp()),
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

fn ok_resp() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .body(Full::default())
        .unwrap()
}
