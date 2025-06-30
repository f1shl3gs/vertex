use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use bytes::Bytes;
use framework::ShutdownSignal;
use framework::http::Auth;
use framework::tls::{MaybeTlsListener, TlsConfig};
use headers::Authorization;
use headers::authorization::Credentials;
use http::header::AUTHORIZATION;
use http::{HeaderMap, HeaderValue, Request, Response, StatusCode};
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::service::service_fn;

use crate::Error;

enum HttpAuth {
    Basic(HeaderValue),

    Bearer(HeaderValue),

    None,
}

impl HttpAuth {
    pub fn handle(&self, headers: &HeaderMap) -> bool {
        match self {
            HttpAuth::None => true,
            HttpAuth::Basic(expect) => {
                if let Some(got) = headers.get(AUTHORIZATION) {
                    return got == expect;
                }

                false
            }
            HttpAuth::Bearer(expect) => {
                if let Some(got) = headers.get("bearer") {
                    return got == expect;
                }

                false
            }
        }
    }
}

pub async fn serve(
    addr: SocketAddr,
    tls: Option<&TlsConfig>,
    auth: Option<Auth>,
    shutdown: ShutdownSignal,
) -> Result<(), Error> {
    let listener = MaybeTlsListener::bind(&addr, tls).await?;

    let auth = Arc::new(match auth {
        Some(auth) => match &auth {
            Auth::Basic { user, password } => {
                HttpAuth::Basic(Authorization::basic(user, password).0.encode())
            }
            Auth::Bearer { token } => {
                let value = HeaderValue::from_str(token)?;
                HttpAuth::Bearer(value)
            }
        },
        None => HttpAuth::None,
    });

    let service = service_fn(move |req: Request<Incoming>| {
        let auth = Arc::clone(&auth);

        async move {
            if !auth.handle(req.headers()) {
                return Ok(Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .body(Full::from("Unauthorized"))
                    .expect("build an unauthorized response"));
            }

            if req.method() != http::Method::GET {
                return Ok::<_, hyper::Error>(
                    Response::builder()
                        .status(StatusCode::METHOD_NOT_ALLOWED)
                        .body(Full::default())
                        .expect("should build not allowed response"),
                );
            }

            let (status, body) = match req.uri().path() {
                "/healthy" => (StatusCode::OK, "Vertex is Healthy.\n"),
                "/ready" => {
                    if super::READINESS.load(Ordering::Acquire) {
                        (StatusCode::OK, "Vertex is Ready.\n")
                    } else {
                        (StatusCode::SERVICE_UNAVAILABLE, "Vertex is not ready.\n")
                    }
                }
                _ => (
                    StatusCode::NOT_FOUND,
                    "Only \"/healthy\" and \"/ready\" allowed",
                ),
            };

            Ok(Response::builder()
                .status(status)
                .body(Full::new(Bytes::from_static(body.as_bytes())))
                .expect("should build response"))
        }
    });

    framework::http::serve(listener, service)
        .with_graceful_shutdown(shutdown)
        .await
}
