use http::{Request, Response, StatusCode};
use hyper::{Body, Server};
use hyper::service::{make_service_fn, service_fn};

async fn dump_request(mut req: Request<Body>) -> hyper::Result<Response<Body>> {
    println!("{:?} {} {}", req.version(), req.method(), req.uri());
    for (k, v) in req.headers() {
        println!("{}: {:?}", k, v);
    }

    println!();
    let body = hyper::body::to_bytes(req.body_mut()).await.unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();
    println!("{}\n", body);

    Ok(Response::builder().status(StatusCode::OK).body(Body::empty()).unwrap())
}

// Note: this is not a test actually, it is used to test some http api
#[tokio::test]
async fn start_echo_server() {
    match std::env::var("CI") {
        Ok(val) if val == "true" => return,
        _ => {}
    }

    let addr = "127.0.0.1:9010".parse().unwrap();
    let service = make_service_fn(|_| async {
        Ok::<_, hyper::Error>(service_fn(dump_request))
    });

    println!("Listening on http://{}", addr);
    Server::bind(&addr)
        .serve(service)
        .await
        .unwrap();
}