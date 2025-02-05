use std::fmt;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::Bytes;
use bytesize::ByteSizeOf;
use event::{Event, EventFinalizers, EventStatus, Finalizable};
use futures::ready;
use futures_util::future::BoxFuture;
use http::{Request, Response, StatusCode};
use http_body_util::{BodyExt, Full};
use pin_project_lite::pin_project;
use tower::Service;

use crate::batch::{Batch, EncodedEvent};
use crate::http::{HttpClient, HttpError};
use crate::sink::util::retries::{RetryAction, RetryLogic};
use crate::sink::util::service::BatchedSink;
use crate::sink::util::{service::RequestSettings, sink};
use crate::stream::DriverResponse;

#[derive(Clone, Debug, Default)]
pub struct HttpRetryLogic;

impl RetryLogic for HttpRetryLogic {
    type Error = HttpError;
    type Response = Response<Bytes>;

    fn is_retriable_error(&self, _err: &Self::Error) -> bool {
        true
    }

    fn should_retry_resp(&self, resp: &Self::Response) -> RetryAction {
        let status = resp.status();

        match status {
            StatusCode::TOO_MANY_REQUESTS => RetryAction::Retry("too many requests".into()),
            StatusCode::NOT_IMPLEMENTED => RetryAction::Retry("endpoint not implemented".into()),
            _ if status.is_server_error() => RetryAction::Retry(
                format!("{}: {}", status, String::from_utf8_lossy(resp.body())).into(),
            ),
            _ if status.is_success() => RetryAction::Successful,
            _ => RetryAction::DontRetry(format!("response status: {}", status).into()),
        }
    }
}

impl<T: fmt::Debug> sink::Response for Response<T> {
    fn is_successful(&self) -> bool {
        self.status().is_success()
    }

    fn is_transient(&self) -> bool {
        self.status().is_server_error()
    }
}

pub trait HttpEventEncoder<T> {
    fn encode_event(&mut self, event: Event) -> Option<T>;
}

pub trait HttpSink: Send + Sync + 'static {
    type Input;
    type Output;
    type Encoder: HttpEventEncoder<Self::Input>;

    fn build_encoder(&self) -> Self::Encoder;

    fn build_request(
        &self,
        events: Self::Output,
    ) -> impl Future<Output = crate::Result<Request<Bytes>>> + Send;
}

pin_project! {
    /// Provides a simple wrapper around internal tower and batching
    /// sinks for http.
    ///
    /// This type wraps some `HttpSink` and some `Batch` type and will
    /// apply request, batch and tls settings. Internally, it holds an
    /// Arc reference to the `HttpSink`. It then exposes a `Sink`
    /// interface that can be returned from `SinkConfig`.
    ///
    /// Implementation details we require to buffer a single item due
    /// to how `Sink` works. this is because we must "encode" the type
    /// to be able to send it to the inner batch type and sink. Because
    /// of this we must provide a single buffer slot. To ensure the
    /// buffer is fully flushed make sure `poll_flush` returns ready.
    pub struct BatchedHttpSink<T, B, RL = HttpRetryLogic>
    where
        B: Batch,
        B::Output: ByteSizeOf,
        B::Output: Clone,
        B::Output: Send,
        B::Output: 'static,
        T: HttpSink<Input = B::Input, Output = B::Output>,
        RL: RetryLogic<Response = http::Response<Bytes>>,
        RL: Send,
        RL: 'static,
    {
        sink: Arc<T>,
        #[pin]
        inner: BatchedSink<
            HttpBatchService<BoxFuture<'static, crate::Result<Request<Bytes>>>, B::Output>,
            B,
            RL,
        >,

        encoder: T::Encoder,

        // An empty slot is needed to buffer an item where we encoded it but
        // the inner sink is applying back pressure. This trick is used in
        // the `WithFlatMap` sink combinator.
        //
        // See https://docs.rs/futures/0.1.29/src/futures/sink/with_flat_map.rs.html#20
        slot: Option<EncodedEvent<B::Input>>,
    }
}

impl<T, B> BatchedHttpSink<T, B>
where
    B: Batch,
    B::Output: ByteSizeOf + Clone + Send + 'static,
    T: HttpSink<Input = B::Input, Output = B::Output>,
{
    pub fn new(
        sink: T,
        batch: B,
        request_settings: RequestSettings,
        batch_timeout: Duration,
        client: HttpClient,
    ) -> Self {
        Self::with_logic(
            sink,
            batch,
            HttpRetryLogic,
            request_settings,
            batch_timeout,
            client,
        )
    }
}

impl<T, B, RL> BatchedHttpSink<T, B, RL>
where
    B: Batch,
    B::Output: ByteSizeOf + Clone + Send + 'static,
    RL: RetryLogic<Response = Response<Bytes>, Error = HttpError> + Send + 'static,
    T: HttpSink<Input = B::Input, Output = B::Output>,
{
    pub fn with_logic(
        sink: T,
        batch: B,
        retry_logic: RL,
        request_settings: RequestSettings,
        batch_timeout: Duration,
        client: HttpClient,
    ) -> Self {
        let sink = Arc::new(sink);
        let sink1 = Arc::clone(&sink);
        let request_builder = move |b| -> BoxFuture<'static, crate::Result<Request<Bytes>>> {
            let sink = Arc::clone(&sink1);
            Box::pin(async move { sink.build_request(b).await })
        };

        let svc = HttpBatchService::new(client, request_builder);
        let inner = request_settings.batch_sink(retry_logic, svc, batch, batch_timeout);
        let encoder = sink.build_encoder();

        Self {
            sink,
            inner,
            encoder,
            slot: None,
        }
    }
}

impl<T, B, RL> futures_util::Sink<Event> for BatchedHttpSink<T, B, RL>
where
    B: Batch,
    B::Output: ByteSizeOf + Clone + Send + 'static,
    T: HttpSink<Input = B::Input, Output = B::Output>,
    RL: RetryLogic<Response = http::Response<Bytes>> + Send + 'static,
{
    type Error = crate::Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.slot.is_some() {
            match self.as_mut().poll_flush(cx) {
                Poll::Ready(Ok(())) => {}
                Poll::Ready(Err(err)) => return Poll::Ready(Err(err)),
                Poll::Pending => {
                    if self.slot.is_some() {
                        return Poll::Pending;
                    }
                }
            }
        }

        Poll::Ready(Ok(()))
    }

    fn start_send(mut self: Pin<&mut Self>, mut event: Event) -> Result<(), Self::Error> {
        let byte_size = event.size_of();
        let finalizers = event.metadata_mut().take_finalizers();
        if let Some(item) = self.encoder.encode_event(event) {
            *self.project().slot = Some(EncodedEvent {
                item,
                finalizers,
                byte_size,
            });
        }

        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut this = self.project();
        if this.slot.is_some() {
            ready!(this.inner.as_mut().poll_ready(cx))?;
            this.inner.as_mut().start_send(this.slot.take().unwrap())?;
        }

        this.inner.poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().poll_flush(cx))?;
        self.project().inner.poll_close(cx)
    }
}

pub struct HttpBatchService<F, B = Bytes> {
    inner: HttpClient,
    request_builder: Arc<dyn Fn(B) -> F + Send + Sync>,
}

impl<F, B> HttpBatchService<F, B> {
    pub fn new(
        inner: HttpClient,
        request_builder: impl Fn(B) -> F + Send + Sync + 'static,
    ) -> Self {
        HttpBatchService {
            inner,
            request_builder: Arc::new(Box::new(request_builder)),
        }
    }
}

impl<F, B> Service<B> for HttpBatchService<F, B>
where
    F: Future<Output = crate::Result<Request<Bytes>>> + Send + 'static,
    B: ByteSizeOf + Send + 'static,
{
    type Response = Response<Bytes>;
    type Error = crate::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, body: B) -> Self::Future {
        let builder = Arc::clone(&self.request_builder);
        let mut http_client = self.inner.clone();

        Box::pin(async move {
            let req = builder(body).await?;
            let resp = http_client.call(req.map(Full::new)).await?;
            let (parts, incoming) = resp.into_parts();
            let data = incoming.collect().await?.to_bytes();

            Ok(Response::from_parts(parts, data))
        })
    }
}

impl<F, B> Clone for HttpBatchService<F, B> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            request_builder: Arc::clone(&self.request_builder),
        }
    }
}

/// Request type for use in the `Service` implementation of HTTP stream sinks.
#[derive(Clone)]
pub struct HttpRequest<T = ()> {
    payload: Bytes,
    finalizers: EventFinalizers,
    /// Number of events represented by this batch request
    event_count: usize,
    /// Size, in bytes, of the in-memory representation of all events in this batch request.
    events_byte_size: usize,

    metadata: T,
}

impl<T: Send> HttpRequest<T> {
    /// Creates a nwe `HttpRequest`
    #[inline]
    pub fn new(
        payload: Bytes,
        finalizers: EventFinalizers,
        event_count: usize,
        events_byte_size: usize,
        metadata: T,
    ) -> Self {
        Self {
            payload,
            finalizers,
            event_count,
            events_byte_size,
            metadata,
        }
    }

    #[inline]
    pub fn count_and_bytes(&self) -> (usize, usize) {
        (self.event_count, self.events_byte_size)
    }

    #[inline]
    pub const fn metadata(&self) -> &T {
        &self.metadata
    }

    #[inline]
    pub fn take_payload(&mut self) -> Bytes {
        std::mem::take(&mut self.payload)
    }
}

impl<T: Send> Finalizable for HttpRequest<T> {
    fn take_finalizers(&mut self) -> EventFinalizers {
        self.finalizers.take_finalizers()
    }
}

/// Response type for use in the 'Service' implementation of HTTP stream skins
pub struct HttpResponse {
    http_resp: Response<Bytes>,
    /// Number of events represented by this batch request
    event_count: usize,
    /// Size, in bytes, of the in-memory representation of all events in this batch request.
    events_byte_size: usize,
}

impl DriverResponse for HttpResponse {
    fn event_status(&self) -> EventStatus {
        let status = self.http_resp.status();

        if status.is_success() {
            EventStatus::Delivered
        } else if status.is_server_error() {
            EventStatus::Errored
        } else {
            EventStatus::Rejected
        }
    }

    fn events_send(&self) -> usize {
        self.event_count
    }

    fn bytes_sent(&self) -> usize {
        self.events_byte_size
    }
}

/// HTTP request builder for HTTP stream sinks using the generic `HttpService`
pub trait HttpRequestBuilder<T: Send> {
    fn build(&self, req: HttpRequest<T>) -> Result<Request<Bytes>, crate::Error>;
}

/// Generic 'Service' implementation for HTTP stream sink.
#[derive(Clone)]
pub struct HttpService<B> {
    client: HttpClient,
    request_builder: Arc<B>,
}

impl<B> HttpService<B> {
    pub fn new(client: HttpClient, request_builder: B) -> Self {
        let request_builder = Arc::new(request_builder);

        Self {
            client,
            request_builder,
        }
    }
}

impl<B, T> Service<HttpRequest<T>> for HttpService<B>
where
    B: HttpRequestBuilder<T> + Send + Sync + 'static,
    T: Send + 'static,
{
    type Response = HttpResponse;
    type Error = crate::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: HttpRequest<T>) -> Self::Future {
        let builder = Arc::clone(&self.request_builder);
        let client = self.client.clone();

        Box::pin(async move {
            let (event_count, events_byte_size) = req.count_and_bytes();
            let req = builder.build(req)?.map(Full::new);

            let resp = client.send(req).await?;
            let (parts, incoming) = resp.into_parts();
            let data = incoming.collect().await?.to_bytes();

            Ok(HttpResponse {
                http_resp: Response::from_parts(parts, data),
                event_count,
                events_byte_size,
            })
        })
    }
}

/// A more generic version of `HttpRetryLogic` that accepts anything that can
/// be converted to a status code.
pub struct HttpStatusRetryLogic<F, T> {
    f: F,
    _request: PhantomData<T>,
}

impl<F, T> HttpStatusRetryLogic<F, T>
where
    F: Fn(&T) -> StatusCode + Clone + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    pub const fn new(f: F) -> HttpStatusRetryLogic<F, T> {
        Self {
            f,
            _request: PhantomData,
        }
    }
}

impl<F, T> RetryLogic for HttpStatusRetryLogic<F, T>
where
    F: Fn(&T) -> StatusCode + Clone + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    type Error = HttpError;
    type Response = T;

    fn is_retriable_error(&self, _err: &Self::Error) -> bool {
        true
    }

    fn should_retry_resp(&self, resp: &Self::Response) -> RetryAction {
        let status = (self.f)(resp);

        match status {
            StatusCode::TOO_MANY_REQUESTS => RetryAction::Retry("too many requests".into()),
            StatusCode::NOT_IMPLEMENTED => {
                RetryAction::DontRetry("endpoint not implemented".into())
            }
            _ if status.is_server_error() => {
                RetryAction::Retry(format!("Http Status: {}", status).into())
            }
            _ if status.is_success() => RetryAction::Successful,
            _ => RetryAction::DontRetry(format!("Http status: {}", status).into()),
        }
    }
}

impl<F, T> Clone for HttpStatusRetryLogic<F, T>
where
    F: Clone,
{
    fn clone(&self) -> Self {
        Self {
            f: self.f.clone(),
            _request: PhantomData,
        }
    }
}

/// Creates a `RetryLogic` for use with `HttpResponse`
pub fn http_response_retry_logic() -> HttpStatusRetryLogic<
    impl Fn(&HttpResponse) -> StatusCode + Clone + Send + Sync + 'static,
    HttpResponse,
> {
    HttpStatusRetryLogic::new(|resp: &HttpResponse| resp.http_resp.status())
}
