use bytes::Bytes;
use event::{fields, tags, Event, LogRecord};
use futures_util::{FutureExt, SinkExt, StreamExt, TryFutureExt};
use humanize::{deserialize_bytes, serialize_bytes};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tail::{Checkpointer, Fingerprint, Harvester, Line, ReadFrom};

use crate::config::{
    deserialize_std_duration, serialize_std_duration, DataType, GenerateConfig, SourceConfig,
    SourceContext, SourceDescription,
};
use crate::sources::utils::OrderedFinalizer;
use crate::sources::Source;

#[derive(Debug, Deserialize, Serialize)]
pub enum ReadFromConfig {
    Beginning,
    End,
}

impl From<ReadFromConfig> for ReadFrom {
    fn from(c: ReadFromConfig) -> Self {
        match c {
            ReadFromConfig::Beginning => ReadFrom::Beginning,
            ReadFromConfig::End => ReadFrom::End,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct TailConfig {
    #[serde(default = "default_ignore_older_than")]
    #[serde(
        deserialize_with = "deserialize_std_duration",
        serialize_with = "serialize_std_duration"
    )]
    ignore_older_than: Duration,

    host_key: Option<String>,

    include: Vec<PathBuf>,
    #[serde(default)]
    exclude: Vec<PathBuf>,

    read_from: Option<ReadFromConfig>,
    #[serde(
        default = "default_max_line_bytes",
        deserialize_with = "deserialize_bytes",
        serialize_with = "serialize_bytes"
    )]
    max_line_bytes: usize,
    #[serde(
        default = "default_max_read_bytes",
        deserialize_with = "deserialize_bytes",
        serialize_with = "serialize_bytes"
    )]
    max_read_bytes: usize,
    line_delimiter: String,

    #[serde(default = "default_glob_interval")]
    #[serde(
        deserialize_with = "deserialize_std_duration",
        serialize_with = "serialize_std_duration"
    )]
    glob_interval: Duration,
}

fn default_ignore_older_than() -> Duration {
    Duration::from_secs(12 * 60 * 60)
}

fn default_glob_interval() -> Duration {
    Duration::from_secs(3)
}

fn default_max_read_bytes() -> usize {
    16 * 1024
}

fn default_max_line_bytes() -> usize {
    100 * 1024 // 100kb
}

impl GenerateConfig for TailConfig {
    fn generate_config() -> serde_yaml::Value {
        serde_yaml::to_value(Self {
            ignore_older_than: default_ignore_older_than(),
            host_key: None,
            include: vec!["/path/to/include/*.log".into()],
            exclude: vec!["/path/to/exclude/noop.log".into()],
            read_from: Some(ReadFromConfig::End),
            max_line_bytes: default_max_line_bytes(),
            max_read_bytes: default_max_read_bytes(),
            line_delimiter: "\n".to_string(),
            glob_interval: default_glob_interval(),
        })
        .unwrap()
    }
}

inventory::submit! {
    SourceDescription::new::<TailConfig>("tail")
}

#[derive(Debug)]
pub(crate) struct FinalizerEntry {
    pub(crate) fingerprint: Fingerprint,
    pub(crate) offset: u64,
}

#[async_trait::async_trait]
#[typetag::serde(name = "tail")]
impl SourceConfig for TailConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        // add the source name as a subdir, so that multiple sources can operate
        // within the same given data_dir(e.g. the global one) without the file
        // servers' checkpointers interfering with each other
        let data_dir = ctx.global.make_subdir(&ctx.name)?;
        let glob = tail::provider::Glob::new(&self.include, &self.exclude)
            .ok_or("glob provider create failed")?;

        let read_from = if self.read_from.is_some() {
            self.read_from.map(Into::into).unwrap_or_default()
        } else {
            ReadFrom::default()
        };

        let mut output = ctx.out;
        let include = self.include.clone();
        let exclude = self.exclude.clone();
        let shutdown = ctx.shutdown.shared();
        let checkpointer = Checkpointer::new(&data_dir);
        let checkpoints = checkpointer.view();
        // let finalizer = Some({
        //     let checkpoints = checkpointer.view();
        //     OrderedFinalizer::new(shutdown.clone(), move |entry: FinalizerEntry| {
        //         checkpoints.update(entry.fingerprint, entry.offset)
        //     })
        // });

        let harvester = Harvester {
            provider: glob,
            read_from,
            max_read_bytes: self.max_read_bytes,
            handle: tokio::runtime::Handle::current(),
            ignore_before: None,
            max_line_bytes: self.max_line_bytes,
            line_delimiter: Bytes::from(self.line_delimiter.clone()),
        };

        Ok(Box::pin(async move {
            info!(
                message = "Starting harvest files",
                include = ?include,
                exclude = ?exclude,
            );

            // sizing here is just a guess
            let (tx, rx) = futures::channel::mpsc::channel::<Vec<Line>>(16);
            let rx = rx
                .map(futures::stream::iter)
                .flatten()
                .map(move |mut line| line); // TODO: transcode each line from the file's encoding charset to utf8

            let message = Box::new(rx);

            // Once harvester ends this will run until it has finished processing remaining
            // logs in the queue
            let mut messages = message
                .map(move |line| {
                    let event: Event = LogRecord::new(
                        tags!(
                            "filename" => line.filename,
                        ),
                        fields!(
                            "message" => line.text,
                            "offset" => line.offset
                        ),
                    )
                    .into();

                    /*if let Some(finalizer) = &finalizer {
                        let (batch, receiver) = BatchNotifier::new_with_reciever();
                        event = event.with_batch_notifier(&batch);

                        finalizer.add( FinalizerEntry {
                            fingerprint: line.fingerprint,
                            offset: line.offset,
                        }, receiver);
                    } else {

                    }*/

                    checkpoints.update(line.fingerprint, line.offset);

                    event
                })
                .map(Ok);

            tokio::spawn(async move { output.send_all(&mut messages).await });

            tokio::task::spawn_blocking(move || {
                let result = harvester.run(tx, shutdown, checkpointer);
                // Panic if we encounter any error originating from the harvester.
                // We're at the `spawn_blocking` call, the panic will be caught and
                // passed to the `JoinHandle` error, similar to the usual threads.
                result.unwrap();
            })
            .map_err(|err| {
                error!(
                    message = "Harvester unexpectedly stopped",
                    %err
                );
            })
            .await
        }))
    }

    fn output_type(&self) -> DataType {
        DataType::Log
    }

    fn source_type(&self) -> &'static str {
        "tail"
    }
}
