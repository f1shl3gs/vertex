#![allow(clippy::print_stdout)] // tests

use bytes::Bytes;
use http::{Request, Response, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use tokio::net::TcpListener;

async fn dump_request(req: Request<Incoming>) -> hyper::Result<Response<Full<Bytes>>> {
    println!("{:?} {} {}", req.version(), req.method(), req.uri());
    for (k, v) in req.headers() {
        println!("{}: {:?}", k, v);
    }

    println!();
    let data = req.into_body().collect().await.unwrap().to_bytes();

    // First two bytes is the magic header of gzip content
    // http://www33146ue.sakura.ne.jp/staff/iz/formats/gzip.html
    if data[0] == 0x1f && data[1] == 0x8b {
        println!("gzip content received")
    } else {
        let body = String::from_utf8_lossy(&data);
        println!("{}\n", body);
    }

    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Full::default())
        .unwrap())
}

// Note: this is not a test actually, it is used to test some http api
#[tokio::test]
#[ignore = "This is a handy tool for dumping http request"]
async fn start_echo_server() {
    match std::env::var("CI") {
        Ok(val) if val == "true" => return,
        _ => {}
    }

    let addr: SocketAddr = "127.0.0.1:9010".parse().unwrap();
    let service = service_fn(dump_request);

    println!("Listening on http://{}", addr);
    let listener = TcpListener::bind(addr).await.unwrap();

    loop {
        let (conn, _peer) = listener.accept().await.unwrap();

        tokio::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(TokioIo::new(conn), service)
                .await
            {
                panic!("handle http connection failed, {err}")
            }
        });
    }
}
