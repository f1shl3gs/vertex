use std::io::Read;
use std::time::Duration;

use configurable::configurable_component;
use framework::config::{OutputType, SourceConfig, SourceContext, default_interval};
use framework::{Pipeline, ShutdownSignal, Source};

use crate::common::prometheus::convert_metrics;

/// This source read metrics from Prometheus text format files.
#[configurable_component(source, name = "prometheus_textfile")]
struct Config {
    /// File or directory path pattern
    include: Vec<String>,

    /// Interval between each file reads.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "prometheus_textfile")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        Ok(Box::pin(run(
            self.include.clone(),
            self.interval,
            cx.output,
            cx.shutdown,
        )))
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::metric()]
    }
}

async fn run(
    patterns: Vec<String>,
    interval: Duration,
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()> {
    let mut ticker = tokio::time::interval(interval);

    loop {
        tokio::select! {
            _ = ticker.tick() => {},
            _ = &mut shutdown => break,
        }

        let mut buf = Vec::new();
        for pattern in &patterns {
            let Ok(paths) = glob::glob(pattern) else {
                continue;
            };

            for path in paths.flatten() {
                let size = match std::fs::OpenOptions::new().read(true).open(&path) {
                    Ok(mut file) => match file.read_to_end(&mut buf) {
                        Ok(size) => size,
                        Err(err) => {
                            warn!(message = "read textfile failed", ?path, ?err,);

                            continue;
                        }
                    },
                    Err(err) => {
                        warn!(message = "open textfile failed", ?path, ?err);
                        continue;
                    }
                };

                match prometheus::parse_text(String::from_utf8_lossy(&buf[..size]).as_ref()) {
                    Ok(metrics) => {
                        let metrics = convert_metrics(metrics);

                        if let Err(_err) = output.send(metrics).await {
                            return Ok(());
                        }
                    }
                    Err(err) => {
                        warn!(message = "parse textfile failed", ?path, ?err,);
                    }
                }
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
        crate::testing::generate_config::<Config>();
    }
}
