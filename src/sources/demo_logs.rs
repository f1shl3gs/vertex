use std::time::Duration;

use async_trait::async_trait;
use configurable::configurable_component;
use event::{fields, tags, LogRecord};
use framework::config::{DataType, Output, SourceConfig, SourceContext};
use framework::Source;
use log_schema::log_schema;

const fn default_interval() -> Duration {
    Duration::from_secs(1)
}

const fn default_count() -> usize {
    usize::MAX
}

#[configurable_component(source, name = "demo_logs")]
struct DemoLogsConfig {
    /// How many logs to produce.
    #[serde(default = "default_count")]
    count: usize,

    /// The amount of time, to pause between each batch of output lines.
    #[serde(default = "default_interval", with = "humanize::duration::serde")]
    interval: Duration,

    /// Log content to produce
    #[configurable(required = true)]
    log: String,
}

#[async_trait]
#[typetag::serde(name = "demo_logs")]
impl SourceConfig for DemoLogsConfig {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        let message = self.log.clone();
        let mut output = cx.output;
        let count = self.count;
        let mut ticker = tokio::time::interval(self.interval);
        let mut shutdown = cx.shutdown;

        Ok(Box::pin(async move {
            for _n in 0..count {
                tokio::select! {
                    _ = &mut shutdown => break,
                    _ = ticker.tick() => {}
                }

                let log = LogRecord::new(
                    tags!(
                        "source_type" => "demo_logs",
                    ),
                    fields!(
                        log_schema().message_key() => message.as_str()
                    ),
                );

                if let Err(err) = output.send(log).await {
                    error!(message = "send demo log to output failed", ?err);

                    break;
                }
            }

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<DemoLogsConfig>()
    }
}
