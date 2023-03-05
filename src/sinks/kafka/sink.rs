use codecs::encoding::Transformer;
use codecs::Encoder;
use event::{Event, EventContainer, Events};
use framework::sink::util::builder::SinkBuilderExt;
use framework::template::{Template, TemplateParseError};
use framework::StreamSink;
use futures::{stream::BoxStream, StreamExt};
use log_schema::log_schema;
use rskafka::client::ClientBuilder;
use thiserror::Error;
use tower::limit::ConcurrencyLimit;

use super::config::{KafkaSinkConfig, QUEUE_MIN_MESSAGES};
use super::request_builder::KafkaRequestBuilder;
use super::service::KafkaService;

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
}

impl KafkaSink {
    pub async fn new(config: KafkaSinkConfig) -> crate::Result<Self> {
        let transformer = config.encoding.transformer();
        let serializer = config.encoding.build();
        let encoder = Encoder::<()>::new(serializer);
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
        })
    }

    async fn run_inner(self: Box<Self>, input: BoxStream<'_, Events>) -> Result<(), ()> {
        let service = ConcurrencyLimit::new(self.service, QUEUE_MIN_MESSAGES as usize);
        let mut request_builder = KafkaRequestBuilder {
            key_field: self.key_field,
            headers_field: self.headers_field,
            topic_template: self.topic,
            transformer: self.transformer.clone(),
            encoder: self.encoder.clone(),
            log_schema: log_schema(),
        };

        let sink = input
            .flat_map(|events| futures::stream::iter(events.into_events()))
            .filter_map(|event| futures_util::future::ready(request_builder.build_request(event)))
            .into_driver(service);
        sink.run().await
    }
}

pub async fn health_check(config: KafkaSinkConfig) -> crate::Result<()> {
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
