mod scheduled;
mod streaming;

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;

use bytes::Bytes;
use chrono::Utc;
use codecs::decoding::{Decoder, DecodingConfig, DeserializerConfig, FramingConfig};
use configurable::{Configurable, configurable_component};
use event::event_path;
use framework::config::{Output, SourceConfig, SourceContext};
use framework::{Pipeline, ShutdownSignal, Source};
use futures::StreamExt;
use scheduled::ScheduledConfig;
use serde::{Deserialize, Serialize};
use streaming::StreamingConfig;
use tokio::io::AsyncRead;
use tokio::process::{Child, Command};
use tokio_util::codec::FramedRead;

const READ_BUFFER_SIZE: usize = 16 * 1024;

const EXEC: &[u8] = b"exec";
const STREAM_KEY: &str = "stream";
const PID_KEY: &str = "pid";
const COMMAND_KEY: &str = "command";

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

    fn outputs(&self) -> Vec<Output> {
        vec![Output::logs()]
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

async fn pump<R: AsyncRead + Unpin>(
    reader: R,
    command: Vec<String>,
    pid: Option<u32>,
    stream: &'static str,
    hostname: String,
    decoder: Decoder,
    mut output: Pipeline,
    shutdown: ShutdownSignal,
) {
    let mut framed =
        FramedRead::with_capacity(reader, decoder, READ_BUFFER_SIZE).take_until(shutdown);
    let hostname = Bytes::from(hostname);
    let stream = Bytes::from_static(stream.as_bytes());
    let exec = Bytes::from_static(EXEC);

    while let Some(result) = framed.next().await {
        match result {
            Ok((mut events, _size)) => {
                events.for_each_log(|log| {
                    // Add timestamp and hostname
                    log.insert("timestamp", Utc::now());
                    log.insert("host", hostname.clone());

                    // Add source type
                    log.insert_metadata("source_type", exec.clone());

                    // Add data stream of stdin or stderr(if needed)
                    log.try_insert(event_path!(STREAM_KEY), stream.clone());

                    // Add pid (if needed)
                    if let Some(pid) = pid {
                        log.try_insert(event_path!(PID_KEY), pid);
                    }

                    // Add command
                    log.try_insert(event_path!(COMMAND_KEY), command.clone());
                });

                if let Err(_err) = output.send(events).await {
                    break;
                }
            }
            Err(err) => {
                error!(message = "error reading framed stream", ?err);
            }
        }
    }
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
            framing: None,
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
