use std::path::Path;

use hyper::{Body, Response, StatusCode};

static NOTFOUND: &[u8] = b"Not Found";

pub fn unauthorized() -> Response<Body> {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .body("401 Unauthorized\n".into())
        .unwrap()
}

/// HTTP status code 404
pub fn not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(NOTFOUND.into())
        .unwrap()
}

pub async fn file_send(filename: impl AsRef<Path>) -> hyper::Result<Response<Body>> {
    if let Ok(contents) = tokio::fs::read(filename).await {
        let body = contents.into();
        return Ok(Response::new(body));
    }

    Ok(not_found())
}
