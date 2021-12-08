use std::task::{Context, Poll};

use futures_util::future::BoxFuture;
use http::{StatusCode, Uri};
use tower::Service;
use tracing::Instrument;
use buffers::Ackable;
use event::{EventFinalizers, EventStatus, Finalizable};
use internal::EventsSent;
use snafu::Snafu;
use crate::config::UriSerde;

use crate::http::{Auth, HttpClient};
use crate::sinks::util::Compression;
use crate::stream::DriverResponse;


#[derive(Debug, Snafu)]
pub enum LokiError {
    #[snafu(display("Server responded with an error: {}", code))]
    ServerError { code: StatusCode },
    #[snafu(display("Failed to make HTTP(S) request: {}", source))]
    HttpError { source: crate::http::HttpError },
}

pub struct LokiRequest {
    pub batch_size: usize,
    pub finalizers: EventFinalizers,
    pub payload: Vec<u8>,
    pub tenant: Option<String>,
    pub events_byte_size: usize,
}

impl Ackable for LokiRequest {
    fn ack_size(&self) -> usize {
        self.batch_size
    }
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

    fn events_send(&self) -> EventsSent {
        EventsSent {
            count: self.batch_size,
            byte_size: self.events_byte_size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LokiService {
    endpoint: UriSerde,
    client: HttpClient,
    compression: Compression,
}

impl LokiService {
    pub fn new(client: HttpClient, endpoint: UriSerde, auth: Option<Auth>) -> crate::Result<Self> {
        let endpoint = endpoint.append_path("loki/api/v1/push")?
            .with_auth(auth);

        Ok(Self { client, endpoint, compression: Compression::gzip_default() })
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
        let mut builder = http::Request::post(&self.endpoint.uri)
            .header("Content-Type", "application/json");

        if let Some(ce) = self.compression.content_encoding() {
            builder = builder.header(http::header::CONTENT_ENCODING, ce);
        }

        if let Some(tenant) = request.tenant {
            builder = builder.header("X-Scope-OrgID", tenant)
        }

        let body = hyper::Body::from(request.payload);
        let mut req = builder.body(body).unwrap();

        if let Some(auth) = &self.endpoint.auth {
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
                        Err(LokiError::ServerError { code: status })
                    }
                }

                Err(err) => Err(LokiError::HttpError { source: err })
            }
        })
    }
}