use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use configurable::configurable_component;
use framework::Extension;
use framework::config::{ExtensionConfig, ExtensionContext};
use framework::observe::{Endpoint, Observer, run};
use tokio::process::Command;

const fn default_interval() -> Duration {
    Duration::from_secs(10)
}

/// This is a simple but very useful extension, users can write their own script
/// or with any other language. The only requirement is the stdout must be an array
/// of `Endpoint`.
///
/// If the program exit status is not success, then this observer will log it to help
/// user to debug
#[configurable_component(extension, name = "exec_observer")]
struct Config {
    command: PathBuf,

    #[serde(default)]
    args: Vec<String>,

    #[serde(default)]
    working_directory: Option<PathBuf>,

    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "exec_observer")]
impl ExtensionConfig for Config {
    async fn build(&self, cx: ExtensionContext) -> crate::Result<Extension> {
        let observer = Observer::register(cx.key);
        let program = self.command.clone();
        let args = self.args.clone();
        let working_directory = self.working_directory.clone();

        Ok(Box::pin(run(
            observer,
            self.interval,
            cx.shutdown,
            async move || list_endpoints(&program, &args, working_directory.as_ref()).await,
        )))
    }
}

async fn list_endpoints(
    command: &PathBuf,
    args: &[String],
    working_dir: Option<&PathBuf>,
) -> crate::Result<Vec<Endpoint>> {
    let mut cmd = Command::new(command);
    if !args.is_empty() {
        cmd.args(args);
    }

    if let Some(path) = &working_dir {
        cmd.current_dir(path);
    }

    let child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;

    let output = child.wait_with_output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        warn!(
            message = "command exited with non-zero exit code",
            ?command,
            ?args,
            ?working_dir,
            status = ?output.status,
            stderr = stderr.as_ref(),
        );

        return Err(stderr.into());
    }

    serde_json::from_slice(&output.stdout).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }
}
