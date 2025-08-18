mod multiline;
mod provider;
mod transcode;

use std::collections::BTreeMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::Duration;

use configurable::{Configurable, configurable_component};
use encoding_rs::Encoding;
use event::LogRecord;
use framework::config::{OutputType, Resource, SourceConfig, SourceContext};
use framework::{Pipeline, Source};
use futures::{FutureExt, StreamExt};
use multiline::MergeLogic;
use provider::{GlobProvider, Ordering};
use serde::{Deserialize, Deserializer, Serialize};
use tail::decode::BytesDelimitDecoder;
use tail::multiline::Multiline;
use tail::{Checkpointer, Conveyor, FileReader, ReadyFrames, Shutdown, harvest};
use tokio_util::codec::FramedRead;
use transcode::{Decoder, Encoder};

/// Where to start reader for a file which is never read
#[derive(Configurable, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum ReadFrom {
    #[default]
    Beginning,

    End,
}

fn default_scan_interval() -> Duration {
    Duration::from_secs(10)
}

#[derive(Configurable, Debug, Deserialize, Serialize)]
struct ScanConfig {
    /// How often the component checks for new files
    #[serde(default = "default_scan_interval", with = "humanize::duration::serde")]
    interval: Duration,

    /// If this is set, scanner ignores any files that were modified before the
    /// specified timespan. This is very useful if you keep log files for a long
    /// time.
    #[serde(default, with = "humanize::duration::serde_option")]
    ignore_older_than: Option<Duration>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            interval: default_scan_interval(),
            ignore_older_than: None,
        }
    }
}

// this function do not just deserialize `Ordering`, validation is done also,
// so invalid error will be raised when deserializing
fn deserialize_ordering<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Ordering>, D::Error> {
    let ordering: Option<Ordering> = Deserialize::deserialize(deserializer)?;

    match ordering {
        Some(ordering) => {
            if let Err(err) = ordering.validate() {
                return Err(serde::de::Error::custom(err));
            }

            Ok(Some(ordering))
        }
        None => Ok(None),
    }
}

/// This source reads every matched file in the `include` pattern. And this can
/// have a history of tracked files and a state of offsets. This helps resume a
/// state if the service is restarted.
///
/// Vertex is able to track files correctly in the following strategies:
/// - CREATE: new active file with a unique name is created on rotation
/// - RENAME: rotated files are renamed (with some special prefix/suffix)
/// - COPY_TRUNCATE: not support
///
/// When dealing with file rotation, avoid harvesting symlinks
#[configurable_component(source, name = "tail")]
struct Config {
    /// Array of file patterns to include. glob is supported. Watching rotated
    /// files is not necessary, and vertex can handle it properly.
    include: Vec<String>,

    /// Array of file patterns to exclude. glob is supported.
    ///
    /// Takes precedence over the `include` option. Note: The `exclude` patterns are applied
    /// _after_ the attempt to glob everything in `include`. This means that all files are
    /// first matched by `include` and then filtered by the `exclude` patterns. This can be
    /// impactful if `include` contains directories with contents that are not accessible.
    #[serde(default)]
    exclude: Vec<String>,

    /// File position to use when reading a new file.
    #[serde(default)]
    read_from: ReadFrom,

    /// Config the behavior of scanner, which scans the filesystem periodically and
    /// return the files to tail
    #[serde(default)]
    scan: ScanConfig,

    /// Multiline aggregation configuration. If not specified, multiline aggregation is disabled.
    #[serde(default)]
    multiline: Option<multiline::Config>,

    #[serde(default, deserialize_with = "deserialize_ordering")]
    ordering: Option<Ordering>,

    /// Encoding of the file
    #[serde(default)]
    charset: Option<&'static Encoding>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "tail")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let path = cx.globals.make_subdir(&format!("{}/checkpoints", cx.key))?;
        let checkpointer = Checkpointer::load(path)?;
        let read_from = match self.read_from {
            ReadFrom::Beginning => tail::ReadFrom::Beginning,
            ReadFrom::End => tail::ReadFrom::End,
        };

        let provider = match GlobProvider::new(
            self.include.clone(),
            &self.exclude,
            self.scan.interval,
            self.ordering.clone(),
            self.scan.ignore_older_than,
            checkpointer.view(),
        ) {
            Ok(provider) => provider,
            Err(err) => {
                return Err(format!("invalid exclude pattern, {err}").into());
            }
        };

        let delimiter = match self.charset {
            Some(charset) => {
                let mut encoder = Encoder::new(charset);
                encoder.encode_from_utf8("\n").to_vec()
            }
            None => b"\n".to_vec(),
        };

        let (logic, timeout) = match &self.multiline {
            None => (MergeLogic::None, Duration::from_millis(200)),
            Some(config) => (config.mode.build()?, config.timeout),
        };
        let output = OutputSender {
            delimiter,
            encoding: self.charset,
            output: cx.output,
            logic,
            timeout,
        };

        let shutdown = cx.shutdown.map(|_| ());
        Ok(Box::pin(async move {
            if let Err(err) = harvest(provider, read_from, checkpointer, output, shutdown).await {
                error!(message = "harvest log files failed", ?err);
            }

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::log()]
    }

    fn resources(&self) -> Vec<Resource> {
        vec![]
    }

    fn can_acknowledge(&self) -> bool {
        // TODO: enable this
        false
    }
}

struct OutputSender {
    delimiter: Vec<u8>,
    encoding: Option<&'static Encoding>,
    logic: MergeLogic,
    timeout: Duration,

    output: Pipeline,
}

impl Conveyor for OutputSender {
    type Metadata = BTreeMap<String, String>;

    fn run(
        &self,
        reader: FileReader,
        _meta: Self::Metadata,
        offset: Arc<AtomicU64>,
        mut shutdown: Shutdown,
    ) -> impl Future<Output = Result<(), ()>> + Send + 'static {
        // TODO: add metrics:
        // - files_opened_total
        // - files_closed_total
        // - files_active
        // - read_lines_total
        // - read_bytes_total
        // - process_errors_total
        // - process_event_total

        let mut decoder = self.encoding.map(Decoder::new);
        let framed = FramedRead::new(reader, BytesDelimitDecoder::new(&self.delimiter, 4 * 1024))
            .map(move |result| match &mut decoder {
                Some(d) => result.map(|(data, size)| (d.decode(data), size)),
                None => result,
            });

        let merged = Multiline::new(framed, self.logic.clone(), self.timeout);

        let mut stream = ReadyFrames::new(merged, 128, 4 * 1024 * 1024);
        let mut output = self.output.clone();

        Box::pin(async move {
            use std::sync::atomic::Ordering;

            loop {
                let (lines, size) = tokio::select! {
                    _ = &mut shutdown => break,
                    result = stream.next() => match result {
                        Some(Ok(batched)) => batched,
                        Some(Err(err)) => {
                            error!(message = "decode failed", ?err);
                            continue;
                        },
                        None => break,
                    }
                };

                let logs = lines.into_iter().map(LogRecord::from).collect::<Vec<_>>();
                if let Err(_err) = output.send(logs).await {
                    break;
                }

                offset.fetch_add(size as u64, Ordering::Relaxed);
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

    #[test]
    fn validate_ordering() {
        let input = r#"
include:
  - /path/to/*/*.log
ordering:
  pattern: /path/to/(?<app>\S+)/(?<name>\S+).log
  group_by: app
  sort:
    by:
    - name
        "#;

        let _config = serde_yaml::from_str::<Config>(input).unwrap();
    }
}
