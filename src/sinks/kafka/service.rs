use std::task::{Context, Poll};

use bytes::Bytes;
use event::{EventFinalizers, EventStatus, Finalizable};
use framework::stream::DriverResponse;
use futures_util::future::BoxFuture;
use rdkafka::error::KafkaError;
use rdkafka::message::OwnedHeaders;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;
use tower::Service;

use crate::common::kafka::KafkaStatisticsContext;

pub struct KafkaRequestMetadata {
    pub finalizers: EventFinalizers,
    pub key: Option<Bytes>,
    pub timestamp_millis: Option<i64>,
    pub headers: Option<OwnedHeaders>,
    pub topic: String,
}

pub struct KafkaRequest {
    pub body: Bytes,
    pub metadata: KafkaRequestMetadata,
    pub event_byte_size: usize,
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
        std::mem::take(&mut self.metadata.finalizers)
    }
}

pub struct KafkaService {
    producer: FutureProducer<KafkaStatisticsContext>,
}

impl KafkaService {
    pub const fn new(producer: FutureProducer<KafkaStatisticsContext>) -> KafkaService {
        KafkaService { producer }
    }
}

impl Service<KafkaRequest> for KafkaService {
    type Response = KafkaResponse;
    type Error = KafkaError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: KafkaRequest) -> Self::Future {
        let producer = self.producer.clone();

        Box::pin(async move {
            let mut record = FutureRecord::to(&req.metadata.topic).payload(req.body.as_ref());
            if let Some(key) = &req.metadata.key {
                record = record.key(&key[..]);
            }
            if let Some(timestamp) = req.metadata.timestamp_millis {
                record = record.timestamp(timestamp);
            }
            if let Some(headers) = req.metadata.headers {
                record = record.headers(headers);
            }

            // rdkafka will internally retry forever if the queue is full
            match producer.send(record, Timeout::Never).await {
                Ok((_partition, _offset)) => {
                    // TODO: metrics?
                    // emit!(&BytesSent {
                    //     byte_size: req.body.len() + req.metadata.key.map(|x| x.len()).unwrap_or(0),
                    //     protocol: "kafka"
                    // });

                    Ok(KafkaResponse {
                        event_byte_size: req.event_byte_size,
                    })
                }
                Err((err, _original_record)) => Err(err),
            }
        })
    }
}
