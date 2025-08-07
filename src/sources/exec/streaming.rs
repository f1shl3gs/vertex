use std::time::{Duration, Instant};

use codecs::Decoder;
use configurable::Configurable;
use framework::{Pipeline, ShutdownSignal};
use serde::{Deserialize, Serialize};

use super::{ExecConfig, pump};

const fn default_restart_delay() -> Duration {
    Duration::from_secs(5)
}

#[derive(Configurable, Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum RestartPolicy {
    /// The command will be restarted regardless of whether it exited cleanly or not.
    #[default]
    Always,

    /// The command will be restarted, only if the process exits with a non-zero exit code.
    OnFailure,

    /// The command will not be restarted.
    Never,
}

#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
pub struct StreamingConfig {
    /// Whether the command should be rerun if the command exits.
    #[serde(default)]
    restart: RestartPolicy,

    /// The amount of time, in seconds, that Vertex will wait before rerunning a
    /// streaming command that exited.
    #[serde(default = "default_restart_delay", with = "humanize::duration::serde")]
    delay: Duration,
}

pub async fn run(
    config: StreamingConfig,
    exec: ExecConfig,
    hostname: String,
    decoder: Decoder,
    output: Pipeline,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()> {
    loop {
        let start = Instant::now();
        match exec.execute() {
            Ok(mut child) => {
                let pid = child.id();

                if let Some(stdout) = child.stdout.take() {
                    tokio::spawn(pump(
                        stdout,
                        exec.command.clone(),
                        pid,
                        "stdout",
                        hostname.clone(),
                        decoder.clone(),
                        output.clone(),
                        shutdown.clone(),
                    ));
                }
                if let Some(stderr) = child.stderr.take() {
                    tokio::spawn(pump(
                        stderr,
                        exec.command.clone(),
                        pid,
                        "stderr",
                        hostname.clone(),
                        decoder.clone(),
                        output.clone(),
                        shutdown.clone(),
                    ));
                }

                tokio::select! {
                    _ = &mut shutdown => {
                        if let Err(err) = child.kill().await {
                            error!(
                                message = "failed to kill child process",
                                command = ?exec.command,
                                ?err,
                            );
                        }

                        break;
                    },
                    result = child.wait() => {
                        match result {
                            Ok(status) => {
                                let elapsed = start.elapsed();
                                debug!(
                                    message = "command exit",
                                    command = ?exec.command,
                                    ?elapsed,
                                    code = status.code(),
                                );

                                match config.restart {
                                    RestartPolicy::Always => {},
                                    RestartPolicy::Never => break,
                                    RestartPolicy::OnFailure => {
                                        if status.success() {
                                            break;
                                        }
                                    }
                                }
                            },
                            Err(err) => {
                                error!(
                                    message = "wait child process exit failed",
                                    command = ?exec.command,
                                    %err,
                                );
                            }
                        }
                    },
                }
            }
            Err(err) => {
                error!(message = "failed to run command", command = ?exec.command, %err);
            }
        }

        tokio::select! {
            _ = tokio::time::sleep(config.delay) => {},
            _ = &mut shutdown => {},
        }
    }

    Ok(())
}
