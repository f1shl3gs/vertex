use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use event::{AddBatchNotifier, BatchNotifier, BatchStatus, Events};
use http::header::{AUTHORIZATION, CONTENT_ENCODING};
use http::{HeaderMap, Method, Request, Response, StatusCode, Uri};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::service::service_fn;

use super::auth::HttpSourceAuth;
use super::error::ErrorMessage;
use super::{decode, HttpSourceAuthConfig};
use crate::config::SourceContext;
use crate::pipeline::Pipeline;
use crate::tls::{MaybeTlsListener, TlsConfig};
use crate::Source;

pub trait HttpSource: Clone + Send + Sync + 'static {
    fn build_events(
        &self,
        uri: &Uri,
        headers: &HeaderMap,
        peer_addr: &SocketAddr,
        body: Bytes,
    ) -> Result<Events, ErrorMessage>;

    fn run(
        self,
        address: SocketAddr,
        method: Method,
        path: &str,
        strict_path: bool,
        tls: Option<&TlsConfig>,
        auth: Option<&HttpSourceAuthConfig>,
        cx: SourceContext,
    ) -> crate::Result<Source> {
        let auth = HttpSourceAuth::try_from(auth)?;
        let acknowledgements = cx.acknowledgements();
        let shutdown = cx.shutdown;
        let output = cx.output;
        let builder = self.clone();
        let tls = tls.cloned();
        let inner = Arc::new(Inner {
            method,
            path: path.to_string(),
            auth,
        });
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

                if strict_path && inner.path != parts.uri.path() {
                    let resp = Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(Full::default())
                        .unwrap();
                    return Ok(resp);
                } else if !parts.uri.path().starts_with(inner.path.as_str()) {
                    let resp = Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(Full::from(format!(
                            "Path must begin with '{}', but got '{}'",
                            inner.path,
                            parts.uri.path()
                        )))
                        .unwrap();
                    return Ok(resp);
                }

                let body = incoming.collect().await?.to_bytes();
                let body = if let Ok(encoding) = parts
                    .headers
                    .get(CONTENT_ENCODING)
                    .map(|v| v.to_str())
                    .transpose()
                {
                    match decode(encoding, body) {
                        Ok(body) => body,
                        Err(err) => {
                            return Ok(err.into());
                        }
                    }
                } else {
                    body
                };

                let peer = parts
                    .extensions
                    .get::<SocketAddr>()
                    .expect("hyper service ConnectInfo is already applied");
                let result = builder.build_events(&parts.uri, &parts.headers, peer, body);
                let resp = handle_request(result, acknowledgements, &mut output).await;

                Ok::<Response<Full<Bytes>>, hyper::Error>(resp)
            }
        });

        Ok(Box::pin(async move {
            let listener = MaybeTlsListener::bind(&address, tls.as_ref())
                .await
                .map_err(|err| {
                    error!(
                        message = "Unable to bind tcp listener",
                        %address,
                        %err,
                    );
                })?;

            crate::http::serve(listener, service)
                .with_graceful_shutdown(shutdown)
                .await
                .map_err(|_err| ())
        }))
    }
}

struct Inner {
    method: Method,
    path: String,
    auth: HttpSourceAuth,
}

async fn handle_request(
    result: Result<Events, ErrorMessage>,
    acknowledgements: bool,
    output: &mut Pipeline,
) -> Response<Full<Bytes>> {
    match result {
        Ok(mut events) => {
            let (batch, receiver) = BatchNotifier::maybe_new_with_receiver(acknowledgements);
            if let Some(batch) = batch {
                events.add_batch_notifier(batch);
            }

            if let Err(err) = output.send(events).await {
                error!(
                    message = "Failed to forward events, downstream is closed",
                    %err
                );

                return ErrorMessage::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to forward events",
                )
                .into();
            }

            if let Some(receiver) = receiver {
                match receiver.await {
                    BatchStatus::Delivered => ok_resp(),
                    BatchStatus::Errored => ErrorMessage::new(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Error delivering contents to sink",
                    )
                    .into(),
                    BatchStatus::Failed => ErrorMessage::new(
                        StatusCode::BAD_REQUEST,
                        "Contents failed to deliver to sink",
                    )
                    .into(),
                }
            } else {
                ok_resp()
            }
        }

        Err(err) => err.into(),
    }
}

fn ok_resp() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .body(Full::default())
        .unwrap()
}
