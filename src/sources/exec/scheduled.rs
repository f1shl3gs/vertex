use std::time::Duration;

use codecs::Decoder;
use configurable::Configurable;
use framework::{Pipeline, ShutdownSignal};
use serde::{Deserialize, Serialize};

use super::{ExecConfig, pump};

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
    output: Pipeline,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let mut ticker = tokio::time::interval(config.interval);

    loop {
        tokio::select! {
            _ = ticker.tick() => {},
            _ = &mut shutdown => break,
        }

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
                    _ = &mut shutdown => break,
                    result = child.wait() => match result {
                        Ok(status) => {
                            if !status.success() {
                                error!(
                                    message = "command exited with non-zero status",
                                    command = ?exec.command,
                                    code = status.code()
                                );
                            }
                        },
                        Err(err) => {
                            error!(
                                message = "wait for child process failed",
                                command = ?exec.command,
                                ?err,
                            );
                        }
                    }
                }
            }
            Err(err) => {
                error!(
                    message = "failed to build and run command",
                    command = ?exec.command,
                    ?err
                );
            }
        }
    }

    Ok(())
}
