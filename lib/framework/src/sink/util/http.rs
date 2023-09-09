use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use async_trait::async_trait;
use bytes::{Buf, Bytes};
use event::Event;
use futures::ready;
use futures_util::future::BoxFuture;
use http::StatusCode;
use hyper::{body, Body};
use measurable::ByteSizeOf;
use pin_project_lite::pin_project;
use tower::Service;

use crate::batch::{Batch, EncodedEvent};
use crate::http::{HttpClient, HttpError};
use crate::sink::util::retries::{RetryAction, RetryLogic};
use crate::sink::util::service::BatchedSink;
use crate::sink::util::{service::RequestSettings, sink};

#[derive(Clone, Debug, Default)]
pub struct HttpRetryLogic;

impl RetryLogic for HttpRetryLogic {
    type Error = HttpError;
    type Response = hyper::Response<Bytes>;

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

impl<T: fmt::Debug> sink::Response for http::Response<T> {
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

#[async_trait]
pub trait HttpSink: Send + Sync + 'static {
    type Input;
    type Output;
    type Encoder: HttpEventEncoder<Self::Input>;

    fn build_encoder(&self) -> Self::Encoder;

    async fn build_request(&self, events: Self::Output) -> crate::Result<http::Request<Bytes>>;
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
            HttpBatchService<BoxFuture<'static, crate::Result<hyper::Request<Bytes>>>, B::Output>,
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
    RL: RetryLogic<Response = http::Response<Bytes>, Error = HttpError> + Send + 'static,
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
        let request_builder = move |b| -> BoxFuture<'static, crate::Result<http::Request<Bytes>>> {
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
    inner: HttpClient<Body>,
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
    F: Future<Output = crate::Result<hyper::Request<Bytes>>> + Send + 'static,
    B: ByteSizeOf + Send + 'static,
{
    type Response = http::Response<Bytes>;
    type Error = crate::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, body: B) -> Self::Future {
        let builder = Arc::clone(&self.request_builder);
        let mut http_client = self.inner.clone();

        Box::pin(async move {
            let request = builder(body).await?;
            // let byte_size = request.body().len();
            let request = request.map(Body::from);
            // let (protocol, endpoint) = protocol_endpoint(request.uri().clone());
            let response = http_client.call(request).await?;

            if response.status().is_success() {
                // TODO: metric
            }

            let (parts, body) = response.into_parts();
            let mut body = body::aggregate(body).await?;
            Ok(hyper::Response::from_parts(
                parts,
                body.copy_to_bytes(body.remaining()),
            ))
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
