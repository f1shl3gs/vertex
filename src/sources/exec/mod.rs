use std::io::ErrorKind;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::Utc;
use event::Event;
use framework::async_read::VecAsyncReadExt;
use framework::codecs::decoding::{
    BytesDeserializerConfig, DecodingConfig, DeserializerConfig, FramingConfig,
};
use framework::codecs::{Decoder, NewlineDelimitedDecoderConfig, StreamDecodingError};
use framework::config::{
    deserialize_duration, serialize_duration, DataType, GenerateConfig, Output, SourceConfig,
    SourceContext, SourceDescription,
};
use framework::{codecs, Pipeline, ShutdownSignal, Source};
use futures::FutureExt;
use futures_util::StreamExt;
use humanize::{deserialize_bytes, serialize_bytes};
use log_schema::log_schema;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use snafu::Snafu;
use tokio::io::{AsyncRead, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc::{channel, Sender};
use tokio_stream::wrappers::IntervalStream;
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

#[derive(Clone, Debug, Deserialize, Serialize)]
enum RestartPolicy {
    Always,
    Never,
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self::Never
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct ScheduledConfig {
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
    interval: Duration,
}

#[derive(Debug, Deserialize, Serialize)]
struct StreamingConfig {
    #[serde(default)]
    restart_policy: RestartPolicy,
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration",
        default = "default_restart_delay"
    )]
    delay: Duration,
}

const fn default_include_stderr() -> bool {
    true
}

const fn default_maximum_buffer_size() -> usize {
    1024 * 1024 // 1MiB
}

fn default_framing() -> FramingConfig {
    NewlineDelimitedDecoderConfig::new().into()
}

fn default_decoding() -> DeserializerConfig {
    BytesDeserializerConfig::new().into()
}

#[derive(Debug, PartialEq, Snafu)]
pub enum ExecConfigError {
    #[snafu(display("A non-empty list for command must be provided"))]
    CommandEmpty,
    #[snafu(display("The maximum buffer size must be greater than zero"))]
    ZeroBuffer,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecConfig {
    scheduled: Option<ScheduledConfig>,
    streaming: Option<StreamingConfig>,

    command: Vec<String>,
    #[serde(default)]
    working_directory: Option<PathBuf>,

    #[serde(default = "default_include_stderr")]
    include_stderr: bool,
    #[serde(
        default = "default_maximum_buffer_size",
        deserialize_with = "deserialize_bytes",
        serialize_with = "serialize_bytes"
    )]
    maximum_buffer_size: usize,

    #[serde(default = "default_framing")]
    framing: FramingConfig,
    #[serde(default = "default_decoding")]
    decoding: DeserializerConfig,
}

impl ExecConfig {
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

impl GenerateConfig for ExecConfig {
    fn generate_config() -> String {
        format!(
            r#"
# The command to be run, plus any arguments if needed.
#
command:
  - echo
  - $HOSTNAME

# The scheduled options
#
scheduled:
  interval: 10s

# The streaming options
#
# streaming:
#   restart_policy: always
#   delay: 3s

# The scheduled options
#
# Available only when `mode: scheduled`
# interval: 10s

# Configures in which way frames are decoded into events.
#
# Available Options:
#   bytes:     Events containing the byte frame as-is.
#   json:      Events being parsed from a JSON string
#   syslog:    Events being parsed form a Syslog message.
#
# decoding: bytes

# Configuration in which way incoming bytes sequences are split up into byte frames.
#
# framing:
{}

#
"#,
            FramingConfig::generate_commented_with_indent(2)
        )
    }
}

inventory::submit! {
    SourceDescription::new::<ExecConfig>("exec")
}

#[async_trait]
#[typetag::serde(name = "exec")]
impl SourceConfig for ExecConfig {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        self.validate()?;
        let hostname = crate::hostname().ok();
        let decoder = DecodingConfig::new(self.framing.clone(), self.decoding.clone()).build();

        if self.scheduled.is_some() && self.streaming.is_some() {
            return Err("`scheduled` and `streaming` can't be defined at the same time".into());
        }

        if let Some(config) = &self.scheduled {
            Ok(Box::pin(run_scheduled(
                self.command.clone(),
                self.working_directory.clone(),
                self.include_stderr,
                hostname,
                config.interval.clone(),
                decoder,
                cx.shutdown,
                cx.output,
            )))
        } else if let Some(config) = &self.streaming {
            Ok(Box::pin(run_streaming(
                self.command.clone(),
                self.working_directory.clone(),
                self.include_stderr,
                hostname,
                config.restart_policy.clone(),
                config.delay.clone(),
                decoder,
                cx.shutdown,
                cx.output,
            )))
        } else {
            Err("`scheduled` or `streaming` must be defined".into())
        }
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }

    fn source_type(&self) -> &'static str {
        EXEC
    }
}

async fn run_scheduled(
    command: Vec<String>,
    working_directory: Option<PathBuf>,
    include_stderr: bool,
    hostname: Option<String>,
    interval: Duration,
    decoder: codecs::Decoder,
    shutdown: ShutdownSignal,
    output: Pipeline,
) -> Result<(), ()> {
    debug!(message = "Staring scheduled exec runs");

    let mut ticker =
        IntervalStream::new(tokio::time::interval(interval)).take_until(shutdown.clone());

    while ticker.next().await.is_some() {
        // Wait for out task to finish, wrapping it in a timeout
        let timeout = tokio::time::timeout(
            interval,
            run_command(
                command.clone(),
                working_directory.clone(),
                include_stderr,
                hostname.clone(),
                shutdown.clone(),
                decoder.clone(),
                output.clone(),
            ),
        )
        .await;

        match timeout {
            Ok(result) => {
                if let Err(err) = result {
                    error!(message = "Unable to exec", ?err,);
                }
            }
            Err(err) => {
                error!(
                    message = "Timeout during exec",
                    timeout_sec = interval.as_secs(),
                    ?err
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
    hostname: Option<String>,
    shutdown: ShutdownSignal,
    decoder: Decoder,
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

    'send: while let Some(((events, _byte_size), stream)) = receiver.recv().await {
        // TODO: metric

        let total_count = events.len();
        let mut processed_count = 0;

        for mut event in events {
            handle_event(&command, &hostname, &Some(stream), pid, &mut event);

            match output.send(event).await {
                Ok(_) => {
                    processed_count += 1;
                }
                Err(err) => {
                    error!(
                        message = "Failed to forward events, downstream is closed",
                        count = total_count - processed_count,
                        ?err
                    );

                    break 'send;
                }
            }
        }
    }

    debug!(
        message = "Finished command run",
        elapsed_ms = start.elapsed().as_millis() as u64
    );

    match child.try_wait() {
        Ok(Some(exit_status)) => Ok(Some(exit_status)),
        Ok(None) => Ok(None),
        Err(err) => {
            error!(message = "Unable to obtain exit status", ?err);

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

fn handle_event(
    command: &Vec<String>,
    hostname: &Option<String>,
    data_stream: &Option<&str>,
    pid: Option<u32>,
    event: &mut Event,
) {
    if let Event::Log(log) = event {
        // Add timestamp
        log.try_insert_field(log_schema().timestamp_key(), Utc::now());

        // Add source type
        log.insert_tag(log_schema().source_type_key(), EXEC);

        // Add data stream of stdin or stderr(if needed)
        if let Some(data_stream) = data_stream {
            log.try_insert_field(STREAM_KEY, data_stream.to_string());
        }

        // Add pid (if needed)
        if let Some(pid) = pid {
            log.try_insert_field(PID_KEY, pid);
        }

        // Add hostname (if needed)
        if let Some(hostname) = hostname {
            log.try_insert_field(log_schema().host_key(), hostname.clone());
        }

        // Add command
        log.try_insert_field(COMMAND_KEY, command.clone())
    }
}

fn spawn_reader_thread<R: 'static + AsyncRead + Unpin + Send>(
    reader: BufReader<R>,
    decoder: codecs::Decoder,
    origin: &'static str,
    sender: Sender<((SmallVec<[Event; 1]>, usize), &'static str)>,
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
    hostname: Option<String>,
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
                        hostname.clone(),
                        shutdown.clone(),
                        decoder.clone(),
                        output.clone()
                    ) => {
                        // handle command finished
                        if let Err(err) = output {
                            error!(
                                message = "Unable to exec",
                                ?err
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
                hostname,
                shutdown,
                decoder,
                output,
            )
            .await
            {
                error!(message = "Unable to exec", ?err);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<ExecConfig>()
    }
}
