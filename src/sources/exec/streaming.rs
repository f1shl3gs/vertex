use std::time::{Duration, Instant};

use codecs::Decoder;
use configurable::Configurable;
use framework::{Pipeline, ShutdownSignal};
use serde::{Deserialize, Serialize};

use super::{ExecConfig, run_and_send};

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
#[serde(deny_unknown_fields)]
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
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()> {
    loop {
        let start = Instant::now();
        let result = run_and_send(
            &exec,
            hostname.clone(),
            decoder.clone(),
            &mut output,
            shutdown.clone(),
        )
        .await;
        let elapsed = start.elapsed();

        match config.restart {
            RestartPolicy::Always => {}
            RestartPolicy::Never => break,
            RestartPolicy::OnFailure => match result {
                Ok(status) => {
                    if status.success() {
                        return Ok(());
                    }

                    warn!(
                        message = "command exit",
                        command = ?exec.command,
                        ?elapsed,
                        code = status.code(),
                    );
                }
                Err(err) => {
                    error!(
                        message = "command exit failed",
                        comand = ?exec.command,
                        ?elapsed,
                        ?err,
                    );
                }
            },
        }

        tokio::select! {
            biased;

            _ = &mut shutdown => break,
            _ = tokio::time::sleep(config.delay) => {},
        }
    }

    Ok(())
}
