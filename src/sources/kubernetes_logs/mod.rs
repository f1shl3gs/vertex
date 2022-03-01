mod provider;
mod pod_metadata_annotator;
mod namespace_metadata_annotator;

use std::path::PathBuf;
use std::time::Duration
;
use serde::{Deserialize, Serialize};
use framework::config::{ComponentKey, DataType, GenerateConfig, GlobalOptions, Output, ProxyConfig, SourceConfig, SourceContext};
use framework::{Pipeline, ShutdownSignal, Source};
use framework::timezone::TimeZone;

use crate::kubernetes;

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
    #[serde(deserialize_with = "deserialize_duration", serialize_with = "serialize_duration")]
    delay_deletion: Duration
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
}

impl LogSource {
    fn new(
        config: &Config,
        globals: &GlobalOptions,
        key: &ComponentKey,
        proxy: &ProxyConfig
    ) -> crate::Result<Self> {

    }

    async fn run(
        self,
        mut output: Pipeline,
        shutdown: ShutdownSignal,
    ) -> crate::Result<()> {

    }
}