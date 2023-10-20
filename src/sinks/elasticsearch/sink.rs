use std::fmt::Debug;
use std::num::NonZeroUsize;

use async_trait::async_trait;
use codecs::encoding::Transformer;
use event::log::path::PathPrefix;
use event::log::{OwnedValuePath, Value};
use event::{Event, EventContainer, Events, LogRecord};
use framework::sink::util::builder::SinkBuilderExt;
use framework::stream::{BatcherSettings, DriverResponse};
use framework::StreamSink;
use futures_util::stream::BoxStream;
use futures_util::StreamExt;
use measurable::ByteSizeOf;
use tower::Service;

use super::encoder::ProcessedEvent;
use super::request_builder::ElasticsearchRequestBuilder;
use super::service::ElasticsearchRequest;
use super::{BulkAction, ElasticsearchCommonMode};

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct PartitionKey {
    pub index: String,
    pub bulk_action: BulkAction,
}

pub struct BatchedEvents {
    pub key: PartitionKey,
    pub events: Vec<ProcessedEvent>,
}

impl ByteSizeOf for BatchedEvents {
    fn allocated_bytes(&self) -> usize {
        self.events.size_of()
    }
}

pub struct ElasticsearchSink<S> {
    pub batch_settings: BatcherSettings,
    pub request_builder: ElasticsearchRequestBuilder,
    pub transformer: Transformer,
    pub service: S,
    pub mode: ElasticsearchCommonMode,
    pub id_key_field: Option<OwnedValuePath>,
}

impl<S> ElasticsearchSink<S>
where
    S: Service<ElasticsearchRequest> + Send + 'static,
    S::Future: Send + 'static,
    S::Response: DriverResponse + Send + 'static,
    S::Error: Debug + Into<crate::Error> + Send,
{
    pub async fn run(self: Box<Self>, input: BoxStream<'_, Events>) -> Result<(), ()> {
        let request_builder_concurrency_limit = NonZeroUsize::new(50);
        let mode = self.mode;
        let id_key_field = self.id_key_field;

        let sink = input
            .flat_map(|events| futures::stream::iter(events.into_events()))
            .map(|mut event| {
                self.transformer.transform(&mut event);
                event
            })
            .map(|event| match event {
                Event::Log(log) => Some(log),
                _ => None,
            })
            .filter_map(|x| async move { x })
            .filter_map(move |log| {
                futures_util::future::ready(process_log(log, &mode, id_key_field.as_ref()))
            })
            .batched(self.batch_settings.into_byte_size_config())
            .request_builder(request_builder_concurrency_limit, self.request_builder)
            .filter_map(|req| async move {
                match req {
                    Ok(req) => Some(req),
                    Err(err) => {
                        error!(
                            message = "Failed to build Elasticsearch request: {:?}",
                            %err
                        );
                        None
                    }
                }
            })
            .into_driver(self.service);

        sink.run().await
    }
}

pub fn process_log(
    mut log: LogRecord,
    mode: &ElasticsearchCommonMode,
    id_key_field: Option<&OwnedValuePath>,
) -> Option<ProcessedEvent> {
    let index = mode.index(&log)?;
    let bulk_action = mode.bulk_action(&log)?;

    if let Some(cfg) = mode.as_data_stream_config() {
        cfg.sync_fields(&mut log);
        cfg.remap_timestamp(&mut log);
    };

    let id = if let Some(Value::Bytes(key)) =
        id_key_field.and_then(|key| log.remove_field((PathPrefix::Event, key)))
    {
        Some(String::from_utf8_lossy(&key).into_owned())
    } else {
        None
    };

    Some(ProcessedEvent {
        index,
        bulk_action,
        log,
        id,
    })
}

#[async_trait]
impl<S> StreamSink for ElasticsearchSink<S>
where
    S: Service<ElasticsearchRequest> + Send + 'static,
    S::Future: Send + 'static,
    S::Response: DriverResponse + Send + 'static,
    S::Error: Debug + Into<crate::Error> + Send,
{
    async fn run(self: Box<Self>, input: BoxStream<'_, Events>) -> Result<(), ()> {
        self.run(input).await
    }
}
