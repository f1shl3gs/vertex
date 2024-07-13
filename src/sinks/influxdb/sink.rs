use std::fmt::Debug;

use async_trait::async_trait;
use event::Events;
use framework::sink::util::builder::SinkBuilderExt;
use framework::sink::util::http::HttpRequest;
use framework::sink::util::Compression;
use framework::stream::{BatcherSettings, DriverResponse};
use framework::template::Template;
use framework::StreamSink;
use futures_util::stream::BoxStream;
use futures_util::StreamExt;
use tower::Service;

use super::request_builder::{InfluxdbRequestBuilder, KeyPartitioner, PartitionKey};

pub struct InfluxdbSink<S> {
    bucket: Template,
    batch: BatcherSettings,
    compression: Compression,
    service: S,
}

impl<S> InfluxdbSink<S>
where
    S: Service<HttpRequest<PartitionKey>> + Send + 'static,
    S::Future: Send + 'static,
    S::Response: DriverResponse + Send + 'static,
    S::Error: Debug + Into<crate::Error> + Send,
{
    pub fn new(
        bucket: Template,
        batch: BatcherSettings,
        compression: Compression,
        service: S,
    ) -> Self {
        Self {
            bucket,
            batch,
            compression,
            service,
        }
    }

    async fn run_inner(self: Box<Self>, input: BoxStream<'_, Events>) -> Result<(), ()> {
        let builder = InfluxdbRequestBuilder::new(self.compression);
        let partitioner = KeyPartitioner::new(self.bucket);

        input
            .flat_map(|events| match events {
                Events::Metrics(metrics) => futures::stream::iter(metrics),
                _ => panic!("unexpect events type"),
            })
            .batched_partitioned(partitioner, self.batch)
            .filter_map(|(key, batch)| async move { key.map(move |k| (k, batch)) })
            .request_builder(None, builder)
            .filter_map(|req| async {
                match req {
                    Err(err) => {
                        error!(message = "build influxdb request failed", %err);
                        None
                    }
                    Ok(req) => Some(req),
                }
            })
            .into_driver(self.service)
            .run()
            .await
    }
}

#[async_trait]
impl<S> StreamSink for InfluxdbSink<S>
where
    S: Service<HttpRequest<PartitionKey>> + Send + 'static,
    S::Future: Send + 'static,
    S::Response: DriverResponse + Send + 'static,
    S::Error: Debug + Into<crate::Error> + Send,
{
    async fn run(self: Box<Self>, input: BoxStream<'_, Events>) -> Result<(), ()> {
        self.run_inner(input).await
    }
}
