use std::path::Path;

use http_body_util::Full;
use hyper::body::Bytes;
use hyper::{Response, StatusCode};

pub fn unauthorized() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .body(Full::new("401 Unauthorized\n".into()))
        .unwrap()
}

/// HTTP status code 404
pub fn not_found() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Full::new("Not Found".into()))
        .unwrap()
}

pub async fn file_send(filename: impl AsRef<Path>) -> hyper::Result<Response<Full<Bytes>>> {
    if let Ok(content) = tokio::fs::read(filename).await {
        return Ok(Response::new(Full::new(content.into())));
    }

    Ok(not_found())
}
