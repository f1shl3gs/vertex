use http::{Request, Response, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Server};

async fn dump_request(mut req: Request<Body>) -> hyper::Result<Response<Body>> {
    println!("{:?} {} {}", req.version(), req.method(), req.uri());
    for (k, v) in req.headers() {
        println!("{}: {:?}", k, v);
    }

    println!();
    let body = hyper::body::to_bytes(req.body_mut()).await.unwrap();
    let data = body.to_vec();

    // First two bytes is the magic header of gzip content
    // http://www33146ue.sakura.ne.jp/staff/iz/formats/gzip.html
    if data[0] == 0x1f && data[1] == 0x8b {
        println!("gzip content received")
    } else {
        let body = String::from_utf8(data).unwrap();
        println!("{}\n", body);
    }

    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::empty())
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

    let addr = "127.0.0.1:9010".parse().unwrap();
    let service = make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(dump_request)) });

    println!("Listening on http://{}", addr);
    Server::bind(&addr).serve(service).await.unwrap();
}
