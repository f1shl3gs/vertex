use std::path::PathBuf;
use std::time::Duration;

use bytes::Bytes;
use chrono::Utc;
use event::{fields, tags, BatchNotifier, Event, LogRecord};
use framework::config::{
    deserialize_duration, serialize_duration, DataType, GenerateConfig, Output, SourceConfig,
    SourceContext, SourceDescription,
};
use framework::source::util::OrderedFinalizer;
use framework::{hostname, Pipeline, ShutdownSignal, Source};
use futures::Stream;
use futures_util::{FutureExt, StreamExt, TryFutureExt};
use humanize::{deserialize_bytes, serialize_bytes};
use log_schema::log_schema;
use multiline::{LineAgg, Logic, MultilineConfig, Parser};
use serde::{Deserialize, Serialize};
use tail::{Checkpointer, Fingerprint, Harvester, Line, ReadFrom};

use crate::encoding_transcode::{Decoder, Encoder};

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
            humanize::duration(&default_glob_interval()),
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
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        // add the source name as a subdir, so that multiple sources can operate
        // within the same given data_dir(e.g. the global one) without the file
        // servers' checkpointers interfering with each other
        let data_dir = cx.globals.make_subdir(cx.key.id())?;
        let acknowledgements = cx.acknowledgements();

        tail_source(self, data_dir, cx.shutdown, cx.output, acknowledgements)
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }

    fn source_type(&self) -> &'static str {
        "tail"
    }
}

fn tail_source(
    config: &TailConfig,
    data_dir: PathBuf,
    shutdown: ShutdownSignal,
    mut output: Pipeline,
    acknowledgements: bool,
) -> crate::Result<Source> {
    let provider = tail::provider::Glob::new(&config.include, &config.exclude)
        .ok_or("glob provider create failed")?;

    let read_from = match &config.read_from {
        Some(read_from) => match read_from {
            ReadFromConfig::Beginning => ReadFrom::Beginning,
            ReadFromConfig::End => ReadFrom::End,
        },
        None => ReadFrom::default(),
    };

    let include = config.include.clone();
    let exclude = config.exclude.clone();
    let shutdown = shutdown.shared();
    let multiline = config.multiline.clone();
    let checkpointer = Checkpointer::new(&data_dir);
    let checkpoints = checkpointer.view();
    let host_key = config
        .host_key
        .clone()
        .unwrap_or_else(|| log_schema().host_key().to_string());
    let hostname = hostname().unwrap();
    let timestamp_key = log_schema().timestamp_key();
    let source_type_key = log_schema().source_type_key();
    let finalizer = acknowledgements.then(|| {
        let checkpoints = checkpointer.view();
        OrderedFinalizer::new(shutdown.clone(), move |entry: FinalizerEntry| {
            checkpoints.update(entry.fingerprint, entry.offset)
        })
    });

    let charset = config.charset;
    let line_delimiter = match charset {
        Some(e) => Encoder::new(e).encode_from_utf8(&config.line_delimiter),
        None => Bytes::from(config.line_delimiter.clone()),
    };

    let harvester = Harvester {
        provider,
        read_from,
        max_read_bytes: config.max_read_bytes,
        handle: tokio::runtime::Handle::current(),
        ignore_before: None,
        max_line_bytes: config.max_line_bytes,
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
                        let logic = Logic::new(multiline::preset::Cri, conf.timeout);
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
                        let logic = Logic::new(multiline::preset::NoIndent, conf.timeout);
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
                    source_type_key => "tail"
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

        tokio::spawn(async move { output.send_all_v2(&mut messages).await });

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

#[cfg(test)]
mod tests {
    use super::*;
    use event::attributes::Key;
    use event::EventStatus;
    use framework::{Pipeline, ShutdownSignal};
    use std::fs;
    use std::fs::File;
    use std::future::Future;
    use std::io::Write;
    use tempfile::tempdir;
    use tokio::time::timeout;

    fn test_default_tail_config(dir: &tempfile::TempDir) -> TailConfig {
        TailConfig {
            ignore_older_than: default_ignore_older_than(),
            host_key: None,
            include: vec![dir.path().join("*")],
            exclude: vec![],
            read_from: None,
            max_line_bytes: default_max_line_bytes(),
            max_read_bytes: default_max_read_bytes(),
            line_delimiter: default_line_delimiter(),
            glob_interval: default_glob_interval(),
            charset: None,
            multiline: None,
        }
    }

    #[derive(Debug, Eq, PartialEq)]
    enum AckingMode {
        No,
        Unfinalized,
        Acks,
    }

    async fn wait_with_timeout<F, R>(fut: F) -> R
    where
        F: Future<Output = R> + Send,
        R: Send,
    {
        timeout(Duration::from_secs(5), fut)
            .await
            .unwrap_or_else(|_| {
                panic!("Unclosed channel: may indicate harvester could not shutdown gracefully")
            })
    }

    async fn run_tail(
        config: &TailConfig,
        data_dir: PathBuf,
        wait_shutdown: bool,
        acking_mode: AckingMode,
        inner: impl Future<Output = ()>,
    ) -> Vec<Event> {
        let (tx, rx) = if acking_mode == AckingMode::Acks {
            let (tx, rx) = Pipeline::new_test_finalize(EventStatus::Delivered);
            (tx, rx.boxed())
        } else {
            let (tx, rx) = Pipeline::new_test();
            (tx, rx.boxed())
        };

        let (trigger_shutdown, shutdown, shutdown_done) = ShutdownSignal::new_wired();
        let acks = !matches!(acking_mode, AckingMode::No);

        tokio::spawn(tail_source(config, data_dir, shutdown, tx, acks).unwrap());

        inner.await;

        drop(trigger_shutdown);

        let result = wait_with_timeout(rx.collect::<Vec<_>>()).await;
        if wait_shutdown {
            shutdown_done.await;
        }

        result
    }

    async fn sleep_500_millis() {
        tokio::time::sleep(Duration::from_millis(500)).await
    }

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<TailConfig>()
    }

    #[tokio::test]
    async fn happy_path() {
        let n = 5;
        let dir = tempdir().unwrap();
        let config = TailConfig {
            include: vec![dir.path().join("*")],
            ..test_default_tail_config(&dir)
        };

        let path1 = dir.path().join("file1");
        let path2 = dir.path().join("file2");

        let received = run_tail(
            &config,
            dir.path().to_path_buf(),
            false,
            AckingMode::No,
            async {
                let mut file1 = File::create(&path1).unwrap();
                let mut file2 = File::create(&path2).unwrap();

                // The files must be observed at their original lengths before writing to them
                sleep_500_millis().await;

                for i in 0..n {
                    writeln!(file1, "foo {}", i).unwrap();
                    writeln!(file2, "bar {}", i).unwrap();
                }

                sleep_500_millis().await;
            },
        )
        .await;

        let mut foo = 0;
        let mut bar = 0;

        for event in received {
            let log = event.as_log();
            let line = log
                .get_field(log_schema().message_key())
                .unwrap()
                .to_string_lossy();

            if line.starts_with("foo") {
                assert_eq!(line, format!("foo {}", foo));
                assert_eq!(
                    log.tags.get(&Key::from("filename")).unwrap().to_string(),
                    path1.to_str().unwrap()
                );
                foo += 1;
            } else {
                assert_eq!(line, format!("bar {}", bar));
                assert_eq!(
                    log.tags.get(&Key::from("filename")).unwrap().to_string(),
                    path2.to_str().unwrap()
                );
                bar += 1;
            }
        }

        assert_eq!(foo, n);
        assert_eq!(bar, n);
    }

    #[tokio::test]
    async fn file_read_empty_lines() {
        let n = 5;

        let dir = tempdir().unwrap();
        let config = TailConfig {
            include: vec![dir.path().join("*")],
            ..test_default_tail_config(&dir)
        };

        let path = dir.path().join("file");

        let received = run_tail(
            &config,
            dir.path().to_path_buf(),
            false,
            AckingMode::No,
            async {
                let mut file = File::create(&path).unwrap();

                // The files must be observed at their original
                // lengths before writing to them
                sleep_500_millis().await;

                writeln!(&mut file, "line for checkpointing").unwrap();
                for _i in 0..n {
                    writeln!(&mut file).unwrap();
                }

                sleep_500_millis().await;
            },
        )
        .await;

        assert_eq!(received.len(), n + 1);
    }

    // TODO: support truncate ?

    #[tokio::test]
    async fn file_rotate() {
        let n = 5;

        let dir = tempdir().unwrap();
        let config = TailConfig {
            include: vec![dir.path().join("*")],
            ..test_default_tail_config(&dir)
        };

        let path = dir.path().join("file");
        let archive_path = dir.path().join("file");
        let received = run_tail(
            &config,
            dir.path().to_path_buf(),
            false,
            AckingMode::No,
            async {
                let mut file = File::create(&path).unwrap();

                // The files must be observed at its original
                // length before writing to it
                sleep_500_millis().await;

                for i in 0..n {
                    writeln!(&mut file, "prerot {}", i).unwrap();
                }

                // The writes must be observed before rotating
                sleep_500_millis().await;

                fs::rename(&path, archive_path).expect("could not rename");
                let mut file = File::create(&path).unwrap();

                // The rotation must be observed before writing again
                sleep_500_millis().await;

                for i in 0..n {
                    writeln!(&mut file, "postrot {}", i).unwrap();
                }

                sleep_500_millis().await
            },
        )
        .await;

        let mut i = 0;
        let mut pre_rot = true;

        for event in received {
            assert_eq!(
                event.as_log().get_field("file").unwrap().to_string_lossy(),
                path.to_str().unwrap()
            );

            let line = event
                .as_log()
                .get_field(log_schema().message_key())
                .unwrap()
                .to_string_lossy();
            if pre_rot {
                assert_eq!(line, format!("prerot {}", i));
            } else {
                assert_eq!(line, format!("postrot {}", i));
            }

            i += 1;
            if i == n {
                i = 0;
                pre_rot = false;
            }
        }
    }

    #[tokio::test]
    async fn multiple_paths() {
        let n = 5;

        let dir = tempdir().unwrap();
        let config = TailConfig {
            include: vec![dir.path().join("*.txt"), dir.path().join("a.*")],
            exclude: vec![dir.path().join("a.*.txt")],
            ..test_default_tail_config(&dir)
        };

        let path1 = dir.path().join("a.txt");
        let path2 = dir.path().join("b.txt");
        let path3 = dir.path().join("a.log");
        let path4 = dir.path().join("a.ignore.txt");
        let received = run_tail(
            &config,
            dir.path().to_path_buf(),
            false,
            AckingMode::No,
            async {
                let mut file1 = File::create(&path1).unwrap();
                let mut file2 = File::create(&path2).unwrap();
                let mut file3 = File::create(&path3).unwrap();
                let mut file4 = File::create(&path4).unwrap();

                // The files must be observed at their original
                // lengths before writing to them
                sleep_500_millis().await;

                for i in 0..n {
                    writeln!(&mut file1, "1 {}", i).unwrap();
                    writeln!(&mut file2, "2 {}", i).unwrap();
                    writeln!(&mut file3, "3 {}", i).unwrap();
                    writeln!(&mut file4, "4 {}", i).unwrap();
                }

                sleep_500_millis().await;
            },
        )
        .await;

        let mut is = [0; 3];
        for event in received {
            let line = event
                .as_log()
                .get_field(log_schema().message_key())
                .unwrap()
                .to_string_lossy();
            let mut split = line.split(' ');
            let file = split.next().unwrap().parse::<usize>().unwrap();
            assert_ne!(file, 4);
            let i = split.next().unwrap().parse::<usize>().unwrap();

            assert_eq!(is[file - 1], i);
            is[file - 1] += 1;
        }

        assert_eq!(is, [n as usize; 3]);
    }
}
