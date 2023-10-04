use std::task::{Context, Poll};

use bytes::Bytes;
use event::{EventFinalizers, EventStatus, Finalizable};
use framework::http::{HttpClient, HttpError};
use framework::sink::util::http::HttpRetryLogic;
use framework::sink::util::retries::{RetryAction, RetryLogic};
use framework::sink::util::Compression;
use framework::stream::DriverResponse;
use futures_util::future::BoxFuture;
use http::header::{AUTHORIZATION, CONTENT_ENCODING, CONTENT_TYPE};
use http::{Request, Response, StatusCode, Uri};
use hyper::service::Service;
use hyper::{body, Body};
use tracing::Instrument;

#[derive(Clone)]
pub struct InfluxdbRequest {
    pub org: String,
    pub bucket: String,
    pub compression: Compression,
    pub finalizers: EventFinalizers,
    pub data: Bytes,
    pub batch_size: usize,
}

impl Finalizable for InfluxdbRequest {
    fn take_finalizers(&mut self) -> EventFinalizers {
        self.finalizers.take_finalizers()
    }
}

pub struct InfluxdbResponse {
    http_resp: Response<Bytes>,
    batch_size: usize,
    event_size: usize,
}

impl DriverResponse for InfluxdbResponse {
    fn event_status(&self) -> EventStatus {
        let status = self.http_resp.status();

        // See https://docs.influxdata.com/influxdb/v2/api/#operation/PostWrite
        if status.is_success() {
            EventStatus::Delivered
        } else if status.is_server_error() {
            EventStatus::Rejected
        } else {
            let body = self.http_resp.body();
            error!(
                message = "write metrics to influxdb failed",
                ?status,
                body = ?String::from_utf8_lossy(body),
                internal_log_rate_limit = true,
            );

            EventStatus::Errored
        }
    }

    fn events_send(&self) -> (usize, usize, Option<&'static str>) {
        (self.batch_size, self.event_size, None)
    }
}

#[derive(Clone)]
pub struct InfluxdbService {
    client: HttpClient,
    endpoint: Uri,
    token: String,
}

impl Service<InfluxdbRequest> for InfluxdbService {
    type Response = InfluxdbResponse;
    type Error = crate::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: InfluxdbRequest) -> Self::Future {
        let mut client = self.client.clone();
        let uri = format!("{}?org={}&bucket={}", self.endpoint, req.org, req.bucket);
        let token = self.token.clone();

        Box::pin(async move {
            let batch_size = req.batch_size;
            let event_size = req.data.len();
            let mut builder = Request::post(uri)
                .header(CONTENT_TYPE, "text/plain")
                .header(AUTHORIZATION, format!("Token {}", token));
            if let Some(ct) = req.compression.content_encoding() {
                builder = builder.header(CONTENT_ENCODING, ct);
            }

            let req = builder
                .body(Body::from(req.data))
                .expect("building HTTP request failed unexpectedly");

            let resp = client.call(req).in_current_span().await?;
            let (parts, body) = resp.into_parts();
            let body = body::to_bytes(body).await?;

            Ok(InfluxdbResponse {
                http_resp: Response::from_parts(parts, body),
                batch_size,
                event_size,
            })
        })
    }
}

impl InfluxdbService {
    pub fn new(client: HttpClient, endpoint: Uri, token: String) -> Self {
        Self {
            client,
            endpoint,
            token,
        }
    }
}

#[derive(Clone, Default)]
pub struct InfluxdbRetryLogic(HttpRetryLogic);

impl RetryLogic for InfluxdbRetryLogic {
    type Error = HttpError;
    type Response = InfluxdbResponse;

    fn is_retriable_error(&self, err: &Self::Error) -> bool {
        self.0.is_retriable_error(err)
    }

    fn should_retry_resp(&self, resp: &Self::Response) -> RetryAction {
        let status = resp.http_resp.status();

        // See https://docs.influxdata.com/influxdb/v2/api/#operation/PostWrite
        match status {
            StatusCode::TOO_MANY_REQUESTS => RetryAction::Retry("too many request".into()),
            StatusCode::PAYLOAD_TOO_LARGE => RetryAction::DontRetry("payload too large".into()),
            _ if status.is_success() => RetryAction::Successful,
            _ if status.is_server_error() => RetryAction::Retry(
                format!(
                    "{}: {}",
                    status,
                    String::from_utf8_lossy(resp.http_resp.body())
                )
                .into(),
            ),
            _ => RetryAction::DontRetry(format!("resp status {status}").into()),
        }
    }
}
