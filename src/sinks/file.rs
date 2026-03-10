use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::time::Duration;

use bytes::BytesMut;
use chrono::{DateTime, NaiveDateTime, Utc};
use codecs::encoding::{Framer, SinkType, Transformer};
use codecs::{Encoder, EncodingConfigWithFraming};
use configurable::{Configurable, configurable_component};
use event::{EventContainer, Events};
use finalize::{EventStatus, Finalizable};
use framework::config::{DataType, InputType, SinkConfig, SinkContext};
use framework::{Healthcheck, Sink, StreamSink};
use futures::stream::BoxStream;
use futures::{FutureExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio_util::codec::Encoder as _;

const fn default_max_size() -> usize {
    1024 * 1024 * 1024 // 1G
}

const fn default_max_files() -> usize {
    100
}

/// Option to rolling log files
#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
struct RotationConfig {
    /// The maximum size of the file before it gets rotated.
    #[serde(default = "default_max_size", with = "humanize::bytes::serde")]
    max_size: usize,

    /// The maximum number of old files to retain
    #[serde(default = "default_max_files")]
    max_files: usize,
}

impl RotationConfig {
    async fn rotate(&self, path: &Path) -> std::io::Result<File> {
        fn parse_timestamp(input: &str) -> Option<DateTime<Utc>> {
            let pos = input.len().checked_sub(23)?;
            let (_name, ts) = input.split_at(pos);

            NaiveDateTime::parse_from_str(ts, TIMESTAMP_FORMAT)
                .map(|n| n.and_utc())
                .ok()
        }

        // trying to clean up outdated
        let root = path.parent().unwrap_or(path);
        let mut files = Vec::with_capacity(self.max_files);
        for entry in root.read_dir()?.flatten() {
            if let Ok(typ) = entry.file_type()
                && !typ.is_file()
            {
                continue;
            }

            let path = entry.path();
            let Some(stem) = path.file_stem() else {
                continue;
            };
            let Some(timestamp) = parse_timestamp(stem.to_string_lossy().as_ref()) else {
                continue;
            };

            files.push((path, timestamp));
        }

        // `>=` can make sure (after rotate), files won't exceed the limit
        if files.len() >= self.max_files {
            files.sort_by(|a, b| a.0.cmp(&b.0));

            // we might have to remove a lot of files
            while files.len() >= self.max_files {
                let Some((oldest, _ts)) = files.pop() else {
                    continue;
                };

                match std::fs::remove_file(&oldest) {
                    Ok(()) => {
                        debug!(message = "remove oldest rotated file", path = ?oldest);
                    }
                    Err(err) => {
                        error!(
                            message = "remove oldest rotated file failed",
                            path = ?oldest,
                            ?err
                        );
                    }
                }
            }
        }

        let stem = path.file_stem().expect("good file name");

        let now = Utc::now();
        let mut name = format!(
            "{}-{}",
            stem.to_string_lossy(),
            now.format(TIMESTAMP_FORMAT)
        );

        if let Some(ext) = path.extension() {
            name.push('.');
            name.push_str(ext.to_string_lossy().as_ref());
        }

        let new = path.with_file_name(name);
        tokio::fs::rename(path, new).await?;

        open_file(path).await
    }
}

#[configurable_component(sink, name = "file")]
struct Config {
    /// Path of the file to write to. Path could be a relative one to current directory,
    /// but absolute path is recommended.
    path: PathBuf,

    #[serde(default)]
    rotation: Option<RotationConfig>,

    #[serde(flatten)]
    encoding: EncodingConfigWithFraming,

    #[serde(default)]
    acknowledgements: bool,
}

#[async_trait::async_trait]
#[typetag::serde(name = "file")]
impl SinkConfig for Config {
    async fn build(&self, _cx: SinkContext) -> crate::Result<(Sink, Healthcheck)> {
        let transformer = self.encoding.transformer();
        let (framer, serializer) = self.encoding.build(SinkType::StreamBased);
        let encoder = Encoder::<Framer>::new(framer, serializer);

        let sink = Sink::Stream(Box::new(FileSink {
            path: self.path.clone(),
            transformer,
            encoder,
            rotation: self.rotation.clone(),
        }));

        Ok((sink, healthcheck(self.path.clone()).boxed()))
    }

    fn input_type(&self) -> InputType {
        InputType::new(DataType::Log | DataType::Metric)
    }
}

// create a temp file make sure we can write data to the directory
async fn healthcheck(path: PathBuf) -> crate::Result<()> {
    let path = path.with_file_name(".write_test");
    let parent = path.parent().ok_or(format!("{:?} has no parent", path))?;

    tokio::fs::create_dir_all(parent).await?;

    File::create(&path).await?.write_all(b"test").await?;

    // create ok, and write ok, then remove the test file
    tokio::fs::remove_file(path).await.map_err(Into::into)
}

const TIMESTAMP_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.3f";

struct FileSink {
    path: PathBuf,
    transformer: Transformer,
    encoder: Encoder<Framer>,

    rotation: Option<RotationConfig>,
}

async fn open_file(path: &Path) -> std::io::Result<File> {
    File::options()
        .write(true)
        .append(true)
        .create(true)
        .open(path)
        .await
}

#[async_trait::async_trait]
impl StreamSink for FileSink {
    async fn run(mut self: Box<Self>, mut input: BoxStream<'_, Events>) -> Result<(), ()> {
        let mut file = match open_file(&self.path).await {
            Ok(file) => file,
            Err(err) => {
                error!(
                    message = "open output file failed",
                    %err,
                    file = ?self.path,
                );
                return Err(());
            }
        };

        let mut interval = tokio::time::interval(Duration::from_secs(5));

        let mut buf = BytesMut::with_capacity(4 * 1024);
        let mut size = file.seek(SeekFrom::End(0)).await.expect("seek success") as usize;
        let mut written = 0;
        loop {
            // if the file is big enough, then rotate
            if let Some(rotation) = &self.rotation
                && size > rotation.max_size
            {
                match rotation.rotate(&self.path).await {
                    Ok(new) => {
                        debug!(message = "rotate success");

                        size = 0;
                        written = 0;
                        file = new;
                    }
                    Err(err) => {
                        warn!(
                            message = "rotate file failed, using old one",
                            ?err,
                            internal_log_rate_limit = true,
                        );
                    }
                }
            }

            tokio::select! {
                result = input.next() => match result {
                    Some(events) => {
                        for mut event in events.into_events() {
                            let finalizers = event.take_finalizers();

                            self.transformer.transform(&mut event);

                            buf.clear();
                            if let Err(err) = self.encoder.encode(event, &mut buf) {
                                finalizers.update_status(EventStatus::Errored);
                                warn!(
                                    message = "encode event failed",
                                    ?err
                                );

                                continue;
                            }

                            finalizers.update_status(match file.write_all(&buf).await {
                                Ok(()) => {
                                    size += buf.len();
                                    written += buf.len();
                                    EventStatus::Delivered
                                },
                                Err(err) => {
                                    warn!(
                                        message = "write event to file failed",
                                        path = ?self.path,
                                        %err,
                                        internal_log_rate_limit = true,
                                    );
                                    EventStatus::Errored
                                }
                            });
                        }
                    },
                    None => break,
                },
                _ = interval.tick(), if written > 0 => {
                    if let Err(err) = file.sync_all().await {
                        error!(message = "Error syncing file.", %err, path = ?self.path);
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codecs::encoding::{FramingConfig, SerializerConfig};
    use event::LogRecord;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }

    #[tokio::test]
    async fn cleanup() {
        let root = testify::temp_dir();
        for filename in [
            "metrics-2026-03-10T11:02:50.528.txt",
            "metrics-2026-03-10T11:03:30.546.txt",
            "metrics-2026-03-10T11:04:15.523.txt",
            "metrics-2026-03-10T11:04:55.535.txt",
            "metrics-2026-03-10T11:05:35.550.txt",
            "metrics.txt",
        ] {
            std::fs::File::create(root.join(filename)).unwrap();
        }

        let sink = FileSink {
            path: root.join("metrics.txt"),
            transformer: Default::default(),
            encoder: Default::default(),
            rotation: Some(RotationConfig {
                max_size: 0,
                max_files: 2,
            }),
        };

        sink.rotation.unwrap().rotate(&sink.path).await.unwrap();

        // 2 backups and 1 current
        assert_eq!(3, root.read_dir().unwrap().count());
    }

    #[tokio::test]
    async fn rotate_once() {
        let root = testify::temp_dir();
        let config = Config {
            path: root.join("logs.txt"),
            rotation: Some(RotationConfig {
                max_size: 10,
                max_files: 2,
            }),
            encoding: (None::<FramingConfig>, SerializerConfig::Text).into(),
            acknowledgements: false,
        };

        let (sink, healthcheck) = config.build(SinkContext::new_test()).await.unwrap();
        healthcheck.await.unwrap();

        run_and_assert(sink, &["abcdefgh", "12345678"]).await;

        root.read_dir()
            .unwrap()
            .flatten()
            .for_each(|entry| println!("{:?}", entry));

        // one backup and one current
        assert_eq!(root.read_dir().unwrap().count(), 2);
    }

    async fn run_and_assert(sink: Sink, msgs: &[&str]) {
        let input =
            futures::stream::iter(msgs.iter().map(|msg| LogRecord::from(*msg)).map(Into::into));
        sink.run(input).await.unwrap();
    }
}
