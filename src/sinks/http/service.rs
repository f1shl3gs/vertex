use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::Bytes;
use event::{EventFinalizers, EventStatus, Finalizable};
use framework::http::{HttpClient, HttpError};
use framework::sink::util::retries::{RetryAction, RetryLogic};
use framework::stream::DriverResponse;
use futures_util::future::BoxFuture;
use http::header::{CONTENT_ENCODING, CONTENT_TYPE};
use http::{HeaderName, HeaderValue, Method, Request, Response, StatusCode, Uri};
use hyper::{body, Body};
use indexmap::IndexMap;
use tower::Service;

#[derive(Clone)]
pub struct HttpService {
    client: Arc<HttpClient>,
    uri: Uri,
    method: Method,
    headers: IndexMap<HeaderName, HeaderValue>,
    content_type: Option<String>,
    content_encoding: Option<&'static str>,
}

impl HttpService {
    #[inline]
    pub fn new(
        client: HttpClient,
        uri: Uri,
        method: Method,
        headers: IndexMap<HeaderName, HeaderValue>,
        content_type: Option<String>,
        content_encoding: Option<&'static str>,
    ) -> Self {
        Self {
            client: Arc::new(client),
            uri,
            method,
            headers,
            content_type,
            content_encoding,
        }
    }
}

/// Request type for use in the `Service` implementation of HTTP stream sinks
#[derive(Clone)]
pub struct HttpRequest {
    payload: Bytes,
    finalizers: EventFinalizers,

    batch_size: usize,
}

impl Finalizable for HttpRequest {
    #[inline]
    fn take_finalizers(&mut self) -> EventFinalizers {
        self.finalizers.take_finalizers()
    }
}

impl HttpRequest {
    #[inline]
    pub fn new(payload: Bytes, finalizers: EventFinalizers, batch_size: usize) -> Self {
        Self {
            payload,
            finalizers,
            batch_size,
        }
    }
}

/// Response type for use in the `Service` implementation of HTTP stream sinks.
pub struct HttpResponse {
    http_resp: Response<Bytes>,
    batch_size: usize,
    event_size: usize,
}

impl DriverResponse for HttpResponse {
    fn event_status(&self) -> EventStatus {
        let status = self.http_resp.status();

        if status.is_success() {
            EventStatus::Delivered
        } else if status.is_server_error() {
            EventStatus::Rejected
        } else {
            EventStatus::Errored
        }
    }

    fn events_send(&self) -> (usize, usize, Option<&'static str>) {
        (self.batch_size, self.event_size, None)
    }
}

impl Service<HttpRequest> for HttpService {
    type Response = HttpResponse;
    type Error = crate::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: HttpRequest) -> Self::Future {
        let client = Arc::clone(&self.client);
        let method = self.method.clone();
        let uri = self.uri.clone();

        let batch_size = req.batch_size;
        let event_size = req.payload.len();
        let mut builder = Request::builder().method(method).uri(uri);

        if let Some(ct) = &self.content_type {
            builder = builder.header(CONTENT_TYPE, ct);
        }
        if let Some(ce) = self.content_encoding {
            builder = builder.header(CONTENT_ENCODING, ce);
        }
        for (key, value) in self.headers.iter() {
            builder = builder.header(key, value)
        }

        Box::pin(async move {
            let req = builder.body(Body::from(req.payload))?;
            let resp = client.send(req).await?;
            let (parts, body) = resp.into_parts();
            let body = body::to_bytes(body).await?;

            Ok(HttpResponse {
                http_resp: Response::from_parts(parts, body),
                batch_size,
                event_size,
            })
        })
    }
}

#[derive(Clone, Default)]
pub struct HttpRetryLogic;

impl RetryLogic for HttpRetryLogic {
    type Error = HttpError;
    type Response = HttpResponse;

    fn is_retriable_error(&self, _err: &Self::Error) -> bool {
        true
    }

    fn should_retry_resp(&self, resp: &Self::Response) -> RetryAction {
        let status = resp.http_resp.status();

        match status {
            StatusCode::TOO_MANY_REQUESTS => RetryAction::Retry("too many requests".into()),
            StatusCode::NOT_IMPLEMENTED => RetryAction::Retry("endpoint not implemented".into()),
            _ if status.is_server_error() => RetryAction::Retry(
                format!(
                    "{}: {}",
                    status,
                    String::from_utf8_lossy(resp.http_resp.body())
                )
                .into(),
            ),
            _ if status.is_success() => RetryAction::Successful,
            _ => RetryAction::DontRetry(format!("response status: {}", status).into()),
        }
    }
}
