use std::future::Future;
use std::time::Duration;

use http::{Method, Request, Response, StatusCode};
use hyper::{Body, Server};
use hyper::service::{make_service_fn, service_fn};
use framework::config::ProxyConfig;
use framework::http::HttpClient;
use testify::{pick_unused_local_port};

static NOTFOUND: &[u8] = b"Not Found";

/// HTTP status code 404
fn not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(NOTFOUND.into())
        .unwrap()
}

async fn simple_file_send(filename: &str) -> hyper::Result<Response<Body>> {
    if let Ok(contents) = tokio::fs::read(filename).await {
        let body = contents.into();
        return Ok(Response::new(body));
    }

    Ok(not_found())
}

async fn v3_handle(req: Request<Body>) -> hyper::Result<Response<Body>> {
    info!(
        message = "serve http request",
        path = req.uri().path(),
        handler = "v3"
    );

    if req.method() != Method::GET {
        return Ok(not_found());
    }

    let path = req.uri().path();
    for available in ["/xml/v3/server", "/xml/v3/status", "/xml/v3/tasks", "/xml/v3/zones"] {
        if available == path {
            return simple_file_send(available.replace("/xml", "tests/fixtures/bind").as_str()).await;
        }
    }

    Ok(not_found())
}

async fn v2_handle(req: Request<Body>) -> hyper::Result<Response<Body>> {
    info!(
        message = "serve http request",
        path = req.uri().path(),
        handler = "v2",
    );

    if req.method() != Method::GET {
        return Ok(not_found());
    }

    if req.uri().path() != "/" {
        Ok(not_found())
    } else {
        simple_file_send("tests/fixtures/bind/v2.xml").await
    }
}

async fn start_server<H, S>(handle: H) -> String
where
    H: FnMut(Request<Body>) -> S + Copy + Send + Sync + 'static,
    S: Future<Output = hyper::Result<Response<Body>>> + Send + 'static,
{
    let port = pick_unused_local_port();
    let endpoint = format!("127.0.0.1:{}", port);
    let service = make_service_fn(move |_conn| async move {
        Ok::<_, hyper::Error>(service_fn(handle))
    });
    let addr = endpoint.parse().unwrap();
    let server = Server::bind(&addr).serve(service);

    tokio::spawn(async move {
        if let Err(err) = server.await {
            error!(
                message = "server error",
                ?err
            );
        }
    });

    // sleep 1s to wait for the http server
    tokio::time::sleep(Duration::from_secs(1)).await;

    format!("http://{}", endpoint)
}

#[tokio::test]
async fn v2_client() {
    let endpoint = start_server(v2_handle).await;
    let http_client = HttpClient::new(None, &ProxyConfig::default()).unwrap();
    let client = super::client::Client::new(endpoint, http_client);

    let _statistics = client.stats().await.unwrap();
}

#[tokio::test]
async fn v3_client() {
    let endpoint = start_server(v3_handle).await;
    let http_client = HttpClient::new(None, &ProxyConfig::default()).unwrap();
    let client = super::client::Client::new(endpoint, http_client);

    let _statistics = client.stats().await.unwrap();
}