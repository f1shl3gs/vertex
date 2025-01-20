use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;
use std::time::{Duration, SystemTime};

use configurable::configurable_component;
use event::{tags, Metric};
use framework::config::{default_interval, Output, SourceConfig, SourceContext};
use framework::{Pipeline, ShutdownSignal, Source};
use glob::glob;

/// The File Stats source collects metrics from files specified with a glob pattern.
#[configurable_component(source, name = "filestats")]
struct Config {
    /// glob pattern to match files
    include: String,

    /// Duration between each sending.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "filestats")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        // validate include
        let _ = glob(&self.include)?;

        Ok(Box::pin(run(
            self.include.clone(),
            self.interval,
            cx.output,
            cx.shutdown,
        )))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::metrics()]
    }
}

async fn run(
    pattern: String,
    interval: Duration,
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let mut ticker = tokio::time::interval(interval);

    loop {
        tokio::select! {
            _ = ticker.tick() => {},
            _ = &mut shutdown => {
                break
            }
        }

        match glob(&pattern) {
            Ok(mut paths) => {
                let mut metrics = vec![];
                while let Some(Ok(path)) = paths.next() {
                    if path.is_dir() {
                        continue;
                    }

                    let partial = stat(&path);
                    metrics.extend(partial);
                }

                if let Err(err) = output.send_batch(metrics).await {
                    warn!(
                        message = "Failed to send metrics",
                        %err
                    );
                }
            }
            Err(err) => {
                warn!(
                    message = "failed to list files",
                    %pattern,
                    %err
                );
            }
        }
    }

    Ok(())
}

fn stat(path: &Path) -> Vec<Metric> {
    match std::fs::metadata(path) {
        Ok(meta) => {
            let mut tags = tags!(
                "path" => path.to_string_lossy().to_string(),
                "permissions" => mode_to_string(meta.permissions().mode()),
            );
            if let Some(name) = path.file_name() {
                tags.insert("name", name.to_string_lossy().to_string());
            }

            let mut metrics = vec![];
            if let Ok(ts) = meta.modified() {
                let value = ts.duration_since(SystemTime::UNIX_EPOCH).unwrap();
                metrics.push(Metric::sum_with_tags(
                    "file_modify_time",
                    "Elapsed time since the last modification of the file or folder, in seconds since Epoch.",
                    value,
                    tags.clone(),
                ));
            }

            if let Ok(ts) = meta.created() {
                let value = ts.duration_since(SystemTime::UNIX_EPOCH).unwrap();
                metrics.push(Metric::sum_with_tags(
                    "file_create_time",
                    "Elapsed time since the last change of the file or folder, in seconds since Epoch. In addition to `file.mtime`, this metric tracks metadata changes such as permissions or renaming the file.",
                    value,
                    tags.clone(),
                ));
            }

            if let Ok(ts) = meta.accessed() {
                let value = ts.duration_since(SystemTime::UNIX_EPOCH).unwrap();
                metrics.push(Metric::sum_with_tags(
                    "file_access_time",
                    "Elapsed time since last access of the file or folder, in seconds since Epoch.",
                    value,
                    tags.clone(),
                ));
            }

            metrics.push(Metric::gauge_with_tags(
                "file_size",
                "The size of the file or folder, in bytes",
                meta.size(),
                tags,
            ));

            metrics
        }
        Err(_err) => {
            vec![]
        }
    }
}

// this function is copied from go's std library
fn mode_to_string(mode: u32) -> String {
    const STR: &[u8; 13] = b"dalTLDpSugct?";
    const RWX: &[u8; 9] = b"rwxrwxrwx";

    let mut buf = [0u8; 32];
    let mut w = 0;
    for (index, char) in STR.iter().enumerate() {
        if mode & (1 << (32 - 1 - index)) != 0 {
            buf[w] = *char;
            w += 1;
        }
    }
    if w == 0 {
        buf[w] = b'-';
        w += 1;
    }

    for (index, char) in RWX.iter().enumerate() {
        if mode & (1 << (9 - 1 - index)) != 0 {
            buf[w] = *char;
        } else {
            buf[w] = b'-';
        }

        w += 1;
    }

    String::from_utf8_lossy(&buf[..w]).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }
}
