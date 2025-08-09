use std::time::{Duration, Instant};

use codecs::Decoder;
use configurable::Configurable;
use framework::{Pipeline, ShutdownSignal};
use serde::{Deserialize, Serialize};

use super::{ExecConfig, run_and_send};

#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
pub struct ScheduledConfig {
    /// The interval, in seconds, between scheduled command runs.
    ///
    /// If the command takes longer than `exec_interval_secs` to run, it will be killed.
    #[serde(with = "humanize::duration::serde")]
    #[configurable(required, example = "1m")]
    pub interval: Duration,
}

pub async fn run(
    config: ScheduledConfig,
    exec: ExecConfig,
    hostname: String,
    decoder: Decoder,
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let mut ticker = tokio::time::interval(config.interval);

    loop {
        tokio::select! {
            _ = ticker.tick() => {},
            _ = &mut shutdown => break,
        }

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

        match result {
            Ok(status) => {
                if status.success() {
                    debug!(
                        message = "command exited successfully",
                        command = ?exec.command,
                        elapsed = ?elapsed,
                        code = status.code(),
                    );
                } else {
                    warn!(
                        message = "command exited with an error",
                        command = ?exec.command,
                        elapsed = ?elapsed,
                        code = status.code(),
                    );
                }
            }
            Err(err) => {
                warn!(
                    message = "cannot execute command",
                    command = ?exec.command,
                    elapsed = ?elapsed,
                    ?err,
                );
            }
        }
    }

    Ok(())
}
