use async_trait::async_trait;
use event::Events;
use framework::sink::util::builder::SinkBuilderExt;
use framework::sink::util::service::Svc;
use framework::sink::util::Compression;
use framework::stream::BatcherSettings;
use framework::template::Template;
use framework::StreamSink;
use futures_util::stream::BoxStream;
use futures_util::StreamExt;

use super::request_builder::{InfluxdbRequestBuilder, KeyPartitioner};
use super::service::{InfluxdbRetryLogic, InfluxdbService};

pub struct InfluxdbSink {
    bucket: Template,
    batch: BatcherSettings,
    compression: Compression,
    service: Svc<InfluxdbService, InfluxdbRetryLogic>,
}

impl InfluxdbSink {
    pub fn new(
        bucket: Template,
        batch: BatcherSettings,
        compression: Compression,
        service: Svc<InfluxdbService, InfluxdbRetryLogic>,
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
                        error!(message = "build influxdb request failed", ?err);
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
impl StreamSink for InfluxdbSink {
    async fn run(self: Box<Self>, input: BoxStream<'_, Events>) -> Result<(), ()> {
        self.run_inner(input).await
    }
}
