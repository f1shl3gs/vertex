use std::collections::BTreeMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

use event::{EventFinalizers, EventStatus, Finalizable};
use framework::stream::DriverResponse;
use futures_util::future::BoxFuture;
use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;
use rskafka::client::partition::{Compression, PartitionClient, UnknownTopicHandling};
use rskafka::client::producer::Error;
use rskafka::client::Client;
use rskafka::record::Record;
use tokio::sync::Mutex;
use tokio::time::{sleep_until, Duration, Instant, Sleep};
use tower::Service;

/// Producer should update topic metadata every 10m
const REFRESH_METADATA_INTERVAL: Duration = Duration::from_secs(10 * 60);

pub struct KafkaRequest {
    pub topic: String,
    pub finalizers: EventFinalizers,
    pub records: Vec<Record>,
}

pub struct KafkaResponse {
    event_byte_size: usize,
}

impl DriverResponse for KafkaResponse {
    fn event_status(&self) -> EventStatus {
        EventStatus::Delivered
    }

    fn events_send(&self) -> (usize, usize, Option<&'static str>) {
        (1, self.event_byte_size, None)
    }
}

impl Finalizable for KafkaRequest {
    fn take_finalizers(&mut self) -> EventFinalizers {
        std::mem::take(&mut self.finalizers)
    }
}

struct PartitionedProducer {
    name: String,
    compression: Compression,

    next: AtomicUsize,
    client: Arc<Client>,
    producers: Vec<PartitionClient>,
    refresh: Pin<Box<Sleep>>,
}

impl PartitionedProducer {
    async fn update_partitions(&mut self) -> Result<(), Error> {
        self.refresh
            .as_mut()
            .reset(Instant::now() + REFRESH_METADATA_INTERVAL);

        let topic = self
            .client
            .fetch_metadata(&self.name)
            .await
            .map_err(|err| Error::Client(Arc::new(err)))?;

        let mut tasks =
            FuturesUnordered::from_iter(topic.partitions.keys().map(|partition| async {
                let partition_client = self
                    .client
                    .partition_client(&self.name, *partition, UnknownTopicHandling::Error)
                    .await
                    .map_err(|err| Error::Client(Arc::new(err)))?;

                Ok((*partition, partition_client))
            }));

        let mut producers = Vec::new();
        while let Some(result) = tasks.next().await {
            match result {
                Err(err) => return Err(err),
                Ok((_partition, batch_producer)) => {
                    producers.push(batch_producer);
                }
            }
        }

        debug!(
            message = "update partitions success",
            topic = &self.name,
            partitions = producers.len()
        );

        self.producers = producers;

        Ok(())
    }

    async fn send(&mut self, req: KafkaRequest) -> Result<KafkaResponse, Error> {
        match futures::poll!(Pin::new(&mut self.refresh)) {
            Poll::Pending => {
                // metadata is not outdated, so nothing need to be done
            }
            Poll::Ready(()) => {
                self.update_partitions().await?;
            }
        }

        if self.producers.is_empty() {
            self.update_partitions().await?;
        }

        // load balance
        let pick = self.next.fetch_add(1, Ordering::SeqCst) % self.producers.len();
        let producer = self
            .producers
            .get(pick)
            .expect("get producer shall never failed");

        let _offset = producer
            .produce(req.records, self.compression)
            .await
            .map_err(Arc::new)?;

        Ok(KafkaResponse { event_byte_size: 0 })
    }
}

#[derive(Clone)]
pub struct KafkaService {
    client: Arc<Client>,
    compression: Compression,
    producers: Arc<Mutex<BTreeMap<String, PartitionedProducer>>>,
}

impl KafkaService {
    pub fn new(client: Client, compression: Compression) -> Self {
        Self {
            client: Arc::new(client),
            compression,
            producers: Default::default(),
        }
    }

    fn send(&mut self, req: KafkaRequest) -> BoxFuture<'static, Result<KafkaResponse, Error>> {
        let svc = self.clone();

        let fut = async move {
            let topic = &req.topic;

            svc.producers
                .lock()
                .await
                .entry(topic.to_string())
                .or_insert_with(|| PartitionedProducer {
                    name: topic.to_string(),
                    compression: svc.compression,
                    next: Default::default(),
                    client: Arc::clone(&svc.client),
                    producers: Default::default(),
                    // set sleep_until to now, so our first
                    refresh: Box::pin(sleep_until(Instant::now())),
                })
                .send(req)
                .await
        };

        Box::pin(fut)
    }
}

impl Service<KafkaRequest> for KafkaService {
    type Response = KafkaResponse;
    type Error = Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: KafkaRequest) -> Self::Future {
        self.send(req)
    }
}
