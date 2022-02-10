use std::time::Duration;

use async_trait::async_trait;
use event::LogRecord;
use framework::config::{
    deserialize_duration, serialize_duration, ticker_from_duration, DataType, GenerateConfig,
    Output, SourceConfig, SourceContext, SourceDescription,
};
use framework::Source;
use futures_util::StreamExt;
use log_schema::log_schema;
use serde::{Deserialize, Serialize};

const fn default_interval() -> Duration {
    Duration::from_secs(1)
}

const fn default_count() -> usize {
    usize::MAX
}

#[derive(Debug, Deserialize, Serialize)]
struct DemoLogsConfig {
    #[serde(
        default = "default_interval",
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
    interval: Duration,

    #[serde(default = "default_count")]
    count: usize,

    log: String,
}

impl GenerateConfig for DemoLogsConfig {
    fn generate_config() -> String {
        format!(
            r#"
# Duration between produce logs
#
interval: 1s

# How many logs to produce.
#
# count: {}

# Log to produce
#
log: abc
"#,
            default_count()
        )
    }
}

inventory::submit! {
    SourceDescription::new::<DemoLogsConfig>("demo_logs")
}

#[async_trait]
#[typetag::serde(name = "demo_logs")]
impl SourceConfig for DemoLogsConfig {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        let mut ticker = ticker_from_duration(self.interval).take_until(cx.shutdown);
        let message = self.log.clone();
        let mut count = self.count;
        let mut output = cx.output;

        Ok(Box::pin(async move {
            while ticker.next().await.is_some() {
                let mut log = LogRecord::from(message.clone());
                log.insert_tag(log_schema().source_type_key(), "demo_logs");

                output.send(log.into()).await.map_err(|err| {
                    error!(message = "Error sending logs", ?err);
                })?;

                count -= 1;
                if count == 0 {
                    break;
                }
            }

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }

    fn source_type(&self) -> &'static str {
        "demo_logs"
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
