mod cri;
mod provider;

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::Duration;

use configurable::{Configurable, configurable_component};
use cri::{Cri, Error as ParseError, Stream};
use event::LogRecord;
use framework::config::{OutputType, SourceConfig, SourceContext};
use framework::{Pipeline, Source};
use futures::{FutureExt, StreamExt};
use metrics::register_counter;
use provider::{Metadata, ProviderConfig};
use serde::{Deserialize, Serialize};
use tail::decode::NewlineDecoder;
use tail::multiline::Multiline;
use tail::{Checkpointer, Conveyor, FileReader, ReadFrom, ReadyFrames, Shutdown, harvest};
use tokio_util::codec::FramedRead;
use value::{OwnedValuePath, Value, owned_value_path};

#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
pub struct PodFieldsConfig {
    name: Option<OwnedValuePath>,
    namespace: Option<OwnedValuePath>,
    uid: Option<OwnedValuePath>,
    ip: Option<OwnedValuePath>,
    ips: Option<OwnedValuePath>,
    labels: Option<OwnedValuePath>,
    annotations: Option<OwnedValuePath>,
    node_name: Option<OwnedValuePath>,
}

impl Default for PodFieldsConfig {
    fn default() -> Self {
        Self {
            name: Some(owned_value_path!("pod", "name")),
            namespace: Some(owned_value_path!("pod", "namespace")),
            uid: Some(owned_value_path!("pod", "uid")),
            ip: Some(owned_value_path!("pod", "ip")),
            ips: Some(owned_value_path!("pod", "ips")),
            labels: Some(owned_value_path!("pod", "labels")),
            annotations: Some(owned_value_path!("pod", "annotations")),
            node_name: Some(owned_value_path!("pod", "node_name")),
        }
    }
}

#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
pub struct ContainerFieldsConfig {
    name: Option<OwnedValuePath>,
    image: Option<OwnedValuePath>,
}

impl Default for ContainerFieldsConfig {
    fn default() -> Self {
        ContainerFieldsConfig {
            name: Some(owned_value_path!("container", "name")),
            image: Some(owned_value_path!("container", "image")),
        }
    }
}

#[derive(Clone, Configurable, Debug, Default, Deserialize, Serialize)]
pub struct FieldsConfig {
    pod: PodFieldsConfig,
    container: ContainerFieldsConfig,
}

impl FieldsConfig {
    pub fn build(&self, metadata: &Metadata) -> Value {
        let Metadata { pod, container } = metadata;

        let mut value = Value::Object(Default::default());

        // pod info
        if let Some(path) = &self.pod.name {
            value.insert(path, pod.metadata.name.clone());
        }
        if let Some(path) = &self.pod.namespace {
            value.insert(path, pod.metadata.namespace.clone());
        }
        if let Some(path) = &self.pod.uid {
            value.insert(path, pod.metadata.uid.clone());
        }
        if let Some(path) = &self.pod.ip {
            value.insert(path, pod.status.pod_ip.clone());
        }
        if let Some(path) = &self.pod.ips {
            value.insert(
                path,
                pod.status
                    .pod_ips
                    .iter()
                    .map(|item| item.ip.clone())
                    .collect::<Vec<_>>(),
            );
        }
        if let Some(path) = &self.pod.labels {
            value.insert(
                path,
                pod.metadata
                    .labels
                    .iter()
                    .map(|(key, value)| (key.clone(), Value::from(value)))
                    .collect::<BTreeMap<_, _>>(),
            );
        }
        if let Some(path) = &self.pod.annotations {
            value.insert(
                path,
                pod.metadata
                    .annotations
                    .iter()
                    .map(|(key, value)| (key.clone(), Value::from(value)))
                    .collect::<BTreeMap<_, _>>(),
            );
        }
        if let Some(path) = &self.pod.node_name {
            value.insert(path, pod.spec.node_name.clone());
        }

        // container
        if let Some(path) = &self.container.name {
            value.insert(path, container.name.clone());
        }
        if let Some(path) = &self.container.image {
            value.insert(path, container.image.clone());
        }

        value
    }
}

/// Collects Pod logs from Kubernetes Nodes, automatically enriching data with
/// metadata via the Kubernetes API.
///
/// Kubernetes version >= 1.22 is required.
///
/// This source requires read access to the `/var/log/pods` directory. When run
/// in a Kubernetes cluster this can be provided with a `hostPath` volume.
#[configurable_component(source, name = "kubernetes_logs")]
struct Config {
    provider: ProviderConfig,

    /// Configuration for how the events are enriched with Pod metadata.
    #[serde(default)]
    fields: FieldsConfig,

    /// Reads from the specified streams
    #[serde(default)]
    stream: Stream,
}

#[async_trait::async_trait]
#[typetag::serde(name = "kubernetes_logs")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let path = cx.globals.make_subdir(cx.key.id())?;
        let checkpointer = Checkpointer::load(path)?;
        let provider = self.provider.build(checkpointer.view())?;

        let output = SendOutput {
            fields: self.fields.clone(),
            stream: self.stream.clone(),
            output: cx.output,
        };
        let shutdown = cx.shutdown.map(|_| ());

        Ok(Box::pin(async move {
            if let Err(err) = harvest(
                provider,
                ReadFrom::Beginning,
                checkpointer,
                output,
                shutdown,
            )
            .await
            {
                error!(message = "harvest kubernetes logs failed", ?err);
            }

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::log()]
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

#[derive(Clone)]
struct SendOutput {
    fields: FieldsConfig,
    stream: Stream,
    output: Pipeline,
}

impl Conveyor for SendOutput {
    type Metadata = Metadata;

    fn run(
        &self,
        reader: FileReader,
        metadata: Self::Metadata,
        offset: Arc<AtomicU64>,
        mut shutdown: Shutdown,
    ) -> impl Future<Output = Result<(), ()>> + Send + 'static {
        let mut output = self.output.clone();
        let stream = self.stream.clone();
        let base = self.fields.build(&metadata);

        let path = reader.path().to_string_lossy().to_string();

        let framed = FramedRead::new(reader, NewlineDecoder::new(4 * 1024));
        let merged =
            Multiline::new(framed, Cri::default(), Duration::from_millis(200)).map(move |result| {
                match result {
                    Ok((data, size)) => {
                        let (timestamp, stream, msg) = cri::parse(data, &stream)?;

                        let mut value = base.clone();
                        value.insert("timestamp", timestamp);
                        value.insert("stream", stream);
                        value.insert("message", msg);

                        Ok((LogRecord::from(value), size))
                    }
                    Err(err) => Err(ParseError::Frame(err)),
                }
            });

        let mut stream = ReadyFrames::new(merged, 128, 4 * 1024 * 1024);

        let attrs = [
            ("path", Cow::Owned(path.clone())),
            ("namespace", Cow::Owned(metadata.pod.metadata.namespace)),
            ("pod", Cow::Owned(metadata.pod.metadata.name)),
            ("container", Cow::Owned(metadata.container.name)),
            ("node_name", Cow::Owned(metadata.pod.spec.node_name)),
        ];
        let bytes = register_counter("k8s_logs_read_bytes", "the total bytes read by kubernetes")
            .recorder(attrs.clone());
        let events = register_counter(
            "k8s_logs_processed_events",
            "the total number of events processed",
        )
        .recorder(attrs);

        Box::pin(async move {
            loop {
                let (logs, size) = tokio::select! {
                    _ = &mut shutdown => break,
                    result = stream.next() => match result {
                        Some(Ok(batched)) => batched,
                        Some(Err(err)) => {
                            warn!(message = "process kubernetes logs failed", %err);
                            continue;
                        },
                        None => break,
                    }
                };

                let num = logs.len();
                if let Err(_err) = output.send(logs).await {
                    break;
                }

                bytes.inc(size as u64);
                events.inc(num as u64);

                offset.fetch_add(size as u64, std::sync::atomic::Ordering::Relaxed);
            }

            Ok(())
        })
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
