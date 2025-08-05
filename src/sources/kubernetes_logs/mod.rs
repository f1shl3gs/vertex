mod cri;
mod provider;

use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use configurable::configurable_component;
use cri::{Cri, Error as ParseError, Stream};
use event::LogRecord;
use framework::config::{Output, SourceConfig, SourceContext};
use framework::{Pipeline, Source};
use futures::{FutureExt, StreamExt};
use metrics::register_counter;
use tail::decode::NewlineDecoder;
use tail::{
    Checkpointer, Conveyor, FileReader, Multiline, ReadFrom, ReadyFrames, Shutdown, harvest,
};
use tokio::select;
use tokio_util::codec::FramedRead;
use value::Value;

use provider::{FieldsConfig, ProviderConfig};

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
        /*
                let node_name = match &self.node_name {
                    Some(node_name) => node_name.to_string(),
                    None => std::env::var("NODE_NAME").map_err(|_err| {
                        std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            "default environment variable `NODE_NAME` not set",
                        )
                    })?,
                };
                let field_selector = match &self.field_selector {
                    Some(extra) => format!("spec.nodeName={node_name},{extra}"),
                    None => format!("spec.nodeName={node_name}"),
                };
                let label_selector = self.label_selector.clone();

                let provider =
                    KubernetesProvider::new(label_selector, Some(field_selector), self.fields.clone())?;
        */

        let path = cx.globals.make_subdir(cx.key.id())?;
        let checkpointer = Checkpointer::load(path)?;

        let provider = self.provider.build(self.fields.clone())?;

        let output = SendOutput {
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

    fn outputs(&self) -> Vec<Output> {
        vec![Output::logs()]
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

#[derive(Clone)]
struct SendOutput {
    stream: Stream,

    output: Pipeline,
}

impl Conveyor for SendOutput {
    type Metadata = Value;

    fn run(
        &self,
        reader: FileReader,
        meta: Self::Metadata,
        offset: Arc<AtomicU64>,
        mut shutdown: Shutdown,
    ) -> impl Future<Output = Result<(), ()>> + Send + 'static {
        let mut output = self.output.clone();
        let stream = self.stream.clone();

        let path = reader.path().to_path_buf();
        let path = path.to_string_lossy().to_string();

        let framed = FramedRead::new(reader, NewlineDecoder::new(4 * 1024));
        let merged = Multiline::new(framed, Cri::default()).map(move |result| match result {
            Ok((data, size)) => {
                let (timestamp, stream, msg) = cri::parse(data, &stream)?;

                let mut value = meta.clone();
                value.insert("timestamp", timestamp);
                value.insert("stream", stream);
                value.insert("message", msg);

                Ok((LogRecord::from(value), size))
            }
            Err(err) => Err(ParseError::Frame(err)),
        });

        let mut stream = ReadyFrames::new(merged, 128, 4 * 1024 * 1024);

        let bytes = register_counter("k8s_logs_read_bytes", "the total bytes read by kubernetes")
            .recorder([("path", std::borrow::Cow::Owned(path.clone()))]);
        let events = register_counter(
            "k8s_logs_processed_events",
            "the total number of events processed",
        )
        .recorder([("path", std::borrow::Cow::Owned(path))]);

        Box::pin(async move {
            loop {
                let (logs, size) = select! {
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
