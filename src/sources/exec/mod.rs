mod scheduled;
mod streaming;

use std::collections::HashMap;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::{ExitStatus, Stdio};
use std::task::{Context, Poll};

use bytes::Bytes;
use chrono::Utc;
use codecs::decoding::{DecodeError, Decoder, DecodingConfig, DeserializerConfig, FramingConfig};
use configurable::{Configurable, configurable_component};
use event::Events;
use framework::config::{OutputType, SourceConfig, SourceContext};
use framework::{Pipeline, ShutdownSignal, Source};
use futures::StreamExt;
use scheduled::ScheduledConfig;
use serde::{Deserialize, Serialize};
use streaming::StreamingConfig;
use tokio::process::{Child, ChildStderr, ChildStdout, Command};
use tokio_util::codec::FramedRead;

const READ_BUFFER_SIZE: usize = 16 * 1024;

#[derive(Configurable, Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Stream {
    #[default]
    All,

    Stdout,

    Stderr,
}

struct ExecConfig {
    command: Vec<String>,
    environment: HashMap<String, String>,
    working_directory: Option<PathBuf>,
    stream: Stream,
}

impl ExecConfig {
    fn execute(&self) -> std::io::Result<Child> {
        let [program, args @ ..] = &self.command[..] else {
            // command is checked already
            unreachable!()
        };

        let mut cmd = Command::new(program);
        if !args.is_empty() {
            cmd.args(args);
        }

        cmd.kill_on_drop(true);

        for (key, value) in &self.environment {
            cmd.env(key, value);
        }

        // Explicitly
        if let Some(current) = &self.working_directory {
            cmd.current_dir(current);
        }

        cmd.stdin(Stdio::null());
        match self.stream {
            Stream::All => {
                cmd.stdout(Stdio::piped());
                cmd.stderr(Stdio::piped());
            }
            Stream::Stdout => {
                cmd.stdout(Stdio::piped());
                cmd.stderr(Stdio::null());
            }
            Stream::Stderr => {
                cmd.stdout(Stdio::null());
                cmd.stderr(Stdio::piped());
            }
        }

        cmd.spawn()
    }
}

#[derive(Configurable, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Mode {
    Scheduled(ScheduledConfig),
    Streaming(StreamingConfig),
}

#[configurable_component(source, name = "exec")]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The command to be run, plus any arguments if needed.
    command: Vec<String>,

    /// The directory in which to run the command.
    #[serde(default)]
    working_directory: Option<PathBuf>,

    #[serde(default)]
    environment: HashMap<String, String>,

    /// Whether the output from stderr should be included when generating events.
    #[serde(default)]
    stream: Stream,

    #[serde(flatten)]
    mode: Mode,

    #[serde(default)]
    framing: Option<FramingConfig>,

    #[serde(default)]
    decoding: DeserializerConfig,
}

#[async_trait::async_trait]
#[typetag::serde(name = "exec")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        let hostname = hostname::get()?;
        let framing = self
            .framing
            .clone()
            .unwrap_or_else(|| self.decoding.default_stream_framing());
        let decoder = DecodingConfig::new(framing, self.decoding.clone()).build()?;

        let exec = ExecConfig {
            command: self.command.clone(),
            environment: self.environment.clone(),
            working_directory: self.working_directory.clone(),
            stream: self.stream,
        };

        Ok(match &self.mode {
            Mode::Scheduled(config) => Box::pin(scheduled::run(
                config.clone(),
                exec,
                hostname,
                decoder,
                cx.output,
                cx.shutdown,
            )),
            Mode::Streaming(config) => Box::pin(streaming::run(
                config.clone(),
                exec,
                hostname,
                decoder,
                cx.output,
                cx.shutdown,
            )),
        })
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::log()]
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

struct Combined {
    stdout: Option<Pin<Box<FramedRead<ChildStdout, Decoder>>>>,
    stderr: Option<Pin<Box<FramedRead<ChildStderr, Decoder>>>>,

    command: Vec<String>,
    hostname: String,
    pid: Option<u32>,
}

impl Combined {
    fn new(
        stdout: Option<ChildStdout>,
        stderr: Option<ChildStderr>,
        command: Vec<String>,
        hostname: String,
        pid: Option<u32>,
        decoder: Decoder,
    ) -> Self {
        let stdout = stdout.map(|inner| {
            Box::pin(FramedRead::with_capacity(
                inner,
                decoder.clone(),
                READ_BUFFER_SIZE,
            ))
        });
        let stderr = stderr
            .map(|inner| Box::pin(FramedRead::with_capacity(inner, decoder, READ_BUFFER_SIZE)));

        Combined {
            stdout,
            stderr,
            command,
            hostname,
            pid,
        }
    }

    fn enrich(self: Pin<&mut Self>, events: &mut Events, stream: &'static str) {
        events.for_each_log(|log| {
            // Add timestamp and hostname
            log.insert("timestamp", Utc::now());
            log.insert("host", self.hostname.clone());

            // Add source type
            log.insert_metadata("source_type", Bytes::from_static(b"exec"));

            // Add data stream of stdin or stderr(if needed)
            log.try_insert("stream", Bytes::from_static(stream.as_bytes()));

            // Add pid (if needed)
            if let Some(pid) = self.pid {
                log.try_insert("pid", pid);
            }

            // Add command
            log.try_insert("command", self.command.clone());
        });
    }
}

impl futures::Stream for Combined {
    type Item = Result<Events, DecodeError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut done = false;
        if let Some(stdout) = &mut self.stdout {
            match stdout.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok((mut events, _size)))) => {
                    self.enrich(&mut events, "stdout");
                    return Poll::Ready(Some(Ok(events)));
                }
                Poll::Ready(Some(Err(err))) => {
                    return Poll::Ready(Some(Err(err)));
                }
                Poll::Pending => {}
                Poll::Ready(None) => {
                    done = true;
                }
            }
        }

        if let Some(stderr) = &mut self.stderr {
            return match stderr.poll_next_unpin(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(None) => {
                    if done {
                        Poll::Ready(None)
                    } else {
                        Poll::Pending
                    }
                }
                Poll::Ready(Some(Ok((mut events, _size)))) => {
                    self.enrich(&mut events, "stderr");
                    Poll::Ready(Some(Ok(events)))
                }
                Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err))),
            };
        }

        Poll::Pending
    }
}

async fn run_and_send(
    exec: &ExecConfig,
    hostname: String,
    decoder: Decoder,
    output: &mut Pipeline,
    mut shutdown: ShutdownSignal,
) -> std::io::Result<ExitStatus> {
    let mut child = exec.execute()?;
    let mut combined = Combined::new(
        child.stdout.take(),
        child.stderr.take(),
        exec.command.clone(),
        hostname,
        child.id(),
        decoder,
    );

    loop {
        tokio::select! {
            _ = &mut shutdown => break,
            result = child.wait() => return result,
            result = combined.next() => match result {
                Some(Ok(events)) => {
                    if let Err(_err) = output.send(events).await {
                        break;
                    }
                },
                Some(Err(err)) => {
                    error!(
                        message = "decode command output failed",
                        command = ?exec.command,
                        ?err,
                    );
                },
                None => {
                    // this shall not happen
                    break;
                }
            }
        }
    }

    child.kill().await?;
    child.wait().await
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use event::log::Value;
    use framework::Pipeline;

    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }

    #[tokio::test]
    async fn scheduled() {
        let config = Config {
            command: vec!["./scheduled.sh".to_string()],
            environment: HashMap::new(),
            working_directory: Some(PathBuf::from("tests/exec")),
            mode: Mode::Scheduled(ScheduledConfig {
                interval: Duration::from_secs(1),
            }),
            stream: Stream::All,
            framing: Default::default(),
            decoding: Default::default(),
        };

        let (tx, mut rx) = Pipeline::new_test();
        let source = config.build(SourceContext::new_test(tx)).await.unwrap();

        tokio::spawn(source);

        tokio::time::sleep(Duration::from_secs(2)).await;

        let stdout = rx.recv().await.unwrap();
        let binding = stdout.into_logs().unwrap();
        let log = binding.first().unwrap();
        assert_eq!(
            log.get("stream"),
            Some(&Value::from(Bytes::from_static(b"stdout")))
        );

        let stderr = rx.recv().await.unwrap();
        let binding = stderr.into_logs().unwrap();
        let log = binding.first().unwrap();
        assert_eq!(
            log.get("stream"),
            Some(&Value::from(Bytes::from_static(b"stderr")))
        );
    }
}
