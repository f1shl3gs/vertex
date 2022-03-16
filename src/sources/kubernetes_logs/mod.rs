mod annotator;
mod provider;
mod reflector;

use std::path::PathBuf;
use std::time::Duration;

use annotator::FieldsSpec;
use async_trait::async_trait;
use bytes::Bytes;
use chrono::Utc;
use event::attributes::Key;
use event::log::Value;
use event::LogRecord;
use framework::config::{
    default_true, deserialize_duration, serialize_duration, DataType, GenerateConfig, Output,
    SourceConfig, SourceContext, SourceDescription,
};
use framework::timezone::TimeZone;
use framework::{Pipeline, ShutdownSignal, Source};
use futures_util::{StreamExt, TryFutureExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use kube::api::ListParams;
use kube::runtime::watcher;
use kube::{Api, Client};
use log_schema::log_schema;
use provider::KubernetesPathsProvider;
use serde::{Deserialize, Serialize};
use shared::ByteSizeOf;
use tail::{Checkpointer, Line};

const fn default_max_read_bytes() -> usize {
    2 * 1024
}

const fn default_max_line_bytes() -> usize {
    // The 16KB is the maximum size of the payload at single line for both
    // docker and CRI log formats.
    // We take a double of that to account for metadata and padding, and to
    // have a power of two rounding. Line splitting is countered at the parser,
    32 * 1024
}

fn default_path_exclusion() -> Vec<PathBuf> {
    vec![PathBuf::from("**/*.gz"), PathBuf::from("**/*.tmp")]
}

const fn default_delay_deletion() -> Duration {
    Duration::from_secs(60)
}

/// Configuration for the `kubernetes_logs` source.
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    /// Specifies the label selector to filter `Pod`s with, to be used in
    /// addition to the built-in `vertex.io/exclude` filter
    extra_label_selector: Option<String>,

    /// The `name` of the Kubernetes `Node` that Vertex runs at.
    /// Required to filter the `Pod`s to only include the ones with the
    /// log files accessible locally.
    self_node_name: Option<String>,

    /// Specifies the field selector to filter `Pod`s with, to be used in
    /// addition to the built-in `Node` filter.
    extra_field_selector: Option<String>,

    /// Automatically merge partial events.
    #[serde(default = "default_true")]
    auto_partial_merge: bool,

    /// Override global data_dir
    data_dir: Option<PathBuf>,

    /// A list of glob patterns to exclude from reading the files.
    #[serde(default = "default_path_exclusion")]
    exclude_paths_glob_patterns: Vec<PathBuf>,

    /// Max amount of bytes to read from a single file before switching over
    /// to the next file.
    /// This allows distribution the reads more or less evenly across the
    /// files.
    #[serde(default = "default_max_read_bytes")]
    max_read_bytes: usize,

    /// The maximum number of a bytes a line can contain before being discarded.
    /// This protects against malformed lines or tailing incorrect files.
    #[serde(default = "default_max_line_bytes")]
    max_line_bytes: usize,

    /// A field to use to set the timestamp when Vertex ingested the event.
    /// This is useful to compute the latency between important event processing
    /// stages, i.e. the time delta between log line was written and when it was
    /// processed by the `kubernetes_logs` source.
    #[serde(default)]
    ingestion_timestamp_field: Option<String>,

    /// Specifies the field names for Pod metadata annotation.
    #[serde(default)]
    annotation_fields: annotator::FieldsSpec,

    /// The default timezone for timestamps without an explicit zone
    #[serde(default)]
    timezone: Option<TimeZone>,

    /// How long to delay removing entries from our map when we receive a deletion
    /// event from the watched stream.
    #[serde(
        default = "default_delay_deletion",
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
    delay_deletion: Duration,
}

impl GenerateConfig for Config {
    fn generate_config() -> String {
        format!(
            r#"
# Specifies the label selector to filter Pods with.
#
# extra_label_selector: app=foo

# The name of the Kubernetes Node this Vector instance runs at. Configured to
# use an env var by default, to be evaluated to a value provided by Kubernetes
# at Pod deploy time.
#
# self_node_name: ${{VERTEX_NODE_NAME}}

# Specifies the field selector to filter Pod with, to be used in addition to the
# built-in `Node` filter. The name of the Kubernetes Node this Vertex instance
# runs at. Configured to use an env var by default, to be evaluated to a value
# provided by Kubernetes at Pod deploy time.
#
# extra_field_selector: foo=bar

# Automatically merge partial messages into a single event. Partial here is in
# respect to messages that were split by the Kubernetes Container Runtime log
# driver
#
# default: true
#
# auto_partial_merge: true

# The directory used to persist file checkpoint positions. By default, the global
# `data_dir` option is used. Please make sure the Vertex instance has write
# permissions to this dir.
#
# data_dir: /path/to/your/data_dir

# A list of glob patterns to exclude from reading the files.
#
# exclude_paths_glob_patterns:
# - "**/*.gz"
# - "**/*.tmp"

# THe maximum number of bytes a line can contain before being discarded.
# This protects against malformed lines or tailing incorrect files.
#
# max_line_bytes: {}

# An approximate limit on the amount of data read from a single pod log file
# at a given time.
#
# max_read_bytes: {}

# The exact time the event was ingested into Vertex.
#
# ingestion_timestamp_field: "ingestion_timestamp"

# Configuration for how the events are annotated with Pod metadata.
#
# annotation_fields:
{}

# The name of the time zone to apply to timestamp conversions that do not contain
# an explicit time zone. This overrides the global timezone option. The time zone
# name may be any name in the TZ database, or `local` to indicate system local time.
#
timezone: local

"#,
            humanize::ibytes(default_max_line_bytes()),
            humanize::ibytes(default_max_read_bytes()),
            FieldsSpec::generate_commented_with_indent(2)
        )
    }
}

inventory::submit! {
    SourceDescription::new::<Config>("kubernetes_logs")
}

impl Config {
    fn prepare_field_selector(&self) -> crate::Result<String> {
        let node_name = match &self.self_node_name {
            Some(key) => std::env::var(key),
            None => std::env::var("VERTEX_NODE_NAME"),
        }?;

        let selector = match &self.extra_field_selector {
            Some(extra) => format!("spec.nodeName={},{}", node_name, extra),
            None => format!("spec.nodeName={}", node_name),
        };

        Ok(selector)
    }

    fn prepare_label_selector(&self) -> Option<String> {
        self.extra_label_selector
            .as_ref()
            .map(|extra| extra.to_string())
    }
}

#[async_trait]
#[typetag::serde(name = "kubernetes_logs")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        let data_dir = cx.globals.make_subdir(cx.key.id())?;
        let field_selector = self.prepare_field_selector()?;
        let label_selector = self.prepare_label_selector();

        let log_source = LogSource {
            source: cx.key.to_string(),
            exclude_pattern: vec![],
            fields_spec: self.annotation_fields.clone(),
            data_dir,
            max_read_bytes: self.max_read_bytes,
            max_line_bytes: self.max_line_bytes,
            ingestion_timestamp_field: None,
            field_selector: Some(field_selector),
            label_selector,
        };

        Ok(Box::pin(log_source.run(cx.output, cx.shutdown).map_err(
            |err| error!(message = "Kubernetes log source failed", ?err),
        )))
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
    source: String,
    field_selector: Option<String>,
    label_selector: Option<String>,
    exclude_pattern: Vec<glob::Pattern>,
    fields_spec: FieldsSpec,
    data_dir: PathBuf,
    max_read_bytes: usize,
    max_line_bytes: usize,
    ingestion_timestamp_field: Option<String>,
}

impl LogSource {
    async fn run(self, mut output: Pipeline, shutdown: ShutdownSignal) -> crate::Result<()> {
        let LogSource {
            source,
            field_selector,
            label_selector,
            exclude_pattern,
            fields_spec,
            data_dir,
            max_read_bytes,
            max_line_bytes,
            ingestion_timestamp_field,
            ..
        } = self;

        // Build kubernetes pod store, indexed by uuid
        let client = Client::try_default().await?;
        let api: Api<Pod> = Api::all(client);
        let store = reflector::Store::new();

        // shutdown background task when this source shutdown
        let rfl_shutdown = shutdown.clone();
        let rfl_store = store.clone();
        tokio::spawn(async move {
            info!(
                message = "Obtained Kubernetes Node name to collect logs for (self)",
                label_selector = ?&label_selector,
                field_selector = ?&field_selector,
            );

            let list_params = ListParams {
                field_selector,
                label_selector,
                ..Default::default()
            };

            let mut rfl = reflector::reflector(rfl_store, watcher(api, list_params))
                .take_until(rfl_shutdown)
                .boxed();

            while let Some(_event) = rfl.try_next().await? {
                // TODO: metric
            }

            Ok::<(), kube::runtime::watcher::Error>(())
        });

        let provider = KubernetesPathsProvider::new(store.clone(), exclude_pattern).await?;
        let pod_annotator = annotator::PodMetadataAnnotator::new(store, fields_spec);
        let checkpointer = Checkpointer::new(&data_dir);
        let harvester = tail::Harvester {
            provider,
            read_from: Default::default(),
            max_read_bytes,
            handle: tokio::runtime::Handle::current(),
            ignore_before: None,
            max_line_bytes,
            line_delimiter: Bytes::from("\n"),
        };

        let checkpoints = checkpointer.view();

        let (tx, rx) = futures::channel::mpsc::channel::<Vec<Line>>(16);
        let mut events = rx.map(futures::stream::iter).flatten().map(move |line| {
            let bytes = line.text.len();
            counter!("component_received_bytes_total", bytes as u64, "source" => source.clone());

            let mut log = create_log(
                line.text,
                &line.filename,
                ingestion_timestamp_field.as_deref(),
            );

            match pod_annotator.annotate(&mut log, &line.filename) {
                Some(file_info) => {
                    counter!("component_received_events_total", 1, "pod" => file_info.pod_name.to_owned());
                    counter!("component_received_bytes_total", log.size_of() as u64, "pod" => file_info.pod_name.to_owned());
                }
                None => {
                    counter!("component_received_events_total", 1);
                    counter!("component_received_bytes_total", log.size_of() as u64);

                    counter!("kubernetes_event_annotation_failures_total", 1);
                    error!(
                        message = "Failed to annotate log with pod metadata",
                        file = line.filename.as_str(),
                        internal_log_rate_secs = 10
                    );
                }
            }

            checkpoints.update(line.fingerprint, line.offset);
            log.into()
        });

        tokio::spawn(async move { output.send_all(&mut events).await });

        tokio::task::spawn_blocking(move || {
            let result = harvester.run(tx, shutdown, checkpointer);

            result.unwrap();
        })
        .await?;

        Ok(())
    }
}

const FILE_KEY: Key = Key::from_static_str("file");

fn create_log(line: Bytes, file: &str, ingestion_timestamp_field: Option<&str>) -> LogRecord {
    let mut log = match serde_json::from_slice::<Value>(line.as_ref()) {
        Ok(value) => LogRecord::from(value),
        Err(err) => {
            // TODO: metrics

            warn!(
                message = "Parse kubernetes container logs failed",
                ?err,
                internal_log_rate_secs = 30
            );
            LogRecord::from(line)
        }
    };

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

    log
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>();
    }
}
