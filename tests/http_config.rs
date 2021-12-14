#![allow(clippy::print_stdout)] // tests
#![allow(clippy::print_stderr)] // tests

use http::{Request, Response, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Server};
use tokio_util::codec::{BytesCodec, FramedRead};

// serve a file by asynchronously reading it by chunks using tokio-util crate
async fn send_file(_req: Request<Body>) -> hyper::Result<Response<Body>> {
    if let Ok(file) = tokio::fs::File::open("").await {
        let stream = FramedRead::new(file, BytesCodec::new());
        let body = Body::wrap_stream(stream);
        return Ok(Response::new(body));
    }

    Ok(Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Body::empty())
        .unwrap())
}

#[tokio::test]
async fn http_config() {
    let addr = testify::next_addr();
    let service = make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(send_file)) });

    let server = Server::bind(&addr).serve(service);

    println!("Listening on http://{}", addr);
    tokio::spawn(async move {
        if let Err(err) = server.await {
            eprintln!("server error: {}", err);
        }
    });

    std::thread::sleep(std::time::Duration::from_secs(5));
}
