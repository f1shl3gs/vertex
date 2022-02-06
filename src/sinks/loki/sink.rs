use std::collections::HashMap;
use std::io::Error;
use std::num::NonZeroUsize;

use buffers::Acker;
use event::encoding::{EncodingConfig, EncodingConfiguration};
use event::{Event, EventFinalizers, Finalizable, Value};
use framework::config::SinkContext;
use framework::http::HttpClient;
use framework::partition::Partitioner;
use framework::sink::util::builder::SinkBuilderExt;
use framework::sink::util::{Compression, RequestBuilder};
use framework::stream::BatcherSettings;
use framework::template::Template;
use framework::StreamSink;
use futures_util::stream::BoxStream;
use futures_util::StreamExt;
use shared::ByteSizeOf;
use snafu::Snafu;

use super::config::{Encoding, LokiConfig, OutOfOrderAction};
use super::event::{LokiEventDropped, LokiEventUnlabeled, LokiOutOfOrderEventRewrite};
use super::request_builder::{LokiBatchEncoder, LokiEvent, LokiRecord, PartitionKey};
use super::service::{LokiRequest, LokiService};
use crate::common::events::TemplateRenderingFailed;

#[derive(Clone)]
pub struct KeyPartitioner(Option<Template>);

impl KeyPartitioner {
    pub const fn new(template: Option<Template>) -> Self {
        Self(template)
    }
}

impl Partitioner for KeyPartitioner {
    type Item = Event;
    type Key = Option<String>;

    fn partition(&self, item: &Self::Item) -> Self::Key {
        self.0.as_ref().and_then(|tmpl| {
            tmpl.render_string(item)
                .map_err(|err| {
                    emit!(&TemplateRenderingFailed {
                        err,
                        field: Some("tenant_id"),
                        drop_event: false,
                    })
                })
                .ok()
        })
    }
}

#[derive(Default)]
struct RecordPartitionner;

impl Partitioner for RecordPartitionner {
    type Item = LokiRecord;
    type Key = PartitionKey;

    fn partition(&self, item: &Self::Item) -> Self::Key {
        item.partition.clone()
    }
}

#[derive(Clone)]
pub struct LokiRequestBuilder {
    encoder: LokiBatchEncoder,
}

impl LokiRequestBuilder {
    fn new() -> Self {
        Self {
            encoder: LokiBatchEncoder::default(),
        }
    }
}

#[derive(Debug, Snafu)]
pub enum RequestBuildError {
    #[snafu(display("Encoded payload is greater than the max limit"))]
    PayloadTooBig,
    #[snafu(display("Failed to build payload, err: {}", source))]
    IO { source: std::io::Error },
}

impl From<std::io::Error> for RequestBuildError {
    fn from(err: Error) -> Self {
        RequestBuildError::IO { source: err }
    }
}

impl RequestBuilder<(PartitionKey, Vec<LokiRecord>)> for LokiRequestBuilder {
    type Metadata = (Option<String>, usize, EventFinalizers, usize);
    type Events = Vec<LokiRecord>;
    type Encoder = LokiBatchEncoder;
    type Payload = Vec<u8>;
    type Request = LokiRequest;
    type Error = RequestBuildError;

    fn compression(&self) -> Compression {
        Compression::None
    }

    fn encoder(&self) -> &Self::Encoder {
        &self.encoder
    }

    fn split_input(
        &self,
        input: (PartitionKey, Vec<LokiRecord>),
    ) -> (Self::Metadata, Self::Events) {
        let (key, mut events) = input;
        let batch_size = events.len();
        let events_byte_size = events.size_of();
        let finalizers =
            events
                .iter_mut()
                .fold(EventFinalizers::default(), |mut finalizers, record| {
                    finalizers.merge(record.take_finalizers());
                    finalizers
                });

        (
            (key.tenant, batch_size, finalizers, events_byte_size),
            events,
        )
    }

    fn build_request(&self, metadata: Self::Metadata, payload: Self::Payload) -> Self::Request {
        let (tenant, batch_size, finalizers, events_byte_size) = metadata;
        emit!(&internal::EventProcessed {
            component: "loki",
            byte_size: payload.len()
        });

        LokiRequest {
            batch_size,
            finalizers,
            payload,
            tenant,
            events_byte_size,
        }
    }
}

#[derive(Clone)]
pub(super) struct EventEncoder {
    key_partitioner: KeyPartitioner,
    encoding: EncodingConfig<Encoding>,
    labels: HashMap<Template, Template>,
    remove_label_fields: bool,
    remove_timestamp: bool,
}

impl EventEncoder {
    fn build_labels(&self, event: &Event) -> Vec<(String, String)> {
        let log = event.as_log();
        log.tags
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .chain(self.labels.iter().filter_map(|(key_tmpl, value_tmpl)| {
                if let (Ok(key), Ok(value)) = (
                    key_tmpl.render_string(event),
                    value_tmpl.render_string(event),
                ) {
                    Some((key, value))
                } else {
                    None
                }
            }))
            .collect()
    }

    fn remove_label_fields(&self, event: &mut Event) {
        if !self.remove_label_fields {
            return;
        }

        for tmpl in self.labels.values() {
            if let Some(fields) = tmpl.get_fields() {
                for field in fields {
                    event.as_mut_log().remove_field(&field);
                }
            }
        }
    }

    pub fn encode_event(&self, mut event: Event) -> LokiRecord {
        let tenant = self.key_partitioner.partition(&event);
        let finalizers = event.take_finalizers();
        let mut labels = self.build_labels(&event);
        self.remove_label_fields(&mut event);

        let schema = log_schema::log_schema();
        let timestamp_key = schema.timestamp_key();
        let timestamp = match event.as_log().get_field(timestamp_key) {
            Some(event::Value::Timestamp(ts)) => ts.timestamp_nanos(),
            _ => chrono::Utc::now().timestamp_nanos(),
        };

        if self.remove_timestamp {
            event.as_mut_log().remove_field(timestamp_key);
        }

        self.encoding.apply_rules(&mut event);
        let log = event.into_log();
        let event = match &self.encoding.codec() {
            Encoding::Json => {
                serde_json::to_string(&log.fields).expect("json encoding should never fail")
            }

            Encoding::Text => log
                .get_field(schema.message_key())
                .map(Value::to_string_lossy)
                .unwrap_or_default(),

            Encoding::Logfmt => {
                todo!()
            }
        };

        // If no labels are provided we set our own default `{agent="vertex"}` label. This can
        // happen if the only label is a templatable one but the vent doesn't match.
        if labels.is_empty() {
            emit!(&LokiEventUnlabeled);
            labels = vec![("agent".to_string(), "vertex".to_string())]
        }

        let partition = PartitionKey::new(tenant, &mut labels);

        LokiRecord {
            partition,
            labels,
            event: LokiEvent { event, timestamp },
            finalizers,
        }
    }
}

struct RecordFilter {
    timestamps: HashMap<PartitionKey, i64>,
    out_of_order_action: OutOfOrderAction,
}

impl RecordFilter {
    fn new(out_of_order_action: OutOfOrderAction) -> Self {
        Self {
            timestamps: HashMap::new(),
            out_of_order_action,
        }
    }

    pub fn filter_record(&mut self, mut record: LokiRecord) -> Option<LokiRecord> {
        if let Some(latest) = self.timestamps.get_mut(&record.partition) {
            if record.event.timestamp < *latest {
                match self.out_of_order_action {
                    OutOfOrderAction::Drop => {
                        emit!(&LokiEventDropped);
                        None
                    }
                    OutOfOrderAction::RewriteTimestamp => {
                        emit!(&LokiOutOfOrderEventRewrite);
                        record.event.timestamp = *latest;
                        Some(record)
                    }
                }
            } else {
                *latest = record.event.timestamp;
                Some(record)
            }
        } else {
            self.timestamps
                .insert(record.partition.clone(), record.event.timestamp);
            Some(record)
        }
    }
}

#[derive(Clone)]
pub struct LokiSink {
    acker: Acker,
    request_builder: LokiRequestBuilder,
    pub(super) encoder: EventEncoder,
    batch_settings: BatcherSettings,
    out_of_order_action: OutOfOrderAction,
    service: LokiService,
}

impl LokiSink {
    pub fn new(config: LokiConfig, client: HttpClient, cx: SinkContext) -> crate::Result<Self> {
        Ok(Self {
            acker: cx.acker,
            request_builder: LokiRequestBuilder::new(),
            encoder: EventEncoder {
                key_partitioner: KeyPartitioner::new(config.tenant),
                encoding: config.encoding,
                labels: config.labels,
                remove_label_fields: config.remove_label_fields,
                remove_timestamp: config.remove_timestamp,
            },
            batch_settings: config.batch.into_batcher_settings()?,
            out_of_order_action: config.out_of_order_action,
            service: LokiService::new(client, config.endpoint, config.auth)?,
        })
    }

    async fn run_inner(self: Box<Self>, input: BoxStream<'_, Event>) -> Result<(), ()> {
        let service = tower::ServiceBuilder::new()
            .concurrency_limit(1)
            .service(self.service);

        let encoder = self.encoder.clone();
        let mut filter = RecordFilter::new(self.out_of_order_action);

        input
            .map(|event| encoder.encode_event(event))
            .filter_map(|record| {
                let res = filter.filter_record(record);
                async { res }
            })
            .batched_partitioned(RecordPartitionner::default(), self.batch_settings)
            .request_builder(NonZeroUsize::new(1), self.request_builder)
            .filter_map(|req| async move {
                match req {
                    Err(err) => {
                        error!(
                            message = "Failed to build Loki request",
                            %err
                        );

                        None
                    }
                    Ok(req) => Some(req),
                }
            })
            .into_driver(service, self.acker)
            .run()
            .await
    }
}

#[async_trait::async_trait]
impl StreamSink for LokiSink {
    async fn run(mut self: Box<Self>, input: BoxStream<'_, Event>) -> Result<(), ()> {
        self.run_inner(input).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use log_schema::log_schema;
    use testify::random::random_lines;

    #[test]
    fn encoder_no_labels() {
        let encoder = EventEncoder {
            key_partitioner: KeyPartitioner::new(None),
            encoding: EncodingConfig::from(Encoding::Json),
            labels: HashMap::default(),
            remove_label_fields: false,
            remove_timestamp: false,
        };

        let mut event = Event::from("hello");
        let log = event.as_mut_log();
        log.insert_field(log_schema().timestamp_key(), chrono::Utc::now());

        let record = encoder.encode_event(event);
        assert!(record.event.event.contains(log_schema().timestamp_key()));
        assert_eq!(record.labels.len(), 1);
        assert_eq!(
            record.labels[0],
            ("agent".to_string(), "vertex".to_string())
        )
    }

    #[test]
    fn encoder_with_labels() {
        let mut labels = HashMap::default();
        labels.insert(
            Template::try_from("k1").unwrap(),
            Template::try_from("v1").unwrap(),
        );

        labels.insert(
            Template::try_from("{{ name }}").unwrap(),
            Template::try_from("{{ value }}").unwrap(),
        );

        let encoder = EventEncoder {
            key_partitioner: KeyPartitioner::new(None),
            encoding: EncodingConfig::from(Encoding::Json),
            labels,
            remove_label_fields: false,
            remove_timestamp: false,
        };

        let mut event = Event::from("hello");
        let log = event.as_mut_log();
        log.insert_field(log_schema().timestamp_key(), chrono::Utc::now());
        log.insert_field("name", "k2");
        log.insert_field("value", "v2");
        let record = encoder.encode_event(event);
        assert!(record.event.event.contains(log_schema().timestamp_key()));
        assert_eq!(record.labels.len(), 2);
        let labels: HashMap<String, String> = record.labels.into_iter().collect();
        assert_eq!(labels["k1"], "v1".to_string());
        assert_eq!(labels["k2"], "v2".to_string())
    }

    #[test]
    fn encoder_no_ts() {
        let encoder = EventEncoder {
            key_partitioner: KeyPartitioner::new(None),
            encoding: EncodingConfig::from(Encoding::Json),
            labels: HashMap::default(),
            remove_label_fields: false,
            remove_timestamp: true,
        };

        let mut event = Event::from("hello");
        let log = event.as_mut_log();
        log.insert_field(log_schema().timestamp_key(), chrono::Utc::now());
        let record = encoder.encode_event(event);
        assert!(!record.event.event.contains(log_schema().timestamp_key()));
    }

    #[test]
    fn encoder_no_record_labels() {
        let mut labels = HashMap::default();
        labels.insert(
            Template::try_from("k1").unwrap(),
            Template::try_from("v1").unwrap(),
        );
        labels.insert(
            Template::try_from("{{ name }}").unwrap(),
            Template::try_from("{{ value }}").unwrap(),
        );

        let encoder = EventEncoder {
            key_partitioner: KeyPartitioner::new(None),
            encoding: EncodingConfig::from(Encoding::Json),
            labels,
            remove_label_fields: true,
            remove_timestamp: true,
        };

        let mut event = Event::from("hello");
        let log = event.as_mut_log();
        log.insert_field("name", "k2");
        log.insert_field("value", "v2");
        let record = encoder.encode_event(event);
        assert!(!record.event.event.contains("value"));
    }

    #[tokio::test]
    async fn filter_encoder_drop() {
        let encoder = EventEncoder {
            key_partitioner: KeyPartitioner::new(None),
            encoding: EncodingConfig::from(Encoding::Json),
            labels: HashMap::default(),
            remove_label_fields: false,
            remove_timestamp: false,
        };
        let base = chrono::Utc::now();
        let events = random_lines(100)
            .take(20)
            .map(Event::from)
            .enumerate()
            .map(|(i, mut event)| {
                let log = event.as_mut_log();
                let ts = if i % 5 == 1 {
                    base
                } else {
                    base + chrono::Duration::seconds(i as i64)
                };

                log.insert_field(log_schema().timestamp_key(), ts);
                event
            })
            .collect::<Vec<_>>();

        let mut filter = RecordFilter::new(OutOfOrderAction::Drop);
        let stream = futures::stream::iter(events)
            .map(|event| encoder.encode_event(event))
            .filter_map(|event| {
                let res = filter.filter_record(event);
                async { res }
            });

        tokio::pin!(stream);

        let mut result = Vec::new();
        while let Some(item) = stream.next().await {
            result.push(item);
        }

        assert_eq!(result.len(), 17);
    }
}
