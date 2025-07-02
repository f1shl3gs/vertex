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

use super::READINESS;

enum HttpAuth {
    Basic(HeaderValue),

    Bearer(HeaderValue),

    None,
}

impl HttpAuth {
    pub fn handle(&self, headers: &HeaderMap) -> bool {
        match self {
            HttpAuth::None => true,
            HttpAuth::Basic(expect) | HttpAuth::Bearer(expect) => {
                if let Some(got) = headers.get(AUTHORIZATION) {
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
) -> Result<(), crate::Error> {
    let listener = MaybeTlsListener::bind(&addr, tls).await?;

    let auth = Arc::new(match auth {
        Some(auth) => match &auth {
            Auth::Basic { user, password } => {
                HttpAuth::Basic(Authorization::basic(user, password).0.encode())
            }
            Auth::Bearer { token } => HttpAuth::Bearer(Authorization::bearer(token)?.0.encode()),
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
                    if READINESS.load(Ordering::Acquire) {
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

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use framework::ShutdownSignal;
    use framework::config::ProxyConfig;
    use framework::http::{Auth, HttpClient};
    use http::{Method, Request, StatusCode};
    use testify::wait::wait_for_tcp;

    use super::*;

    #[tokio::test]
    async fn readiness() {
        let addr = testify::next_addr();

        let (_trigger, shutdown, _) = ShutdownSignal::new_wired();
        tokio::spawn(async move { serve(addr, None, None, shutdown).await.unwrap() });

        wait_for_tcp(addr).await;

        let client = HttpClient::new(None, &ProxyConfig::default()).unwrap();

        let req = Request::builder()
            .method(Method::GET)
            .uri(format!("http://{addr}/ready"))
            .body(Full::<Bytes>::default())
            .expect("request build successful");
        let resp = client.send(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

        READINESS.store(true, Ordering::Release);
        let req = Request::builder()
            .method(Method::GET)
            .uri(format!("http://{addr}/ready"))
            .body(Full::<Bytes>::default())
            .expect("request build successful");
        let resp = client.send(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        READINESS.store(false, Ordering::Release);
        let req = Request::builder()
            .method(Method::GET)
            .uri(format!("http://{addr}/ready"))
            .body(Full::<Bytes>::default())
            .expect("request build successful");
        let resp = client.send(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    async fn run(auth: Option<Auth>) {
        let addr = testify::next_addr();

        let (trigger, shutdown, _) = ShutdownSignal::new_wired();
        let srv_auth = auth.clone();
        tokio::spawn(async move { serve(addr, None, srv_auth, shutdown).await.unwrap() });

        wait_for_tcp(addr).await;

        let client = HttpClient::new(None, &ProxyConfig::default()).unwrap();
        let mut req = Request::builder()
            .method(Method::GET)
            .uri(format!("http://{addr}/healthy"))
            .body(Full::<Bytes>::default())
            .unwrap();
        if let Some(auth) = &auth {
            auth.apply(&mut req);
        }

        let resp = client.send(req).await.unwrap();

        drop(trigger);

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn authorized() {
        run(None).await;
        run(Some(Auth::Basic {
            user: "foo".into(),
            password: "bar".into(),
        }))
        .await;
        run(Some(Auth::Bearer {
            token: "test".into(),
        }))
        .await;
    }

    #[tokio::test]
    async fn unauthorized() {
        let addr = testify::next_addr();
        let (_trigger, shutdown, _) = ShutdownSignal::new_wired();

        tokio::spawn(async move {
            let auth = Auth::Basic {
                user: "foo".into(),
                password: "bar".into(),
            };
            serve(addr, None, Some(auth), shutdown).await.unwrap()
        });

        wait_for_tcp(addr).await;

        let client = HttpClient::new(None, &ProxyConfig::default()).unwrap();

        for auth in [
            None,
            Some(Auth::Basic {
                user: "foo".into(),
                password: "foo".into(), // should be `bar`
            }),
            Some(Auth::Bearer {
                token: "test".into(),
            }),
        ] {
            let mut req = Request::builder()
                .method(Method::GET)
                .uri(format!("http://{addr}/healthy"))
                .body(Full::<Bytes>::default())
                .unwrap();

            if let Some(auth) = &auth {
                auth.apply(&mut req);
            }

            let resp = client.send(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        }
    }
}
