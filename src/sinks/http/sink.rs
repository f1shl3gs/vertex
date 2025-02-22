use std::fmt::Debug;

use async_trait::async_trait;
use event::{EventContainer, Events};
use framework::StreamSink;
use framework::sink::util::builder::SinkBuilderExt;
use framework::sink::util::http::HttpRequest;
use framework::stream::{BatcherSettings, DriverResponse};
use futures::StreamExt;
use futures_util::stream::BoxStream;
use tower::Service;

use super::request_builder::HttpRequestBuilder;

pub struct HttpSink<S> {
    service: S,
    batch_settings: BatcherSettings,
    request_builder: HttpRequestBuilder,
}

impl<S> HttpSink<S>
where
    S: Service<HttpRequest> + Send + 'static,
    S::Future: Send + 'static,
    S::Response: DriverResponse + Send + 'static,
    S::Error: Debug + Into<crate::Error> + Send,
{
    pub fn new(
        service: S,
        batch_settings: BatcherSettings,
        request_builder: HttpRequestBuilder,
    ) -> Self {
        Self {
            service,
            batch_settings,
            request_builder,
        }
    }

    async fn run_inner(self: Box<Self>, input: BoxStream<'_, Events>) -> Result<(), ()> {
        input
            .flat_map(|events| futures::stream::iter(events.into_events()))
            // Batch the input stream with size calculation based on the configured codec
            .batched(self.batch_settings.into_byte_size_config())
            // Build requests with default concurrency limit.
            .request_builder(None, self.request_builder)
            .filter_map(|res| async move {
                match res {
                    Ok(req) => Some(req),
                    Err(err) => {
                        warn!(message = "build http request failed", %err);

                        None
                    }
                }
            })
            // Generate the driver that will send requests and handle retries,
            // event finalization, and logging/internal metric reporting.
            .into_driver(self.service)
            .run()
            .await
    }
}

#[async_trait]
impl<S> StreamSink for HttpSink<S>
where
    S: Service<HttpRequest> + Send + 'static,
    S::Future: Send + 'static,
    S::Response: DriverResponse + Send + 'static,
    S::Error: Debug + Into<crate::Error> + Send,
{
    async fn run(self: Box<Self>, input: BoxStream<'_, Events>) -> Result<(), ()> {
        self.run_inner(input).await
    }
}
