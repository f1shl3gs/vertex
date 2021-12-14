use std::time::Duration;

use buffers::Acker;
use event::encoding::{EncodingConfig, StandardEncodings};
use event::Event;
use futures::{stream::BoxStream, StreamExt};
use log_schema::log_schema;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::error::KafkaError;
use rdkafka::producer::FutureProducer;
use rdkafka::ClientConfig;
use snafu::{ResultExt, Snafu};
use tower::limit::ConcurrencyLimit;

use super::config::{KafkaRole, KafkaSinkConfig, QUEUE_MIN_MESSAGES};
use super::request_builder::KafkaRequestBuilder;
use super::service::KafkaService;
use crate::common::kafka::KafkaStatisticsContext;
use crate::sinks::{util::builder::SinkBuilderExt, StreamSink};
use crate::template::{Template, TemplateParseError};

#[derive(Debug, Snafu)]
pub enum BuildError {
    #[snafu(display("creating kafka producer failed: {}", source))]
    KafkaCreateFailed { source: KafkaError },
    #[snafu(display("invalid topic template: {}", source))]
    TopicTemplate { source: TemplateParseError },
}

pub struct KafkaSink {
    encoding: EncodingConfig<StandardEncodings>,
    acker: Acker,
    service: KafkaService,
    topic: Template,
    key_field: Option<String>,
    headers_field: Option<String>,
}

pub fn create_producer(
    config: ClientConfig,
) -> crate::Result<FutureProducer<KafkaStatisticsContext>> {
    let producer = config
        .create_with_context(KafkaStatisticsContext)
        .context(KafkaCreateFailed)?;
    Ok(producer)
}

impl KafkaSink {
    pub fn new(config: KafkaSinkConfig, acker: Acker) -> crate::Result<Self> {
        let producer = create_producer(config.to_rdkafka(KafkaRole::Producer)?)?;

        Ok(KafkaSink {
            headers_field: config.headers_field,
            encoding: config.encoding,
            acker,
            service: KafkaService::new(producer),
            topic: Template::try_from(config.topic).context(TopicTemplate)?,
            key_field: config.key_field,
        })
    }

    async fn run_inner(self: Box<Self>, input: BoxStream<'_, Event>) -> Result<(), ()> {
        // rdkafka will internally retry forever, so we need some limit to prevent this from
        // overflowing
        let service = ConcurrencyLimit::new(self.service, QUEUE_MIN_MESSAGES as usize);
        let request_builder = KafkaRequestBuilder {
            key_field: self.key_field,
            headers_field: self.headers_field,
            topic_template: self.topic,
            encoder: self.encoding,
            log_schema: log_schema(),
        };

        let sink = input
            .filter_map(|event| futures_util::future::ready(request_builder.build_request(event)))
            .into_driver(service, self.acker);
        sink.run().await
    }
}

pub async fn health_check(config: KafkaSinkConfig) -> crate::Result<()> {
    trace!(message = "Health check started",);

    let client = config.to_rdkafka(KafkaRole::Consumer).unwrap();
    let topic = match Template::try_from(config.topic)
        .context(TopicTemplate)?
        .render_string(&Event::from(""))
    {
        Ok(topic) => Some(topic),
        Err(err) => {
            warn!(
                message = "Could not generate topic for health check",
                %err
            );

            None
        }
    };

    tokio::task::spawn_blocking(move || {
        let consumer: BaseConsumer = client.create().unwrap();
        let topic = topic.as_ref().map(|t| &t[..]);

        consumer
            .fetch_metadata(topic, Duration::from_secs(3))
            .map(|_| ())
    })
    .await??;

    trace!(message = "Health check completed");

    Ok(())
}

#[async_trait::async_trait]
impl StreamSink for KafkaSink {
    async fn run(self: Box<Self>, input: BoxStream<'_, Event>) -> Result<(), ()> {
        self.run_inner(input).await
    }
}
