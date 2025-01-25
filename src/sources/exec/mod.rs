use std::io::ErrorKind;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::Utc;
use codecs::decoding::StreamDecodingError;
use codecs::decoding::{DeserializerConfig, FramingConfig};
use codecs::DecodingConfig;
use configurable::{configurable_component, Configurable};
use event::log::path::TargetPath;
use event::{event_path, Events};
use framework::async_read::VecAsyncReadExt;
use framework::config::{Output, SourceConfig, SourceContext};
use framework::{Pipeline, ShutdownSignal, Source};
use futures::FutureExt;
use futures_util::StreamExt;
use log_schema::log_schema;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::io::{AsyncRead, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc::{channel, Sender};
use tokio_util::codec::FramedRead;

const EXEC: &str = "exec";
const STDOUT: &str = "stdout";
const STDERR: &str = "stderr";
const STREAM_KEY: &str = "stream";
const PID_KEY: &str = "pid";
const COMMAND_KEY: &str = "command";

const fn default_restart_delay() -> Duration {
    Duration::from_secs(1)
}

#[derive(Configurable, Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum RestartPolicy {
    Always,

    #[default]
    Never,
}

#[derive(Configurable, Debug, Deserialize, Serialize)]
struct ScheduledConfig {
    /// The interval, in seconds, between scheduled command runs.
    ///
    /// If the command takes longer than `exec_interval_secs` to run, it will be killed.
    #[serde(with = "humanize::duration::serde")]
    #[configurable(required, example = "1m")]
    interval: Duration,
}

#[derive(Configurable, Debug, Deserialize, Serialize)]
struct StreamingConfig {
    /// Whether or not the command should be rerun if the command exits.
    #[serde(default)]
    restart_policy: RestartPolicy,

    /// The amount of time, in seconds, that Vertex will wait before rerunning a
    /// streaming command that exited.
    #[serde(default = "default_restart_delay", with = "humanize::duration::serde")]
    delay: Duration,
}

const fn default_include_stderr() -> bool {
    true
}

const fn default_maximum_buffer_size() -> usize {
    1024 * 1024 // 1MiB
}

#[derive(Debug, Error)]
pub enum ExecConfigError {
    #[error("A non-empty list for command must be provided")]
    CommandEmpty,

    #[error("The maximum buffer size must be greater than zero")]
    ZeroBuffer,
}

#[derive(Configurable, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
enum Mode {
    Scheduled(ScheduledConfig),
    Streaming(StreamingConfig),
}

#[configurable_component(source, name = "exec")]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[configurable(required)]
    mode: Mode,

    /// The command to be run, plus any arguments if needed.
    #[configurable(required)]
    command: Vec<String>,

    /// The directory in which to run the command.
    #[serde(default)]
    working_directory: Option<PathBuf>,

    /// Whether or not the output from stderr should be included when generating events.
    #[serde(default = "default_include_stderr")]
    include_stderr: bool,

    /// The maximum buffer size allowed before a log event will be generated.
    #[serde(
        default = "default_maximum_buffer_size",
        with = "humanize::bytes::serde"
    )]
    maximum_buffer_size: usize,

    framing: Option<FramingConfig>,

    #[serde(default)]
    decoding: DeserializerConfig,
}

impl Config {
    fn validate(&self) -> Result<(), ExecConfigError> {
        if self.command.is_empty() {
            Err(ExecConfigError::CommandEmpty)
        } else if self.maximum_buffer_size == 0 {
            Err(ExecConfigError::ZeroBuffer)
        } else {
            Ok(())
        }
    }
}

#[async_trait]
#[typetag::serde(name = "exec")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        self.validate()?;
        let hostname = hostname::get()?;
        let framing = self
            .framing
            .clone()
            .unwrap_or_else(|| self.decoding.default_stream_framing());
        let decoder = DecodingConfig::new(framing, self.decoding.clone()).build();

        match &self.mode {
            Mode::Scheduled(config) => Ok(Box::pin(run_scheduled(
                self.command.clone(),
                self.working_directory.clone(),
                self.include_stderr,
                hostname,
                config.interval,
                decoder,
                cx.shutdown,
                cx.output,
            ))),
            Mode::Streaming(config) => Ok(Box::pin(run_streaming(
                self.command.clone(),
                self.working_directory.clone(),
                self.include_stderr,
                hostname,
                config.restart_policy.clone(),
                config.delay,
                decoder,
                cx.shutdown,
                cx.output,
            ))),
        }
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::logs()]
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

async fn run_scheduled(
    command: Vec<String>,
    working_directory: Option<PathBuf>,
    include_stderr: bool,
    hostname: String,
    interval: Duration,
    decoder: codecs::Decoder,
    mut shutdown: ShutdownSignal,
    output: Pipeline,
) -> Result<(), ()> {
    debug!(message = "Staring scheduled exec runs");

    let mut ticker = tokio::time::interval(interval);

    loop {
        tokio::select! {
            biased;

            _ = &mut shutdown => break,
            _ = ticker.tick() => {}
        }

        // Wait for out task to finish, wrapping it in a timeout
        let timeout = tokio::time::timeout(
            interval,
            run_command(
                command.clone(),
                working_directory.clone(),
                include_stderr,
                &hostname,
                shutdown.clone(),
                decoder.clone(),
                output.clone(),
            ),
        )
        .await;

        match timeout {
            Ok(result) => {
                if let Err(err) = result {
                    error!(message = "Unable to exec", %err,);
                }
            }
            Err(err) => {
                error!(
                    message = "Timeout during exec",
                    timeout_sec = interval.as_secs(),
                    %err
                );
            }
        }
    }

    Ok(())
}

async fn run_command(
    command: Vec<String>,
    working_dir: Option<PathBuf>,
    include_stderr: bool,
    hostname: &str,
    shutdown: ShutdownSignal,
    decoder: codecs::Decoder,
    mut output: Pipeline,
) -> Result<Option<ExitStatus>, std::io::Error> {
    let mut cmd = build_command(command.clone(), working_dir, include_stderr);

    // Mark the start time just before spawning the process as
    // this seems to be the best approximation of exec duration.
    let start = Instant::now();

    let mut child = cmd.spawn()?;

    // Set up communication channels
    let (sender, mut receiver) = channel(1024);

    // Optionally include stderr
    if include_stderr {
        let stderr = child.stderr.take().ok_or_else(|| {
            std::io::Error::new(ErrorKind::Other, "Unable to take stderr of spawned process")
        })?;

        // Crate stderr async reader
        let stderr = stderr.allow_read_until(shutdown.clone().map(|_| ()));
        let stderr_reader = BufReader::new(stderr);

        spawn_reader_thread(stderr_reader, decoder.clone(), STDERR, sender.clone());
    }

    let stdout = child.stdout.take().ok_or_else(|| {
        std::io::Error::new(ErrorKind::Other, "Unable to take stdout of spawned process")
    })?;

    // Create stdout async reader
    let stdout = stdout.allow_read_until(shutdown.clone().map(|_| ()));
    let stdout_reader = BufReader::new(stdout);

    let pid = child.id();

    spawn_reader_thread(stdout_reader, decoder.clone(), STDOUT, sender);

    'send: while let Some(((mut events, _byte_size), stream)) = receiver.recv().await {
        // TODO: metric

        let total_count = events.len();
        let mut processed_count = 0;

        handle_events(&command, hostname, &Some(stream), pid, &mut events);

        // The variable `processed_count` is used in the Err branch
        #[allow(unused_assignments)]
        match output.send(events).await {
            Ok(_) => {
                processed_count += 1;
            }
            Err(err) => {
                error!(
                    message = "Failed to forward events, downstream is closed",
                    count = total_count - processed_count,
                    %err
                );

                break 'send;
            }
        }
    }

    debug!(
        message = "Finished command run",
        elapsed = ?start.elapsed()
    );

    match child.try_wait() {
        Ok(Some(exit_status)) => Ok(Some(exit_status)),
        Ok(None) => Ok(None),
        Err(err) => {
            error!(message = "Unable to obtain exit status", %err);

            Ok(None)
        }
    }
}

fn build_command(
    command: Vec<String>,
    working_directory: Option<PathBuf>,
    include_stderr: bool,
) -> Command {
    let mut cmd = Command::new(&command[0]);

    if command.len() > 1 {
        cmd.args(&command[1..]);
    };

    cmd.kill_on_drop(true);

    // Explicitly set the current dir if needed
    if let Some(current_dir) = &working_directory {
        cmd.current_dir(current_dir);
    }

    // Pipe our stdout to the process
    cmd.stdout(std::process::Stdio::piped());

    // Pipe stderr to the process if needed
    if include_stderr {
        cmd.stderr(std::process::Stdio::piped());
    } else {
        cmd.stderr(std::process::Stdio::null());
    }

    // Stdin is not needed
    cmd.stdin(std::process::Stdio::null());

    cmd
}

fn handle_events(
    command: &[String],
    hostname: &str,
    data_stream: &Option<&str>,
    pid: Option<u32>,
    events: &mut Events,
) {
    events.for_each_log(|log| {
        // Add timestamp and hostname
        log.insert(log_schema().timestamp_key(), Utc::now());
        log.insert(log_schema().host_key(), hostname);

        // Add source type
        log.insert_metadata(log_schema().source_type_key().value_path(), EXEC);

        // Add data stream of stdin or stderr(if needed)
        if let Some(data_stream) = data_stream {
            log.try_insert(event_path!(STREAM_KEY), data_stream.to_string());
        }

        // Add pid (if needed)
        if let Some(pid) = pid {
            log.try_insert(event_path!(PID_KEY), pid);
        }

        // Add command
        log.try_insert(event_path!(COMMAND_KEY), command.to_owned())
    })
}

fn spawn_reader_thread<R: 'static + AsyncRead + Unpin + Send>(
    reader: BufReader<R>,
    decoder: codecs::Decoder,
    origin: &'static str,
    sender: Sender<((Events, usize), &'static str)>,
) {
    // Start collecting
    tokio::spawn(async move {
        debug!(message = "Start capturing command output", origin);

        let mut stream = FramedRead::new(reader, decoder);
        while let Some(result) = stream.next().await {
            match result {
                Ok(next) => {
                    if sender.send((next, origin)).await.is_err() {
                        // If the receive half of the channel is closed, either due to close
                        // being called or the Receiver handle dropping, the function returns an
                        // error.
                        debug!(message = "Receive channel closed, unable to send");

                        break;
                    }
                }

                Err(err) => {
                    // Error is logged by `Decoder`, no further handling is needed.
                    if !err.can_continue() {
                        break;
                    }
                }
            }
        }

        debug!(message = "Finished capturing command output", origin);
    });
}

async fn run_streaming(
    command: Vec<String>,
    working_dir: Option<PathBuf>,
    include_stderr: bool,
    hostname: String,
    restart: RestartPolicy,
    delay: Duration,
    decoder: codecs::Decoder,
    shutdown: ShutdownSignal,
    output: Pipeline,
) -> Result<(), ()> {
    match restart {
        RestartPolicy::Always => {
            // Continue to loop while not shutdown
            loop {
                tokio::select! {
                    // will break early if a shutdown is started
                    _ = shutdown.clone() => break,

                    output = run_command(
                        command.clone(),
                        working_dir.clone(),
                        include_stderr,
                        &hostname,
                        shutdown.clone(),
                        decoder.clone(),
                        output.clone()
                    ) => {
                        // handle command finished
                        if let Err(err) = output {
                            error!(
                                message = "Unable to exec",
                                %err
                            );
                        }
                    }
                }

                let mut poll_shutdown = shutdown.clone();
                if futures::poll!(&mut poll_shutdown).is_pending() {
                    warn!(message = "Streaming process ended before shutdown");
                }

                tokio::select! {
                    // will break early if a shutdown is started
                    _ = &mut poll_shutdown => break,
                    _ = tokio::time::sleep(delay) => debug!(message = "Restarting streaming process")
                }
            }
        }

        RestartPolicy::Never => {
            if let Err(err) = run_command(
                command,
                working_dir,
                include_stderr,
                &hostname,
                shutdown,
                decoder,
                output,
            )
            .await
            {
                error!(message = "Unable to exec", %err);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::task::Poll;

    use event::log::Value;

    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }

    #[test]
    fn test_handle_event() {
        let command = vec!["ls".to_string()];
        let hostname = "localhost";
        let data_stream = Some(STDOUT);
        let pid = Some(123);

        let mut events = Events::Logs(vec!["hello".into()]);
        handle_events(&command, hostname, &data_stream, pid, &mut events);

        let log = events.into_logs().unwrap().remove(0);

        assert_eq!(
            log.get(log_schema().host_key()).unwrap(),
            &Value::from("localhost")
        );
        assert_eq!(log.get(STREAM_KEY).unwrap(), &Value::from(STDOUT));
        assert_eq!(log.get(PID_KEY).unwrap(), &Value::from(123));
        assert_eq!(log.get(COMMAND_KEY).unwrap(), &Value::from(vec!["ls"]));
        assert_eq!(
            log.get(log_schema().message_key()).unwrap(),
            &Value::from("hello")
        );
        assert!(log.get(log_schema().timestamp_key()).is_some())
    }

    #[test]
    fn test_build_command() {
        let command = vec![
            "./runner".to_string(),
            "arg1".to_string(),
            "arg2".to_string(),
        ];

        let command = build_command(
            command,
            Some(PathBuf::from("/tmp")),
            default_include_stderr(),
        );

        let mut expected_command = Command::new("./runner");
        expected_command.kill_on_drop(true);
        expected_command.current_dir("/tmp");
        expected_command.args(vec!["arg1".to_string(), "arg2".to_string()]);

        // Unfortunately the current_dir is not included in the formatted string
        let expected_command_string = format!("{:?}", expected_command);
        let command_string = format!("{:?}", command);

        assert_eq!(expected_command_string, command_string);
    }

    #[tokio::test]
    async fn test_spawn_reader_thread() {
        let buf = Cursor::new("hello\nworld");
        let reader = BufReader::new(buf);
        let decoder = codecs::Decoder::default();
        let (sender, mut receiver) = channel(1024);

        spawn_reader_thread(reader, decoder, STDOUT, sender);

        let mut counter = 0;
        if let Some(((events, bytes), origin)) = receiver.recv().await {
            assert_eq!(bytes, 5);
            assert_eq!(events.len(), 1);

            let log = events.into_logs().unwrap().remove(0);
            assert_eq!(
                log.get(log_schema().message_key()).unwrap(),
                &Value::from("hello")
            );
            assert_eq!(origin, STDOUT);
            counter += 1;
        }

        if let Some(((events, byte_size), origin)) = receiver.recv().await {
            assert_eq!(byte_size, 5);
            assert_eq!(events.len(), 1);

            let log = events.into_logs().unwrap().remove(0);
            assert_eq!(
                log.get(log_schema().message_key()).unwrap(),
                &Value::from("world"),
            );
            assert_eq!(origin, STDOUT);
            counter += 1;
        }

        assert_eq!(counter, 2);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_command_on_unix() {
        let command = vec!["echo".into(), "hello".into()];
        let hostname = "localhost";
        let decoder = Default::default();
        let shutdown = ShutdownSignal::noop();
        let (tx, mut rx) = Pipeline::new_test();

        // Wait for our task to finish, wrapping it in a timeout
        let timeout = tokio::time::timeout(
            Duration::from_secs(3),
            run_command(
                command.clone(),
                None,
                default_include_stderr(),
                hostname,
                shutdown,
                decoder,
                tx,
            ),
        );

        let timeout_result = timeout.await;
        let exit_status = timeout_result
            .expect("command timed out")
            .expect("command error");

        assert_eq!(0, exit_status.unwrap().code().unwrap());

        if let Poll::Ready(Some(events)) = futures::poll!(rx.next()) {
            let log = events.into_logs().unwrap().remove(0);

            assert_eq!(log.get(COMMAND_KEY).unwrap(), &Value::from(command));
            assert_eq!(log.get(STREAM_KEY).unwrap(), &Value::from(STDOUT));
            assert_eq!(
                log.get(log_schema().message_key()).unwrap(),
                &Value::from("hello")
            );
            assert_eq!(
                log.get(log_schema().host_key()).unwrap(),
                &Value::from("localhost")
            );
            assert!(log.get(PID_KEY).is_some());
            assert!(log.get(log_schema().timestamp_key()).is_some());
            assert_eq!(7, log.all_fields().unwrap().count());
        }
    }
}
