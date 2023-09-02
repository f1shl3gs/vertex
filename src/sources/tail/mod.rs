mod encoding_transcode;

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytes::Bytes;
use chrono::Utc;
use configurable::{configurable_component, Configurable};
use encoding_transcode::{Decoder, Encoder};
use event::{fields, tags, BatchNotifier, BatchStatus, Event, LogRecord};
use framework::config::{DataType, Output, SourceConfig, SourceContext};
use framework::source::util::OrderedFinalizer;
use framework::{hostname, Pipeline, ShutdownSignal, Source};
use futures::Stream;
use futures_util::{FutureExt, StreamExt, TryFutureExt};
use log_schema::log_schema;
use multiline::{LineAgg, Logic, MultilineConfig, Parser};
use serde::{Deserialize, Serialize};
use tail::{Checkpointer, Fingerprint, Harvester, Line, ReadFrom};
use tokio::sync::oneshot;

const POISONED_FAILED_LOCK: &str = "Poisoned lock on failed files set";

/// File position to use when reading a new file.
#[derive(Configurable, Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ReadFromConfig {
    /// Read from the beginning of the file.
    #[default]
    Beginning,

    /// Start reading from the current end of the file.
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

fn default_file_key() -> String {
    "file".into()
}

#[configurable_component(source, name = "tail")]
#[serde(deny_unknown_fields)]
struct TailConfig {
    /// Ignore files with a data modification date older than the specified duration.
    #[serde(default = "default_ignore_older_than")]
    #[serde(with = "humanize::duration::serde_option")]
    ignore_older_than: Option<Duration>,

    /// Overrides the name of the log field used to add the current hostname to each event.
    ///
    /// By default, the [global `log_schema.host_key` option][global_host_key] is used.
    host_key: Option<String>,

    /// Overrides the name of the log field used to add the current hostname to each event.
    ///
    /// By default, the [global `log_schema.host_key` option][global_host_key] is used.
    #[serde(default = "default_file_key")]
    file_key: String,

    /// Array of file patterns to include. glob is supported.
    include: Vec<PathBuf>,

    /// Array of file patterns to exclude. glob is supported.
    ///
    /// Takes precedence over the `include` option. Note: The `exclude` patterns are applied
    /// _after_ the attempt to glob everything in `include`. This means that all files are
    /// first matched by `include` and then filtered by the `exclude` patterns. This can be
    /// impactful if `include` contains directories with contents that are not accessible.
    #[serde(default)]
    exclude: Vec<PathBuf>,

    #[serde(default)]
    read_from: ReadFromConfig,

    /// The maximum size of a line before it will be discarded.
    ///
    /// This protects against malformed lines or tailing incorrect files.
    #[serde(default = "default_max_line_bytes", with = "humanize::bytes::serde")]
    max_line_bytes: usize,

    /// An approximate limit on the amount of data read from a single file at a given time.
    #[serde(default = "default_max_read_bytes", with = "humanize::bytes::serde")]
    max_read_bytes: usize,

    /// String sequence used to separate one file line from another.
    #[serde(default = "default_line_delimiter")]
    line_delimiter: String,

    /// Delay between file discovery calls. This controls the interval at which
    /// Vertex searches for files. Higher value result in greater chances of some
    /// short living files being missed between searches, but lower value increases
    /// performance impact of file discovery.
    #[serde(default = "default_glob_interval", with = "humanize::duration::serde")]
    glob_interval: Duration,

    /// Encoding of the source messages. Takes one of the encoding "label strings"
    /// defined as part of the "Encoding Standard"
    /// https://encoding.spec.whatwg.org/#concept-encoding-get
    ///
    /// When set, the messages are transcoded from the specified encoding to UTF-8,
    /// which is the encoding vertex assumes internally for string-like data.
    /// Enable this transcoding operation if you need your data to be in UTF-8 for
    /// further processing. At the time of transcoding, any malformed sequences(that's
    /// can't be mapped to UTF-8) will be replaced with "replacement character (see:
    /// https://en.wikipedia.org/wiki/Specials_(Unicode_block)#Replacement_character)
    /// and warnings will be logged.
    charset: Option<&'static encoding_rs::Encoding>,

    /// Multiline aggregation configuration.
    ///
    /// If not specified, multiline aggregation is disabled.
    #[configurable(skip)]
    multiline: Option<MultilineConfig>,
}

const fn default_ignore_older_than() -> Option<Duration> {
    Some(Duration::from_secs(12 * 60 * 60))
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

    fn acknowledgable(&self) -> bool {
        true
    }
}

fn tail_source(
    config: &TailConfig,
    data_dir: PathBuf,
    shutdown: ShutdownSignal,
    mut output: Pipeline,
    acknowledgements: bool,
) -> crate::Result<Source> {
    let file_key = config.file_key.to_owned();
    let provider = tail::provider::Glob::new(&config.include, &config.exclude)
        .ok_or("glob provider create failed")?;

    let read_from = match &config.read_from {
        ReadFromConfig::Beginning => ReadFrom::Beginning,
        ReadFromConfig::End => ReadFrom::End,
    };

    let ignore_before = config
        .ignore_older_than
        .map(|d| Utc::now() - chrono::Duration::from_std(d).unwrap());
    let include = config.include.clone();
    let exclude = config.exclude.clone();
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

    // The `failed_files` set contains `Fingerprint`s, provided by
    // the file server, of all files that have received a negative
    // acknowledgements. This set is shared between the finalizer task,
    // which both holds back checkpointer updates if an identifier is
    // present and adds entries on negative acknowledgements, and the
    // main file server handling task, which holds back further events
    // from files in the set.
    let failed_files: Arc<Mutex<HashSet<Fingerprint>>> = Default::default();
    let (finalizer, shutdown_checkpointer) = if acknowledgements {
        // The shutdown sent in to the finalizer is the global
        // shutdown handle used to tell it to stop accepting new batch
        // statuses and just wait for the remaining acks to come in.
        let (finalizer, mut ack_stream) = OrderedFinalizer::<FinalizerEntry>::new(shutdown.clone());

        // We set up a separate shutdown signal to tie together the
        // finalizer and the checkpoint writer task in the harvester,
        // to make it continue to write out updated checkpoints until
        // all the acks have come in.
        let (send_shutdown, shutdown2) = oneshot::channel::<()>();
        let checkpoints = checkpointer.view();
        let failed_files = Arc::clone(&failed_files);
        tokio::spawn(async move {
            while let Some((status, entry)) = ack_stream.next().await {
                // Don't update the checkpointer on file streams after failed acks
                let mut failed_files = failed_files.lock().expect(POISONED_FAILED_LOCK);

                // Hold back updates for failed files
                if !failed_files.contains(&entry.fingerprint) {
                    if status == BatchStatus::Delivered {
                        checkpoints.update(entry.fingerprint, entry.offset);
                    } else {
                        error!(
                            message =
                                "Event received a negative acknowledgment, file has been stopped."
                        );

                        failed_files.insert(entry.fingerprint);
                    }
                }
            }

            send_shutdown.send(())
        });

        (Some(finalizer), shutdown2.map(|_| ()).boxed())
    } else {
        // When not dealing with end-to-end acknowledgements, just
        // clone the global shutdown to stop the checkpoint writer.
        (None, shutdown.clone().map(|_| ()).boxed())
    };

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
        ignore_before,
        max_line_bytes: config.max_line_bytes,
        line_delimiter,
    };

    Ok(Box::pin(async move {
        info!(message = "Starting harvest files", ?include, ?exclude,);

        let mut encoding_decoder = charset.map(Decoder::new);

        // sizing here is just a guess
        let (tx, rx) = futures::channel::mpsc::channel::<Vec<Line>>(2);
        let rx = rx
            .map(futures::stream::iter)
            .flatten()
            .map(move |mut line| {
                let failed = failed_files
                    .lock()
                    .expect(POISONED_FAILED_LOCK)
                    .contains(&line.fingerprint);

                // Drop the incoming data if the file received a negative acknowledgement.
                (!failed).then(|| {
                    // transcode each line from the file's encoding charset to utf8
                    line.text = match encoding_decoder.as_mut() {
                        Some(decoder) => decoder.decode_to_utf8(line.text),
                        None => line.text,
                    };

                    line
                })
            })
            .map(futures::stream::iter)
            .flatten();

        let messages: Box<dyn Stream<Item = Line> + Send + Unpin> =
            if let Some(ref multiline_config) = multiline {
                // This match looks ugly, but it does not need `dyn`
                match &multiline_config.parser {
                    Parser::Cri => {
                        let logic = Logic::new(multiline::preset::Cri, multiline_config.timeout);
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
                            Logic::new(multiline::preset::NoIndent, multiline_config.timeout);
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
                    Parser::Custom {
                        condition_pattern,
                        start_pattern,
                        mode,
                    } => {
                        let logic = Logic::new(
                            multiline::RegexRule {
                                start_pattern: start_pattern.clone(),
                                condition_pattern: condition_pattern.clone(),
                                mode: mode.to_owned(),
                            },
                            multiline_config.timeout,
                        );

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
                    &file_key => line.filename,
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
            let result = harvester.run(tx, shutdown, shutdown_checkpointer, checkpointer);
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
    use crate::testing::trace_init;
    use encoding_rs::UTF_16LE;
    use event::log::Value;
    use event::tags::Key;
    use event::EventStatus;
    use framework::{Pipeline, ShutdownSignal};
    use multiline::Mode;
    use std::fs;
    use std::fs::File;
    use std::future::Future;
    use std::io::Write;
    use tempfile::tempdir;
    use tokio::time::{sleep, timeout};

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<TailConfig>()
    }

    fn test_default_tail_config(dir: &tempfile::TempDir) -> TailConfig {
        TailConfig {
            ignore_older_than: None,
            host_key: None,
            include: vec![dir.path().join("*")],
            exclude: vec![],
            read_from: Default::default(),
            file_key: default_file_key(),
            max_line_bytes: default_max_line_bytes(),
            max_read_bytes: default_max_read_bytes(),
            line_delimiter: default_line_delimiter(),
            glob_interval: default_glob_interval(),
            charset: None,
            multiline: None,
        }
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum AckingMode {
        No,
        Unfinalized,
        Acks,
    }

    async fn run_tail(
        config: &TailConfig,
        data_dir: PathBuf,
        wait_shutdown: bool,
        acking_mode: AckingMode,
        inner: impl Future<Output = ()>,
    ) -> Vec<Event> {
        let (tx, rx) = match acking_mode {
            AckingMode::Acks => {
                let (tx, rx) = Pipeline::new_test_finalize(EventStatus::Delivered);
                (tx, rx.boxed())
            }
            AckingMode::No | AckingMode::Unfinalized => {
                let (tx, rx) = Pipeline::new_test();
                (tx, rx.boxed())
            }
        };

        let (trigger_shutdown, shutdown, shutdown_done) = ShutdownSignal::new_wired();
        let acks = !matches!(acking_mode, AckingMode::No);

        // Run the collector concurrent to the file source, to execute finalizers.
        let collector = if acking_mode == AckingMode::Unfinalized {
            tokio::spawn(
                rx.take_until(sleep(Duration::from_secs(5)))
                    .collect::<Vec<_>>(),
            )
        } else {
            tokio::spawn(async {
                timeout(Duration::from_secs(5), rx.collect::<Vec<_>>())
                    .await
                    .expect(
                        "Unclosed channel: may indicate harvester could not shutdown gracefully.",
                    )
            })
        };

        tokio::spawn(tail_source(config, data_dir, shutdown, tx, acks).unwrap());

        inner.await;

        drop(trigger_shutdown);

        if wait_shutdown {
            shutdown_done.await;
        }

        collector.await.expect("Collector task failed")
    }

    async fn sleep_500_millis() {
        tokio::time::sleep(Duration::from_millis(500)).await
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
                    log.tags.get(&Key::from("file")).unwrap().to_string(),
                    path1.to_str().unwrap()
                );
                foo += 1;
            } else {
                assert_eq!(line, format!("bar {}", bar));
                assert_eq!(
                    log.tags.get(&Key::from("file")).unwrap().to_string(),
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

    #[tokio::test]
    #[ignore = "This test ignored for now, it need to be test and pass"]
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
                event
                    .tags()
                    .get(&Key::from("filename"))
                    .unwrap()
                    .to_string(),
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

    #[tokio::test]
    async fn file_file_key_acknowledged() {
        file_file_key(AckingMode::Acks).await
    }

    #[tokio::test]
    async fn file_file_key_nonacknowledged() {
        file_file_key(AckingMode::No).await
    }

    async fn file_file_key(acks: AckingMode) {
        // Default
        {
            let dir = tempdir().unwrap();
            let config = TailConfig {
                include: vec![dir.path().join("*")],
                ..test_default_tail_config(&dir)
            };

            let path = dir.path().join("file");
            let received = run_tail(&config, path.clone(), true, acks.clone(), async {
                let mut file = File::create(&path).unwrap();
                sleep_500_millis().await;
                writeln!(&mut file, "hello there").unwrap();
                sleep_500_millis().await;
            })
            .await;

            assert_eq!(received.len(), 1);
            assert_eq!(
                received[0]
                    .as_log()
                    .tags
                    .get(&Key::from("file"))
                    .unwrap()
                    .to_string(),
                path.to_str().unwrap()
            );
        }

        // Custom
        {
            let dir = tempdir().unwrap();
            let config = TailConfig {
                include: vec![dir.path().join("*")],
                file_key: "source".to_string(),
                ..test_default_tail_config(&dir)
            };

            let path = dir.path().join("file");
            let received = run_tail(&config, path.clone(), true, acks.clone(), async {
                let mut file = File::create(&path).unwrap();
                sleep_500_millis().await;
                writeln!(&mut file, "hello there").unwrap();
                sleep_500_millis().await;
            })
            .await;

            assert_eq!(received.len(), 1);
            assert_eq!(
                received[0]
                    .as_log()
                    .tags
                    .get(&Key::from("source"))
                    .unwrap()
                    .to_string(),
                path.to_str().unwrap()
            );
        }
    }

    fn extract_messages_string(received: Vec<Event>) -> Vec<String> {
        received
            .into_iter()
            .map(Event::into_log)
            .map(|log| {
                log.get_field(log_schema().message_key())
                    .unwrap()
                    .to_string_lossy()
            })
            .collect()
    }

    #[tokio::test]
    async fn file_start_position_server_restart_unfinalized() {
        trace_init();

        let dir = tempdir().unwrap();
        let config = TailConfig {
            include: vec![dir.path().join("*")],
            ..test_default_tail_config(&dir)
        };

        let path = dir.path().join("file");
        let mut file = File::create(&path).unwrap();
        writeln!(&mut file, "the line").unwrap();
        sleep_500_millis().await;

        // First time server runs it picks up existing lines.
        let received = run_tail(
            &config,
            path.clone(),
            false,
            AckingMode::Unfinalized,
            sleep_500_millis(),
        )
        .await;
        let lines = extract_messages_string(received);
        assert_eq!(lines, vec!["the line"]);

        // Restart server, it re-reads file since the events were not acknowledged before shutdown
        let received = run_tail(
            &config,
            path,
            false,
            AckingMode::Unfinalized,
            sleep_500_millis(),
        )
        .await;
        let lines = extract_messages_string(received);
        assert_eq!(lines, vec!["the line"]);
    }

    #[tokio::test]
    async fn file_start_position_server_restart_with_file_rotation_acknowledged() {
        file_start_position_server_restart_with_file_rotation(AckingMode::Acks).await
    }

    #[tokio::test]
    async fn file_start_position_server_restart_with_file_rotation_nonacknowledged() {
        file_start_position_server_restart_with_file_rotation(AckingMode::No).await
    }

    async fn file_start_position_server_restart_with_file_rotation(acking: AckingMode) {
        let dir = tempdir().unwrap();
        let config = TailConfig {
            include: vec![dir.path().join("*")],
            ..test_default_tail_config(&dir)
        };

        let data_dir = dir.path().to_path_buf();
        let path = dir.path().join("file");
        let path_for_old_file = dir.path().join("file.old");
        // Run server first time, collect some lines.
        {
            let received = run_tail(&config, data_dir.clone(), true, acking.clone(), async {
                let mut file = File::create(&path).unwrap();
                sleep_500_millis().await;
                writeln!(&mut file, "first line").unwrap();
                sleep_500_millis().await;
            })
            .await;

            let lines = extract_messages_string(received);
            assert_eq!(lines, vec!["first line"]);
        }
        // Perform 'file rotation' to archive old lines.
        fs::rename(&path, &path_for_old_file).expect("could not rename");
        // Restart the server and make sure it does not re-read the old file
        // even though it has a new name.
        {
            let received = run_tail(&config, data_dir, false, acking, async {
                let mut file = File::create(&path).unwrap();
                sleep_500_millis().await;
                writeln!(&mut file, "second line").unwrap();
                sleep_500_millis().await;
            })
            .await;

            let lines = extract_messages_string(received);
            assert_eq!(lines, vec!["second line"]);
        }
    }

    #[cfg(unix)] // this test uses unix-specific function `futimes` during test time
    #[tokio::test]
    async fn file_start_position_ignore_old_files() {
        use std::{
            os::unix::io::AsRawFd,
            time::{Duration, SystemTime},
        };

        let dir = tempdir().unwrap();
        let path = dir.path().to_path_buf();
        let config = TailConfig {
            include: vec![path.join("*")],
            ignore_older_than: Some(Duration::from_secs(5)),
            ..test_default_tail_config(&dir)
        };

        let received = run_tail(&config, path, false, AckingMode::No, async {
            let before_path = dir.path().join("before");
            let mut before_file = File::create(&before_path).unwrap();
            let after_path = dir.path().join("after");
            let mut after_file = File::create(&after_path).unwrap();

            writeln!(&mut before_file, "first line").unwrap(); // first few bytes make up unique file fingerprint
            writeln!(&mut after_file, "_first line").unwrap(); //   and therefore need to be non-identical

            {
                // Set the modified times
                let before = SystemTime::now() - Duration::from_secs(8);
                let after = SystemTime::now() - Duration::from_secs(2);

                let before_time = libc::timeval {
                    tv_sec: before
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as _,
                    tv_usec: 0,
                };
                let before_times = [before_time, before_time];

                let after_time = libc::timeval {
                    tv_sec: after
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as _,
                    tv_usec: 0,
                };
                let after_times = [after_time, after_time];

                unsafe {
                    libc::futimes(before_file.as_raw_fd(), before_times.as_ptr());
                    libc::futimes(after_file.as_raw_fd(), after_times.as_ptr());
                }
            }

            sleep_500_millis().await;
            writeln!(&mut before_file, "second line").unwrap();
            writeln!(&mut after_file, "_second line").unwrap();

            sleep_500_millis().await;
        })
        .await;

        let file_key = Key::from("file");
        let before_lines = received
            .iter()
            .filter(|event| {
                event
                    .as_log()
                    .get_tag(&file_key)
                    .unwrap()
                    .to_string()
                    .ends_with("before")
            })
            .map(|event| {
                event
                    .as_log()
                    .get_field(log_schema().message_key())
                    .unwrap()
                    .to_string_lossy()
            })
            .collect::<Vec<_>>();
        let after_lines = received
            .iter()
            .filter(|event| {
                event
                    .as_log()
                    .get_tag(&file_key)
                    .unwrap()
                    .to_string()
                    .ends_with("after")
            })
            .map(|event| {
                event
                    .as_log()
                    .get_field(log_schema().message_key())
                    .unwrap()
                    .to_string_lossy()
            })
            .collect::<Vec<_>>();
        assert_eq!(before_lines, vec!["second line"]);
        assert_eq!(after_lines, vec!["_first line", "_second line"]);
    }

    #[tokio::test]
    async fn file_max_line_bytes() {
        let dir = tempdir().unwrap();
        let config = TailConfig {
            include: vec![dir.path().join("*")],
            max_line_bytes: 10,
            ..test_default_tail_config(&dir)
        };

        let path = dir.path().join("file");
        let received = run_tail(&config, path.clone(), false, AckingMode::No, async {
            let mut file = File::create(&path).unwrap();

            sleep_500_millis().await; // The files must be observed at their original lengths before writing to them

            writeln!(&mut file, "short").unwrap();
            writeln!(&mut file, "this is too long").unwrap();
            writeln!(&mut file, "11 eleven11").unwrap();
            let super_long = "This line is super long and will take up more space than BufReader's internal buffer, just to make sure that everything works properly when multiple read calls are involved".repeat(10000);
            writeln!(&mut file, "{}", super_long).unwrap();
            writeln!(&mut file, "exactly 10").unwrap();
            writeln!(&mut file, "it can end on a line that's too long").unwrap();

            sleep_500_millis().await;
            sleep_500_millis().await;

            writeln!(&mut file, "and then continue").unwrap();
            writeln!(&mut file, "last short").unwrap();

            sleep_500_millis().await;
            sleep_500_millis().await;
        }).await;

        let received = extract_messages_value(received);

        assert_eq!(
            received,
            vec!["short".into(), "exactly 10".into(), "last short".into()]
        );
    }

    #[tokio::test]
    async fn test_multi_line_aggregation() {
        let dir = tempdir().unwrap();
        let config = TailConfig {
            include: vec![dir.path().join("*")],
            multiline: Some(MultilineConfig {
                timeout: Duration::from_millis(25),
                parser: Parser::Custom {
                    condition_pattern: regex::bytes::Regex::new("INFO").unwrap(),
                    start_pattern: regex::bytes::Regex::new("INFO").unwrap(),
                    mode: Mode::HaltBefore,
                },
            }),
            ..test_default_tail_config(&dir)
        };

        let path = dir.path().join("file");
        let received = run_tail(&config, path.clone(), false, AckingMode::No, async {
            let mut file = File::create(&path).unwrap();

            sleep_500_millis().await; // The files must be observed at their original lengths before writing to them

            writeln!(&mut file, "leftover foo").unwrap();
            writeln!(&mut file, "INFO hello").unwrap();
            writeln!(&mut file, "INFO goodbye").unwrap();
            writeln!(&mut file, "part of goodbye").unwrap();

            sleep_500_millis().await;

            writeln!(&mut file, "INFO hi again").unwrap();
            writeln!(&mut file, "and some more").unwrap();
            writeln!(&mut file, "INFO hello").unwrap();

            sleep_500_millis().await;

            writeln!(&mut file, "too slow").unwrap();
            writeln!(&mut file, "INFO doesn't have").unwrap();
            writeln!(&mut file, "to be INFO in").unwrap();
            writeln!(&mut file, "the middle").unwrap();

            sleep_500_millis().await;
        })
        .await;

        let received = extract_messages_value(received);

        assert_eq!(
            received,
            vec![
                "leftover foo".into(),
                "INFO hello".into(),
                "INFO goodbye\npart of goodbye".into(),
                "INFO hi again\nand some more".into(),
                "INFO hello".into(),
                "too slow".into(),
                "INFO doesn't have".into(),
                "to be INFO in\nthe middle".into(),
            ]
        );
    }

    // Ignoring on mac: https://github.com/vectordotdev/vector/issues/8373
    #[cfg(not(target_os = "macos"))]
    #[tokio::test]
    async fn test_split_reads() {
        let dir = tempdir().unwrap();
        let config = TailConfig {
            include: vec![dir.path().join("*")],
            max_read_bytes: 1,
            ..test_default_tail_config(&dir)
        };

        let path = dir.path().join("file");
        let mut file = File::create(&path).unwrap();

        writeln!(&mut file, "hello i am a normal line").unwrap();

        sleep_500_millis().await;

        let received = run_tail(&config, dir.into_path(), false, AckingMode::No, async {
            sleep_500_millis().await;

            write!(&mut file, "i am not a full line").unwrap();

            // Longer than the EOF timeout
            sleep_500_millis().await;

            writeln!(&mut file, " until now").unwrap();

            sleep_500_millis().await;
        })
        .await;

        let received = extract_messages_value(received);

        assert_eq!(
            received,
            vec![
                "hello i am a normal line".into(),
                "i am not a full line until now".into(),
            ]
        );
    }

    #[tokio::test]
    async fn test_gzipped_file() {
        let dir = tempdir().unwrap();
        let config = TailConfig {
            ignore_older_than: None,
            include: vec![PathBuf::from("tests/fixtures/gzipped.log")],
            // TODO: remove this once files are fingerprinted after decompression
            //
            // Currently, this needs to be smaller than the total size of the compressed file
            // because the fingerprinter tries to read until a newline, which it's not going to see
            // in the compressed data, or this number of bytes. If it hits EOF before that, it
            // can't return a fingerprint because the value would change once more data is written.
            max_line_bytes: 100,
            ..test_default_tail_config(&dir)
        };

        let received = run_tail(
            &config,
            dir.into_path(),
            false,
            AckingMode::No,
            sleep_500_millis(),
        )
        .await;

        let received = extract_messages_value(received);

        assert_eq!(
            received,
            vec![
                "this is a simple file".into(),
                "i have been compressed".into(),
                "in order to make me smaller".into(),
                "but you can still read me".into(),
                "hooray".into(),
            ]
        );
    }

    #[tokio::test]
    async fn test_non_utf8_encoded_file() {
        let dir = tempdir().unwrap();
        let config = TailConfig {
            include: vec![PathBuf::from("tests/fixtures/utf-16le.log")],
            charset: Some(UTF_16LE),
            ..test_default_tail_config(&dir)
        };

        let received = run_tail(
            &config,
            dir.into_path(),
            false,
            AckingMode::No,
            sleep_500_millis(),
        )
        .await;

        let received = extract_messages_value(received);

        assert_eq!(
            received,
            vec![
                "hello i am a file".into(),
                "i can unicode".into(),
                "but i do so in 16 bits".into(),
                "and when i byte".into(),
                "i become little-endian".into(),
            ]
        );
    }

    #[tokio::test]
    async fn test_non_default_line_delimiter() {
        let dir = tempdir().unwrap();
        let config = TailConfig {
            include: vec![dir.path().join("*")],
            line_delimiter: "\r\n".to_string(),
            ..test_default_tail_config(&dir)
        };

        let path = dir.path().join("file");
        let received = run_tail(&config, path.clone(), false, AckingMode::No, async {
            let mut file = File::create(&path).unwrap();

            sleep_500_millis().await; // The files must be observed at their original lengths before writing to them

            write!(&mut file, "hello i am a line\r\n").unwrap();
            write!(&mut file, "and i am too\r\n").unwrap();
            write!(&mut file, "CRLF is how we end\r\n").unwrap();
            write!(&mut file, "please treat us well\r\n").unwrap();

            sleep_500_millis().await;
        })
        .await;

        let received = extract_messages_value(received);

        assert_eq!(
            received,
            vec![
                "hello i am a line".into(),
                "and i am too".into(),
                "CRLF is how we end".into(),
                "please treat us well".into()
            ]
        );
    }

    fn extract_messages_value(received: Vec<Event>) -> Vec<Value> {
        received
            .into_iter()
            .map(Event::into_log)
            .map(|log| log.get_field(log_schema().message_key()).unwrap().clone())
            .collect()
    }
}
