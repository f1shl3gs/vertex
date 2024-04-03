use std::task::{Context, Poll};

use bytes::Bytes;
use event::{EventFinalizers, EventStatus, Finalizable};
use framework::http::{Auth, HttpClient, HttpError};
use framework::stream::DriverResponse;
use futures_util::future::BoxFuture;
use http::header::{CONTENT_ENCODING, CONTENT_TYPE};
use http::StatusCode;
use thiserror::Error;
use tower::Service;
use tracing::Instrument;
use url::Url;

#[derive(Debug, Error)]
pub enum LokiError {
    #[error("Server responded with an error: {0}")]
    Server(StatusCode),

    #[error("Failed to make HTTP(S) request: {0}")]
    Http(HttpError),
}

pub struct LokiRequest {
    pub batch_size: usize,
    pub finalizers: EventFinalizers,
    pub payload: Bytes,
    pub tenant: Option<String>,
    pub events_byte_size: usize,
}

impl Finalizable for LokiRequest {
    fn take_finalizers(&mut self) -> EventFinalizers {
        std::mem::take(&mut self.finalizers)
    }
}

#[derive(Debug)]
pub struct LokiResponse {
    batch_size: usize,
    events_byte_size: usize,
}

impl DriverResponse for LokiResponse {
    fn event_status(&self) -> EventStatus {
        EventStatus::Delivered
    }

    fn events_send(&self) -> usize {
        self.batch_size
    }

    fn bytes_sent(&self) -> usize {
        self.events_byte_size
    }
}

#[derive(Debug, Clone)]
pub struct LokiService {
    endpoint: Url,
    auth: Option<Auth>,
    client: HttpClient,
    content_encoding: Option<&'static str>,
}

impl LokiService {
    pub fn new(
        client: HttpClient,
        mut endpoint: Url,
        auth: Option<Auth>,
        content_encoding: Option<&'static str>,
    ) -> crate::Result<Self> {
        endpoint.set_path("loki/api/v1/push");

        Ok(Self {
            client,
            endpoint,
            auth,
            content_encoding,
        })
    }
}

impl Service<LokiRequest> for LokiService {
    type Response = LokiResponse;
    type Error = LokiError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: LokiRequest) -> Self::Future {
        let mut builder = http::Request::post(self.endpoint.as_str())
            .header(CONTENT_TYPE, "application/x-protobuf");

        if let Some(content_encoding) = self.content_encoding {
            builder = builder.header(CONTENT_ENCODING, content_encoding);
        }
        if let Some(tenant) = request.tenant {
            builder = builder.header("X-Scope-OrgID", tenant)
        }

        let body = hyper::Body::from(request.payload);
        let mut req = builder.body(body).unwrap();

        if let Some(auth) = &self.auth {
            auth.apply(&mut req);
        }

        let mut client = self.client.clone();
        let batch_size = request.batch_size;
        let events_byte_size = request.events_byte_size;

        Box::pin(async move {
            match client.call(req).in_current_span().await {
                Ok(resp) => {
                    let status = resp.status();

                    if status.is_success() {
                        Ok(LokiResponse {
                            batch_size,
                            events_byte_size,
                        })
                    } else {
                        Err(LokiError::Server(status))
                    }
                }

                Err(err) => Err(LokiError::Http(err)),
            }
        })
    }
}
