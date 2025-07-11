mod annotator;
mod pod;
mod provider;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use annotator::{FieldsSpec, PodMetadataAnnotator};
use bytes::Bytes;
use bytesize::ByteSizeOf;
use chrono::Utc;
use configurable::configurable_component;
use event::LogRecord;
use event::log::{OwnedTargetPath, TargetPath, Value, path};
use framework::config::{Output, SourceConfig, SourceContext, default_true};
use framework::timezone::TimeZone;
use framework::{Pipeline, ShutdownSignal, Source};
use futures::{StreamExt, TryFutureExt};
use kubernetes::{Client, Event, WatchConfig, watcher};
use log_schema::log_schema;
use pod::Pod;
use provider::KubernetesPathsProvider;
use tail::{Checkpointer, Line};

pub type Store = Arc<RwLock<BTreeMap<String, Pod>>>;

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

/// Collects Pod logs from Kubernetes Nodes, automatically enriching data with
/// metadata via the Kubernetes API.
///
/// Kubernetes version >= 1.22 is required.
///
/// This source requires read access to the `/var/log/pods` directory. When run
/// in a Kubernetes cluster this can be provided with a `hostPath` volume.
#[configurable_component(source, name = "kubernetes_logs")]
pub struct Config {
    /// Specifies the label selector to filter `Pod`s with, to be used in
    /// addition to the built-in `vertex.io/exclude` filter
    label_selector: Option<String>,

    /// The `name` of the Kubernetes `Node` that Vertex runs at.
    /// Required to filter the `Pod`s to only include the ones with the
    /// log files accessible locally.
    self_node_name: Option<String>,

    /// Specifies the field selector to filter `Pod`s with, to be used in
    /// addition to the built-in `Node` filter.
    field_selector: Option<String>,

    /// Automatically merge partial events.
    #[serde(default = "default_true")]
    auto_partial_merge: bool,

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
    annotation_fields: FieldsSpec,

    /// The default timezone for timestamps without an explicit zone
    #[serde(default)]
    timezone: Option<TimeZone>,

    /// How long to delay removing entries from our map when we receive a deletion
    /// event from the watched stream.
    #[serde(default = "default_delay_deletion", with = "humanize::duration::serde")]
    delay_deletion: Duration,
}

impl Config {
    fn prepare_field_selector(&self) -> crate::Result<String> {
        let node_name = match &self.self_node_name {
            Some(key) => std::env::var(key).map_err(|_err| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("environment variable `{key}` not set"),
                )
            })?,
            None => std::env::var("NODE_NAME").map_err(|_err| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "default environment variable `NODE_NAME` not set",
                )
            })?,
        };

        let selector = match &self.field_selector {
            Some(extra) => format!("spec.nodeName={node_name},{extra}"),
            None => format!("spec.nodeName={node_name}"),
        };

        Ok(selector)
    }

    fn prepare_label_selector(&self) -> Option<String> {
        self.label_selector.as_ref().map(|extra| extra.to_string())
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "kubernetes_logs")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        let client = Client::new(None)?;
        let data_dir = cx.globals.make_subdir(cx.key.id())?;
        let field_selector = self.prepare_field_selector()?;
        let label_selector = self.prepare_label_selector();

        let log_source = LogSource {
            exclude_pattern: vec![],
            fields_spec: self.annotation_fields.clone(),
            data_dir,
            max_read_bytes: self.max_read_bytes,
            max_line_bytes: self.max_line_bytes,
            ingestion_timestamp_field: None,
            field_selector: Some(field_selector),
            label_selector,
        };

        Ok(Box::pin(
            log_source
                .run(client, cx.output, cx.shutdown)
                .map_err(|err| error!(message = "Kubernetes log source failed", %err)),
        ))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::logs()]
    }

    // TODO: change to true, if checkpoint implement
    fn can_acknowledge(&self) -> bool {
        false
    }
}

struct LogSource {
    field_selector: Option<String>,
    label_selector: Option<String>,
    exclude_pattern: Vec<glob::Pattern>,
    fields_spec: FieldsSpec,
    data_dir: PathBuf,
    max_read_bytes: usize,
    max_line_bytes: usize,
    ingestion_timestamp_field: Option<OwnedTargetPath>,
}

impl LogSource {
    async fn run(
        self,
        client: Client,
        mut output: Pipeline,
        shutdown: ShutdownSignal,
    ) -> crate::Result<()> {
        let LogSource {
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
        let store = Arc::new(RwLock::new(BTreeMap::new()));
        tokio::spawn(watch(
            client,
            Arc::clone(&store),
            label_selector,
            field_selector,
            shutdown.clone(),
        ));

        let provider = KubernetesPathsProvider::new(Arc::clone(&store), exclude_pattern);
        let pod_annotator = PodMetadataAnnotator::new(store, fields_spec);
        let checkpointer = Checkpointer::new(&data_dir);
        let harvester = tail::Harvester {
            provider,
            scan_minimum_cooldown: Duration::from_secs(1),
            read_from: Default::default(),
            max_read_bytes,
            handle: tokio::runtime::Handle::current(),
            ignore_before: None,
            max_line_bytes,
            line_delimiter: Bytes::from("\n"),
        };

        let checkpoints = checkpointer.view();

        let (tx, rx) = futures::channel::mpsc::channel::<Vec<Line>>(16);
        let mut stream = rx.map(move |lines| {
            let mut logs = Vec::with_capacity(lines.len());

            for line in lines {
                let mut log = create_log(
                    line.text,
                    &line.filename,
                    ingestion_timestamp_field.as_ref(),
                );

                match pod_annotator.annotate(&mut log, &line.filename) {
                    Some(file_info) => {
                        let attrs = metrics::Attributes::from([(
                            "pod",
                            file_info.pod_name.to_string().into(),
                        )]);

                        metrics::register_counter("component_received_events_total", "")
                            .recorder(attrs.clone())
                            .inc(1);
                        metrics::register_counter("component_received_bytes_total", "")
                            .recorder(attrs)
                            .inc(log.size_of() as u64);
                    }
                    None => {
                        // TODO: metrics
                        // counter!("component_received_events_total", 1);
                        // counter!("component_received_bytes_total", log.size_of() as u64);

                        // counter!("kubernetes_event_annotation_failures_total", 1);
                        error!(
                            message = "Failed to annotate log with pod metadata",
                            file = line.filename.as_str(),
                            internal_log_rate_limit = true
                        );
                    }
                }

                checkpoints.update(line.fingerprint, line.offset);

                logs.push(log);
            }

            logs
        });

        // send events
        tokio::spawn(async move { output.send_stream(&mut stream).await });

        // `spawn_blocking` will run this closure in another thread, so our tokio
        // workers wouldn't be blocked.
        tokio::task::spawn_blocking(move || {
            harvester
                .run(tx, shutdown.clone(), shutdown.clone(), checkpointer)
                .unwrap()
        })
        .await?;

        Ok(())
    }
}

fn create_log(
    line: Bytes,
    file: &str,
    ingestion_timestamp_field: Option<&OwnedTargetPath>,
) -> LogRecord {
    let mut log = match serde_json::from_slice::<Value>(line.as_ref()) {
        Ok(value) => match value {
            Value::Object(map) => LogRecord::from(map),
            _ => LogRecord::from(line),
        },
        Err(err) => {
            // TODO: metrics
            warn!(
                message = "Parse kubernetes container logs failed",
                ?err,
                internal_log_rate_limit = true
            );
            LogRecord::from(line)
        }
    };

    // Add source type
    log.insert_metadata(
        log_schema().source_type_key().value_path(),
        "kubernetes_log",
    );

    // Add file
    log.insert_metadata(path!("file"), file.to_owned());

    // Add ingestion timestamp if requested
    let now = Utc::now();
    if let Some(ingestion_timestamp_field) = ingestion_timestamp_field {
        log.insert(ingestion_timestamp_field, now);
    }

    log.try_insert(log_schema().timestamp_key(), now);

    log
}

async fn watch(
    client: Client,
    store: Store,
    label_selector: Option<String>,
    field_selector: Option<String>,
    mut shutdown: ShutdownSignal,
) {
    let config = WatchConfig {
        label_selector,
        field_selector,
        bookmark: true,
        ..Default::default()
    };

    let stream = watcher::<Pod>(client, config);
    tokio::pin!(stream);

    let mut new_store = None;
    loop {
        let event = tokio::select! {
            _ = &mut shutdown => break,
            result = stream.next() => match result {
                Some(Ok(event)) => event,
                Some(Err(err)) => {
                    warn!(message = "wait next event failed", ?err);
                    return;
                },
                None => break,
            }
        };

        match event {
            Event::Apply(pod) => {
                store.write().unwrap().insert(pod.metadata.uid.clone(), pod);
            }
            Event::Deleted(pod) => {
                store.write().unwrap().remove(&pod.metadata.uid);
            }
            Event::Init => {
                new_store = Some(BTreeMap::new());
            }
            Event::InitApply(pod) => {
                if let Some(store) = &mut new_store {
                    store.insert(pod.metadata.uid.clone(), pod);
                }
            }
            Event::InitDone => {
                if let Some(new) = new_store.take() {
                    *store.write().unwrap() = new;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }
}
