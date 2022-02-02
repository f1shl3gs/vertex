use crate::encoding_transcode::{Decoder, Encoder};
use bytes::Bytes;
use chrono::Utc;
use event::{fields, tags, BatchNotifier, Event, LogRecord};
use futures::Stream;
use futures_util::{FutureExt, StreamExt, TryFutureExt};
use humanize::{deserialize_bytes, serialize_bytes};
use log_schema::log_schema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tail::{Checkpointer, Fingerprint, Harvester, Line, ReadFrom};

use crate::config::{
    deserialize_duration, serialize_duration, DataType, GenerateConfig, Output, SourceConfig,
    SourceContext, SourceDescription,
};
use crate::hostname;
use crate::multiline::{LineAgg, Logic, MultilineConfig, Parser};
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
#[serde(deny_unknown_fields)]
struct TailConfig {
    #[serde(default = "default_ignore_older_than")]
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
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
    #[serde(default = "default_line_delimiter")]
    line_delimiter: String,

    #[serde(default = "default_glob_interval")]
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
    glob_interval: Duration,
    #[serde(default)]
    acknowledgement: bool,
    charset: Option<&'static encoding_rs::Encoding>,
    multiline: Option<MultilineConfig>,
}

const fn default_ignore_older_than() -> Duration {
    Duration::from_secs(12 * 60 * 60)
}

const fn default_glob_interval() -> Duration {
    Duration::from_secs(3)
}

const fn default_max_read_bytes() -> usize {
    2 * 1024
}

const fn default_max_line_bytes() -> usize {
    100 * 1024 // 100kb
}

fn default_line_delimiter() -> String {
    "\n".into()
}

impl GenerateConfig for TailConfig {
    fn generate_config() -> String {
        format!(
            r#"
# Array of file patterns to include. Globbing is support.
#
include:
- /path/to/some-*.log

# Array of file patterns to exclude. Globbing is supported.
# Takes precedence over the "include" option
#
# exlucde:
# - /path/to/some-exclude.log

# In the absence of a checkpoint, this setting tells Vertex where to
# start reading files that are present at startup.
#
# Availabel options:
# - beginning:  Read from the beginning of the file.
# - end:        Start reading from the current end of the file.
#
# read_from: beginning

# Ignore files with a data modification date older than the specified
# duration.
#
# ignore_older: 1h

# The maximum number of a bytes a line can contain before being discarded.
# This protects against malformed lines or tailing incorrect files.
#
# max_line_bytes: {}

# An approximate limit on the amount of data read from a single file at
# a given time.
#
# max_read_bytes: {}

# Delay between file discovery calls. This controls the interval at which
# Vertex searches for files. Higher value result in greater chances of some
# short living files being missed between searches, but lower value increases
# performance impact of file discovery.
#
# glob_interval: {}

# The key name added to each event representing the current host. This can
# be globally set via the global "host_key" option.
#
# host_key: host

# Encoding of the source messages. Takes one of the encoding "label strings"
# defined as part of the "Encoding Standard"
# https://encoding.spec.whatwg.org/#concept-encoding-get
#
# When set, the messages are transcoded from the specified encoding to UTF-8,
# which is the encoding vertex assumes internally for string-like data.
# Enable this transcoding operation if you need your data to be in UTF-8 for
# further processing. At the time of transcoding, any malformed sequences(that's
# can't be mapped to UTF-8) will be replaced with "replacement character (see:
# https://en.wikipedia.org/wiki/Specials_(Unicode_block)#Replacement_character)
# and warnings will be logged.
#
# charset: utf-16be

# Controls how acknowledgements are handled by this source
#
# acknowledgements: false

#
            "#,
            humanize::bytes(default_max_line_bytes()),
            humanize::bytes(default_max_read_bytes()),
            humanize::duration_to_string(&default_glob_interval()),
        )
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
        let data_dir = ctx.globals.make_subdir(&ctx.key.to_string())?;
        let glob = tail::provider::Glob::new(&self.include, &self.exclude)
            .ok_or("glob provider create failed")?;

        let read_from = match &self.read_from {
            Some(read_from) => match read_from {
                ReadFromConfig::Beginning => ReadFrom::Beginning,
                ReadFromConfig::End => ReadFrom::End,
            },
            None => ReadFrom::default(),
        };

        let mut output = ctx.output;
        let include = self.include.clone();
        let exclude = self.exclude.clone();
        let multiline = self.multiline.clone();
        let shutdown = ctx.shutdown.shared();
        let checkpointer = Checkpointer::new(&data_dir);
        let checkpoints = checkpointer.view();
        let host_key = self
            .host_key
            .clone()
            .unwrap_or_else(|| log_schema().host_key().to_string());
        let hostname = hostname().unwrap();
        let timestamp_key = log_schema().timestamp_key();
        let source_type_key = log_schema().source_type_key();
        let finalizer = self.acknowledgement.then(|| {
            let checkpoints = checkpointer.view();
            OrderedFinalizer::new(shutdown.clone(), move |entry: FinalizerEntry| {
                checkpoints.update(entry.fingerprint, entry.offset)
            })
        });

        let charset = self.charset;
        let line_delimiter = match charset {
            Some(e) => Encoder::new(e).encode_from_utf8(&self.line_delimiter),
            None => Bytes::from(self.line_delimiter.clone()),
        };

        let harvester = Harvester {
            provider: glob,
            read_from,
            max_read_bytes: self.max_read_bytes,
            handle: tokio::runtime::Handle::current(),
            ignore_before: None,
            max_line_bytes: self.max_line_bytes,
            line_delimiter,
        };

        Ok(Box::pin(async move {
            info!(
                message = "Starting harvest files",
                include = ?include,
                exclude = ?exclude,
            );

            let mut encoding_decoder = charset.map(Decoder::new);

            // sizing here is just a guess
            let (tx, rx) = futures::channel::mpsc::channel::<Vec<Line>>(16);
            let rx = rx
                .map(futures::stream::iter)
                .flatten()
                .map(move |mut line| {
                    // transcode each line from the file's encoding charset to utf8
                    line.text = match encoding_decoder.as_mut() {
                        Some(decoder) => decoder.decode_to_utf8(line.text),
                        None => line.text,
                    };

                    line
                });

            let messages: Box<dyn Stream<Item = Line> + Send + std::marker::Unpin> =
                if let Some(ref conf) = multiline {
                    // This match looks ugly, but it does not need `dyn`
                    match conf.parser {
                        Parser::Cri => {
                            let logic = Logic::new(crate::multiline::preset::Cri, conf.timeout);
                            Box::new(
                                LineAgg::new(
                                    rx.map(|line| {
                                        (line.filename, line.text, (line.fingerprint, line.offset))
                                    }),
                                    logic,
                                )
                                .map(
                                    |(filename, text, (fingerprint, offset))| Line {
                                        text,
                                        filename,
                                        fingerprint,
                                        offset,
                                    },
                                ),
                            )
                        }
                        Parser::NoIndent => {
                            let logic =
                                Logic::new(crate::multiline::preset::NoIndent, conf.timeout);
                            Box::new(
                                LineAgg::new(
                                    rx.map(|line| {
                                        (line.filename, line.text, (line.fingerprint, line.offset))
                                    }),
                                    logic,
                                )
                                .map(
                                    |(filename, text, (fingerprint, offset))| Line {
                                        text,
                                        filename,
                                        fingerprint,
                                        offset,
                                    },
                                ),
                            )
                        }
                        _ => unreachable!(),
                    }
                } else {
                    Box::new(rx)
                };

            // Once harvester ends this will run until it has finished processing remaining
            // logs in the queue
            let mut messages = messages.map(move |line| {
                let mut event: Event = LogRecord::new(
                    tags!(
                        "filename" => line.filename,
                        &host_key => &hostname,
                        source_type_key => "file"
                    ),
                    fields!(
                        "message" => line.text,
                        "offset" => line.offset,
                        timestamp_key =>  Utc::now()
                    ),
                )
                .into();

                if let Some(finalizer) = &finalizer {
                    let (batch, receiver) = BatchNotifier::new_with_receiver();
                    event = event.with_batch_notifier(&batch);

                    finalizer.add(
                        FinalizerEntry {
                            fingerprint: line.fingerprint,
                            offset: line.offset,
                        },
                        receiver,
                    );
                } else {
                    checkpoints.update(line.fingerprint, line.offset);
                }

                event
            });

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

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }

    fn source_type(&self) -> &'static str {
        "tail"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::config::test_generate_config::<TailConfig>()
    }
}
