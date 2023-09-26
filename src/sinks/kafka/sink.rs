use codecs::encoding::Transformer;
use codecs::Encoder;
use event::{Event, EventContainer, Events};
use framework::sink::util::builder::SinkBuilderExt;
use framework::sink::util::KeyPartitioner;
use framework::stream::BatcherSettings;
use framework::template::{Template, TemplateParseError};
use framework::StreamSink;
use futures::{stream::BoxStream, StreamExt};
use rskafka::client::ClientBuilder;
use thiserror::Error;
use tower::limit::ConcurrencyLimit;

use super::config::Config;
use super::request_builder::KafkaRequestBuilder;
use super::service::KafkaService;

pub const QUEUE_MIN_MESSAGES: usize = 100000;

#[derive(Debug, Error)]
pub enum BuildError {
    #[error("invalid topic template: {0}")]
    TopicTemplate(#[from] TemplateParseError),
}

pub struct KafkaSink {
    transformer: Transformer,
    encoder: Encoder<()>,
    service: KafkaService,
    topic: Template,
    key_field: Option<String>,
    headers_field: Option<String>,
    batch_settings: BatcherSettings,
}

impl KafkaSink {
    pub async fn new(config: Config) -> crate::Result<Self> {
        let transformer = config.encoding.transformer();
        let serializer = config.encoding.build();
        let encoder = Encoder::<()>::new(serializer);
        let batch_settings = config.batch.validate()?.into_batcher_settings()?;
        let client = ClientBuilder::new(config.bootstrap_servers)
            .max_message_size(512 * 1024)
            .client_id("vertex")
            .build()
            .await?;

        Ok(KafkaSink {
            headers_field: config.headers_field,
            transformer,
            encoder,
            service: KafkaService::new(client, config.compression),
            topic: Template::try_from(config.topic)?,
            key_field: config.key_field,
            batch_settings,
        })
    }

    async fn run_inner(self: Box<Self>, input: BoxStream<'_, Events>) -> Result<(), ()> {
        let service = ConcurrencyLimit::new(self.service, QUEUE_MIN_MESSAGES);
        let mut request_builder = KafkaRequestBuilder {
            key_field: self.key_field,
            headers_field: self.headers_field,
            transformer: self.transformer,
            encoder: self.encoder,
        };
        let partitioner = KeyPartitioner::new(Some(self.topic));

        let sink = input
            .flat_map(|events| futures::stream::iter(events.into_events()))
            .batched_partitioned(partitioner, self.batch_settings)
            .filter_map(|(topic, batch)| async { Some((topic?, batch)) })
            .map(|(topic, batch)| request_builder.build(topic, batch))
            .into_driver(service);
        sink.run().await
    }
}

pub async fn health_check(config: Config) -> crate::Result<()> {
    trace!(message = "Health check started",);

    let client = ClientBuilder::new(config.bootstrap_servers)
        .client_id("vertex")
        .build()
        .await?;

    let topic = match Template::try_from(config.topic)?.render_string(&Event::from("")) {
        Ok(topic) => Some(topic),
        Err(err) => {
            warn!(
                message = "Could not generate topic for health check",
                %err
            );

            None
        }
    };

    match topic {
        Some(topic) => {
            client.fetch_metadata(&topic).await?;
        }
        None => {
            client.list_topics().await?;
        }
    }

    trace!(message = "Health check completed");

    Ok(())
}

#[async_trait::async_trait]
impl StreamSink for KafkaSink {
    async fn run(self: Box<Self>, input: BoxStream<'_, Events>) -> Result<(), ()> {
        self.run_inner(input).await
    }
}
