use std::task::{Context, Poll};

use finalize::EventStatus;
use framework::http::{Auth, HttpClient, HttpError};
use framework::sink::retries::RetryLogic;
use framework::stream::DriverResponse;
use futures::future::BoxFuture;
use http::header::CONTENT_TYPE;
use http::uri::PathAndQuery;
use http::{HeaderMap, HeaderValue, Request, StatusCode, Uri};
use http_body_util::{BodyExt, Full};
use thiserror::Error;
use tower::Service;

use super::request_builder::AlertsRequest;

#[derive(Clone)]
pub struct AlertmanagerRetryLogic;

impl RetryLogic for AlertmanagerRetryLogic {
    type Error = AlertsError;
    type Response = AlertsResponse;

    fn is_retriable_error(&self, err: &Self::Error) -> bool {
        match err {
            AlertsError::Request(_) | AlertsError::Http(_) => false,
            AlertsError::Api(status, _) => {
                status.is_server_error() || *status == StatusCode::TOO_MANY_REQUESTS
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum AlertsError {
    #[error(transparent)]
    Request(http::Error),

    #[error(transparent)]
    Http(HttpError),

    #[error("unexpected status code {0}, response: {1}")]
    Api(StatusCode, String),
}

pub struct AlertsResponse {
    status: StatusCode,
}

impl DriverResponse for AlertsResponse {
    fn event_status(&self) -> EventStatus {
        if self.status.is_success() {
            EventStatus::Delivered
        } else if self.status.is_server_error() {
            EventStatus::Errored
        } else {
            EventStatus::Rejected
        }
    }

    fn events_send(&self) -> usize {
        100
    }

    fn bytes_sent(&self) -> usize {
        100
    }
}

#[derive(Clone)]
pub struct AlertsService {
    client: HttpClient,
    endpoint: Uri,
    headers: HeaderMap,
}

impl AlertsService {
    pub fn new(client: HttpClient, endpoint: Uri, auth: Option<&Auth>) -> Self {
        let endpoint = {
            let mut parts = endpoint.into_parts();
            parts.path_and_query = Some(PathAndQuery::from_static("/api/v2/alerts"));
            Uri::from_parts(parts).unwrap()
        };

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if let Some(auth) = &auth {
            auth.apply_headers_map(&mut headers);
        }

        Self {
            client,
            endpoint,
            headers,
        }
    }
}

impl Service<AlertsRequest> for AlertsService {
    type Response = AlertsResponse;
    type Error = AlertsError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, AlertsRequest { data, .. }: AlertsRequest) -> Self::Future {
        let client = self.client.clone();
        let endpoint = self.endpoint.clone();

        let mut builder = Request::post(endpoint);
        for (key, value) in &self.headers {
            builder = builder.header(key, value);
        }

        Box::pin(async move {
            let req = builder
                .body(Full::new(data))
                .map_err(AlertsError::Request)?;

            let resp = client.send(req).await.map_err(AlertsError::Http)?;

            let (parts, incoming) = resp.into_parts();
            if !parts.status.is_success() {
                let response = incoming
                    .collect()
                    .await
                    .map_err(|err| AlertsError::Http(err.into()))?
                    .to_bytes();

                return Err(AlertsError::Api(
                    parts.status,
                    String::from_utf8_lossy(&response).to_string(),
                ));
            }

            Ok(AlertsResponse {
                status: parts.status,
            })
        })
    }
}
