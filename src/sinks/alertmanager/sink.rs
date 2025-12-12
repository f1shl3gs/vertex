use std::fmt::Debug;

use event::Events;
use framework::StreamSink;
use framework::sink::builder::SinkBuilderExt;
use framework::stream::{BatcherSettings, DriverResponse};
use futures::StreamExt;
use futures::stream::BoxStream;
use tower::Service;

use super::request_builder::{AlertmanagerRequestBuilder, AlertsRequest};

pub struct AlertmanagerSink<S> {
    batch: BatcherSettings,
    builder: AlertmanagerRequestBuilder,
    service: S,
}

impl<S> AlertmanagerSink<S>
where
    S: Service<AlertsRequest> + Send + 'static,
    S::Future: Send + 'static,
    S::Response: DriverResponse + Send + 'static,
    S::Error: Debug + Send,
{
    pub fn new(batch: BatcherSettings, builder: AlertmanagerRequestBuilder, service: S) -> Self {
        Self {
            batch,
            builder,
            service,
        }
    }

    async fn run_inner(self: Box<Self>, input: BoxStream<'_, Events>) -> Result<(), ()> {
        use batch::BatchedStream;

        input
            .filter_map(|events| async { events.into_logs() })
            .batched_vector(self.batch)
            .request_builder(None, self.builder)
            .filter_map(|result| async {
                match result {
                    Ok(req) => Some(req),
                    Err(err) => {
                        warn!(message = "build alerts request failed", %err);
                        None
                    }
                }
            })
            .into_driver(self.service)
            .run()
            .await
    }
}

#[async_trait::async_trait]
impl<S> StreamSink for AlertmanagerSink<S>
where
    S: Service<AlertsRequest> + Send + 'static,
    S::Future: Send + 'static,
    S::Response: DriverResponse + Send + 'static,
    S::Error: Debug + Send,
{
    async fn run(self: Box<Self>, input: BoxStream<'_, Events>) -> Result<(), ()> {
        self.run_inner(input).await
    }
}

mod batch {
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use framework::stream::BatcherSettings;
    use futures::Stream;

    pub trait BatchedStream<T>: Sized + Stream<Item = Vec<T>> {
        fn batched_vector(self, batch: BatcherSettings) -> Batched<Self, T> {
            Batched {
                inner: self,
                ticker: tokio::time::interval(batch.timeout),
                pending: Vec::new(),
                config: batch,
            }
        }
    }

    impl<S, T> BatchedStream<T> for S where S: Stream<Item = Vec<T>> {}

    pin_project_lite::pin_project! {
        pub struct Batched<S, T> {
            #[pin]
            inner: S,
            #[pin]
            ticker: tokio::time::Interval,

            pending: Vec<T>,
            config: BatcherSettings,
        }
    }

    impl<S, T> Stream for Batched<S, T>
    where
        S: Stream<Item = Vec<T>>,
    {
        type Item = Vec<T>;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let mut this = self.project();

            if this.pending.len() > this.config.item_limit {
                let remaining = this.pending.split_off(this.config.item_limit);
                return Poll::Ready(Some(std::mem::replace(this.pending, remaining)));
            }

            match this.inner.poll_next(cx) {
                Poll::Ready(Some(items)) => {
                    if this.pending.len() + items.len() > this.config.item_limit {
                        // over limit, just flush current pending
                        let items = std::mem::replace(this.pending, items);
                        return Poll::Ready(Some(items));
                    }

                    this.pending.extend(items);
                }
                Poll::Pending => {
                    if this.pending.is_empty() {
                        return Poll::Pending;
                    }
                }
                Poll::Ready(None) => {
                    if this.pending.is_empty() {
                        return Poll::Ready(None);
                    }

                    return Poll::Ready(Some(std::mem::take(this.pending)));
                }
            }

            match this.ticker.poll_tick(cx) {
                // ticker will never return `Poll::Ready(None)`
                Poll::Ready(_) => Poll::Ready(Some(std::mem::take(this.pending))),
                Poll::Pending => Poll::Pending,
            }
        }
    }

    #[tokio::test]
    async fn limit() {
        use futures::StreamExt;
        use std::time::Duration;

        let mut stream = futures::stream::iter(vec![vec![1, 1], vec![2, 2, 2, 2]]).batched_vector(
            BatcherSettings {
                timeout: Duration::from_secs(2),
                size_limit: 100,
                item_limit: 2,
            },
        );

        while let Some(nums) = stream.next().await {
            assert!(nums.len() <= 2);
        }
    }
}
