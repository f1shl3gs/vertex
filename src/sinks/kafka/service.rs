use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use event::{EventFinalizers, EventStatus, Finalizable};
use framework::stream::DriverResponse;
use futures::future::BoxFuture;
use parking_lot::RwLock;
use rskafka::client::Client;
use rskafka::client::partition::{Compression, PartitionClient, UnknownTopicHandling};
use rskafka::client::producer::Error;
use rskafka::record::Record;
use tokio::select;
use tower::Service;
use tripwire::{Trigger, Tripwire};

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

    fn events_send(&self) -> usize {
        1
    }

    fn bytes_sent(&self) -> usize {
        self.event_byte_size
    }
}

impl Finalizable for KafkaRequest {
    fn take_finalizers(&mut self) -> EventFinalizers {
        std::mem::take(&mut self.finalizers)
    }
}
struct TopicProducer {
    compression: Compression,
    count: AtomicUsize,
    partitions: Vec<PartitionClient>,
}

impl TopicProducer {
    async fn new(
        client: Arc<Client>,
        compression: Compression,
        topic: String,
        partition_num: usize,
    ) -> Result<TopicProducer, Error> {
        let mut partitions = Vec::with_capacity(partition_num);

        for partition in 0..partition_num {
            let pc = client
                .partition_client(&topic, partition as i32, UnknownTopicHandling::Error)
                .await
                .map_err(|err| Error::Client(err.into()))?;

            partitions.push(pc);
        }

        Ok(TopicProducer {
            partitions,
            compression,
            count: AtomicUsize::new(0),
        })
    }

    async fn send(&self, req: KafkaRequest) -> Result<KafkaResponse, Error> {
        let index = self.count.fetch_add(1, Ordering::Relaxed) % self.partitions.len();
        let pc = &self.partitions[index];

        pc.produce(req.records, self.compression)
            .await
            .map_err(|err| Error::Client(err.into()))?;

        Ok(KafkaResponse { event_byte_size: 0 })
    }
}

pub struct KafkaService {
    client: Arc<Client>,
    compression: Compression,
    producers: Arc<RwLock<BTreeMap<String, Arc<TopicProducer>>>>,

    // when this service dropped, the trigger dropped too,
    // so the background task can receive a signal and
    // return
    #[allow(dead_code)]
    trigger: Trigger,
}

impl KafkaService {
    pub fn new(client: Client, compression: Compression) -> Self {
        let producers = Arc::new(Default::default());
        let (trigger, tripwire) = Tripwire::new();
        let client = Arc::new(client);

        tokio::spawn(update_topics(
            Arc::clone(&client),
            tripwire,
            Arc::clone(&producers),
        ));

        KafkaService {
            client,
            compression,
            producers,
            trigger,
        }
    }
}

async fn update_topics(
    client: Arc<Client>,
    mut tripwire: Tripwire,
    producers: Arc<RwLock<BTreeMap<String, Arc<TopicProducer>>>>,
) {
    debug!(message = "start updating topic metadata...");

    let mut ticker = tokio::time::interval(REFRESH_METADATA_INTERVAL);
    loop {
        select! {
            _ = ticker.tick() => {},
            _ = &mut tripwire => {
                debug!(message = "stop updating topic metadata...");
                return
            }
        }

        match client.list_topics().await {
            Ok(topics) => {
                producers.write().retain(|name, producer| {
                    match topics.iter().find(|topic| &topic.name == name) {
                        None => {
                            // producer exists, but it not found in topics we just
                            // received, which means the topic is removed.
                            info!(
                                message = "remove topic producer, cause it removed",
                                topic = %name,
                            );

                            false
                        }
                        Some(topic) => {
                            let keep = topic.partitions.len() == producer.partitions.len();
                            if !keep {
                                info!(
                                    message = "topic partition number changed",
                                    topic = %name,
                                    old = producer.partitions.len(),
                                    new = topic.partitions.len(),
                                );
                            }

                            keep
                        }
                    }
                });
            }
            Err(err) => {
                warn!(message = "Failed to list Kafka topics", %err);
                continue;
            }
        }
    }
}

impl Service<KafkaRequest> for KafkaService {
    type Response = KafkaResponse;
    type Error = crate::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: KafkaRequest) -> Self::Future {
        if let Some(producer) = self.producers.read().get(&req.topic) {
            let producer = Arc::clone(producer);
            return Box::pin(async move { producer.send(req).await.map_err(Into::into) });
        }

        let client = Arc::clone(&self.client);
        let compression = self.compression;
        let producers = Arc::clone(&self.producers);

        Box::pin(async move {
            let topics = client.list_topics().await?;

            let producer = match topics.into_iter().find(|t| t.name == req.topic) {
                Some(topic) => {
                    // create topic partition client
                    let producer =
                        TopicProducer::new(client, compression, topic.name, topic.partitions.len())
                            .await
                            .map(Arc::new)?;

                    producers
                        .write()
                        .insert(req.topic.clone(), Arc::clone(&producer));

                    producer
                }
                None => {
                    return Err(format!("Topic {} not found", req.topic).into());
                }
            };

            producer.send(req).await.map_err(Into::into)
        })
    }
}
