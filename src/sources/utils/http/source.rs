use super::error::ErrorMessage;
use crate::config::SourceContext;
use crate::sources::utils::http::HttpSourceAuthConfig;
use crate::sources::Source;
use crate::tls::{MaybeTlsSettings, TlsConfig};
use bytes::Bytes;
use event::Event;
use http::{HeaderMap, Request, Response, StatusCode, Uri};
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use futures_util::TryFutureExt;
use hyper::{Body, Server};
use hyper::service::{make_service_fn, service_fn};

#[async_trait]
pub trait HttpSource: Clone + Send + Sync + 'static {
    fn build_events(
        &self,
        uri: &Uri,
        headers: &HeaderMap,
        body: Bytes,
    ) -> Result<Vec<Event>, ErrorMessage>;

    fn run(
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
        let acknowledgements = ctx.globals.acknowledgements;

        Ok(Box::pin(async move {
            let listener = tls.bind(&address)
                .await?;

            let service = make_service_fn(move |_conn| {
                counter!("http_source_request_total", 1);

                async move {
                    Ok::<_, Infallible>(service_fn(async |req: Request<Body>| {
                        if req.uri().path() != &path {
                            let resp = Response::builder()
                                .status(StatusCode::NOT_FOUND)
                                .body(Body::empty())
                                .unwrap();

                            return futures::future::ok::<_, Infallible>(resp);
                        }

                        let headers = req.headers();

                        if let Some(auth) = auth {
                            match headers.get("authorization") {
                                Some(value) => {
                                    let s = value.to_str().unwrap();
                                    if s != auth.password {

                                    }
                                },
                                None => {
                                    let resp = Response::builder()
                                        .status(StatusCode::UNAUTHORIZED)
                                        .body(Body::empty())
                                        .unwrap();

                                    return Ok(resp);
                                }
                            }
                        }

                        let body = hyper::body::to_bytes(req.body()).await?;
                        let uri = req.uri();

                        let events = self.build_events(body, header)
                            .unwrap();

                        let mut stream = futures::stream(events.iter());

                        match output.send_all(&mut stream).await {
                            Ok(_) => {},
                            Err(err) => {
                                warn!(
                                    message = "Error sending metrics",
                                    ?err
                                );
                            }
                        }

                        let resp = Response::builder()
                            .status(StatusCode::OK)
                            .body(Body::empty())
                            .unwrap();
                        Ok()
                    }))
                }
            });

            if let Err(err) = Server::builder(listener)
                .serve(service)
                .with_graceful_shutdown(shutdown.map_err(|_| ()))
                .await
            {
                error!(
                    message = "Http source server start failed",
                    ?err
                );

                return Err(());
            }

            Ok(())
        }))
    }
}
