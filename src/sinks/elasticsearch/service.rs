use bytes::Bytes;
use event::{EventFinalizers, EventStatus, Finalizable};
use framework::http::{Auth, HttpClient};
use framework::sink::util::http::HttpBatchService;
use framework::sink::util::service::RequestConfig;
use framework::sink::util::Compression;
use framework::stream::DriverResponse;
use futures_util::future::BoxFuture;
use http::{Request, Response, Uri};
use measurable::ByteSizeOf;
use std::collections::HashMap;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Service, ServiceExt};

#[derive(Clone)]
pub struct ElasticsearchRequest {
    pub payload: Bytes,
    pub finalizers: EventFinalizers,
    pub batch_size: usize,
    pub events_byte_size: usize,
}

impl ByteSizeOf for ElasticsearchRequest {
    fn allocated_bytes(&self) -> usize {
        self.payload.allocated_bytes() + self.finalizers.allocated_bytes()
    }
}

impl Finalizable for ElasticsearchRequest {
    fn take_finalizers(&mut self) -> EventFinalizers {
        std::mem::take(&mut self.finalizers)
    }
}

#[derive(Clone)]
pub struct ElasticsearchService {
    batch_service: HttpBatchService<
        BoxFuture<'static, Result<Request<Bytes>, crate::Error>>,
        ElasticsearchRequest,
    >,
}

impl ElasticsearchService {
    pub fn new(
        client: HttpClient<hyper::Body>,
        request_builder: HttpRequestBuilder,
    ) -> ElasticsearchService {
        let builder = Arc::new(request_builder);
        let batch_service = HttpBatchService::new(client, move |req| {
            let builder = Arc::clone(&builder);
            let fut: BoxFuture<'static, Result<Request<Bytes>, crate::Error>> =
                Box::pin(async move { builder.build_request(req) });

            fut
        });

        ElasticsearchService { batch_service }
    }
}

pub struct HttpRequestBuilder {
    pub bulk_uri: Uri,
    pub query_params: HashMap<String, String>,
    pub compression: Compression,
    pub request_config: RequestConfig,
    pub http_auth: Option<Auth>,
}

impl HttpRequestBuilder {
    pub fn build_request(&self, req: ElasticsearchRequest) -> Result<Request<Bytes>, crate::Error> {
        let mut builder = Request::post(&self.bulk_uri);

        builder = builder.header("Content-Type", "application/x-ndjson");

        if let Some(ce) = self.compression.content_encoding() {
            builder = builder.header("Content-Encoding", ce);
        }

        for (k, v) in &self.request_config.headers {
            builder = builder.header(&k[..], &v[..]);
        }

        if let Some(auth) = &self.http_auth {
            builder = auth.apply_builder(builder);
        }

        let req = builder
            .body(req.payload)
            .expect("Invalid http request value used");

        Ok(req)
    }
}

pub struct ElasticsearchResponse {
    pub http_response: Response<Bytes>,
    pub event_status: EventStatus,
    pub batch_size: usize,
    pub events_byte_size: usize,
}

impl DriverResponse for ElasticsearchResponse {
    fn event_status(&self) -> EventStatus {
        self.event_status
    }

    fn events_send(&self) -> (usize, usize, Option<&'static str>) {
        (self.batch_size, self.events_byte_size, None)
    }
}

impl Service<ElasticsearchRequest> for ElasticsearchService {
    type Response = ElasticsearchResponse;
    type Error = crate::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: ElasticsearchRequest) -> Self::Future {
        let mut http_service = self.batch_service.clone();

        Box::pin(async move {
            http_service.ready().await?;

            let batch_size = req.batch_size;
            let events_byte_size = req.events_byte_size;
            let http_response = http_service.call(req).await?;
            let event_status = get_event_status(&http_response);

            Ok(ElasticsearchResponse {
                event_status,
                http_response,
                batch_size,
                events_byte_size,
            })
        })
    }
}

fn get_event_status(resp: &Response<Bytes>) -> EventStatus {
    let status = resp.status();

    if status.is_success() {
        let body = String::from_utf8_lossy(resp.body());
        if body.contains(r#""errors":true"#) {
            // TODO: metrics
            error!(message = "Response contained errors", ?resp);

            EventStatus::Rejected
        } else {
            EventStatus::Delivered
        }
    } else if status.is_server_error() {
        error!(message = "Response wasn't successful", ?resp);

        EventStatus::Errored
    } else {
        error!(message = "Response failed", ?resp);

        EventStatus::Rejected
    }
}
