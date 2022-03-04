mod namespace_metadata_annotator;
mod pod_metadata_annotator;
mod provider;

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use bytes::Bytes;
use chrono::Utc;
use event::attributes::Key;
use event::{Event, LogRecord};
use framework::config::{
    deserialize_duration, serialize_duration, ComponentKey, DataType, GenerateConfig,
    GlobalOptions, Output, ProxyConfig, SourceConfig, SourceContext, SourceDescription,
};
use framework::timezone::TimeZone;
use framework::{Pipeline, ShutdownSignal, Source};
use futures_util::{FutureExt, StreamExt};
use k8s_openapi::api::core::v1::Pod;
use log_schema::log_schema;
use serde::{Deserialize, Serialize};
use tail::{Checkpointer, Line};

use crate::kubernetes;
use crate::kubernetes::state::hash_value::HashKey;
use crate::sources::kubernetes_logs::provider::KubernetesPathsProvider;

/// Configuration for the `kubernetes_logs` source.
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    /// Specifies the label selector to filter `Pod`s with, to be used in
    /// addition to the built-in `vertex.io/exclude` filter
    extra_label_selector: String,

    /// The `name` of the Kubernetes `Node` that Vertex runs at.
    /// Required to filter the `Pod`s to only include the ones with the
    /// log files accessible locally.
    self_node_name: String,

    /// Specifies the field selector to filter `Pod`s with, to be used in
    /// addition to the built-in `Node` filter.
    extra_field_selector: String,

    /// Automatically merge partial events.
    auto_partial_merge: bool,

    /// Override global data_dir
    data_dir: Option<PathBuf>,

    /// Specifies the field names for Pod metadata annotation
    annotation_fields: pod_metadata_annotator::FieldsSpec,

    /// Specifies the field names for Namespace metadata annotation.
    namespace_annotation_fields: namespace_metadata_annotator::FieldsSpec,

    /// A list of glob patterns to exclude from reading the files.
    exclude_paths_glob_patterns: Vec<PathBuf>,

    /// Max amount of bytes to read from a single file before switching over
    /// to the next file.
    /// This allows distribution the reads more or less evenly across the
    /// files.
    max_read_bytes: usize,

    /// The maximum number of a bytes a line can contain before being discarded.
    /// This protects against malformed lines or tailing incorrect files.
    max_line_bytes: usize,

    /// A field to use to set the timestamp when Vertex ingested the event.
    /// This is useful to compute the latency between important event processing
    /// stages, i.e. the time delta between log line was written and when it was
    /// processed by the `kubernetes_logs` source.
    ingestion_timestamp_field: Option<String>,

    /// The default timezone for timestamps without an explicit zone
    timezone: Option<TimeZone>,

    /// How long to delay removing entries from our map when we receive a deletion
    /// event from the watched stream.
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
    delay_deletion: Duration,
}

impl GenerateConfig for Config {
    fn generate_config() -> String {
        todo!()
    }
}

inventory::submit! {
    SourceDescription::new::<Config>("kubernetes_logs")
}

#[async_trait]
#[typetag::serde(name = "kubernetes_logs")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        todo!()
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }

    fn source_type(&self) -> &'static str {
        "kubernetes_logs"
    }
}

#[derive(Clone)]
struct LogSource {
    client: kubernetes::client::Client,
    data_dir: PathBuf,
    max_read_bytes: usize,
    max_line_bytes: usize,
    ingestion_timestamp_field: Option<String>,
}

impl LogSource {
    fn new(
        config: &Config,
        globals: &GlobalOptions,
        key: &ComponentKey,
        proxy: &ProxyConfig,
    ) -> crate::Result<Self> {
        todo!()
    }

    async fn run(self, mut output: Pipeline, shutdown: ShutdownSignal) -> crate::Result<()> {
        let LogSource {
            client,
            data_dir,
            max_read_bytes,
            max_line_bytes,
            ingestion_timestamp_field,
            ..
        } = self;

        // Start watching pod
        let watcher =
            kubernetes::watch::Watcher::new(client.clone(), Pod::watch_pod_for_all_namespaces);
        let (state_reader, state_writer) = evmap::new();
        let state_writer = kubernetes::state::evmap::Writer::new(
            state_writer,
            Some(Duration::from_millis(10)),
            HashKey::Uid,
        );
        // let state_writer = kubernetes::state::

        // let mut reflector = kubernetes::

        let provider = KubernetesPathsProvider::new(state_reader.clone(), state_writer.clone());
        let checkpointer = Checkpointer::new(&data_dir);
        let harvester = tail::Harvester {
            provider,
            read_from: Default::default(),
            max_read_bytes,
            handle: tokio::runtime::Handle::current(),
            ignore_before: None,
            max_line_bytes,
            line_delimiter: Default::default(),
        };

        let (file_source_tx, file_source_rx) = futures::channel::mpsc::channel::<Vec<Line>>(2);
        let checkpoints = checkpointer.view();
        let events = file_source_rx
            .map(futures::stream::iter)
            .flatten()
            .map(move |line| {
                let bytes = line.text.len();
                counter!("component_received_bytes_total", bytes as u64);

                let mut event = create_event(
                    line.text,
                    &line.filename,
                    ingestion_timestamp_field.as_deref(),
                );

                // TODO: enrich event

                checkpoints.update(line.fingerprint, line.offset);
                event
            });

        tokio::task::spawn_blocking(move || {
            let result = harvester.run(file_source_tx, shutdown, checkpointer);

            result.unwrap();
        })
        .await;

        Ok(())
    }
}

const FILE_KEY: Key = Key::from_static_str("file");

fn create_event(line: Bytes, file: &str, ingestion_timestamp_field: Option<&str>) -> Event {
    let mut log = LogRecord::from(line);

    // Add source type
    log.insert_tag(log_schema().source_type_key(), "kubernetes_log");

    // Add file
    log.insert_tag(FILE_KEY, file.to_owned());

    // Add ingestion timestamp if requested
    let now = Utc::now();
    if let Some(ingestion_timestamp_field) = ingestion_timestamp_field {
        log.insert_field(ingestion_timestamp_field, now);
    }

    log.try_insert_field(log_schema().timestamp_key(), now);

    log.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>();
    }

    #[test]
    fn prepare_exclude_paths() {}
}
