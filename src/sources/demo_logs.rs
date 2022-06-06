use std::num::NonZeroU32;
use std::sync::Arc;

use async_trait::async_trait;
use event::{fields, tags, LogRecord};
use framework::config::{
    DataType, GenerateConfig, Output, SourceConfig, SourceContext, SourceDescription,
};
use framework::Source;
use futures_util::StreamExt;
use governor::state::StreamRateLimitExt;
use governor::{Quota, RateLimiter};
use log_schema::log_schema;
use serde::{Deserialize, Serialize};

const fn default_rate() -> u32 {
    1
}

const fn default_count() -> usize {
    usize::MAX
}

#[derive(Debug, Deserialize, Serialize)]
struct DemoLogsConfig {
    #[serde(default = "default_count")]
    count: usize,

    #[serde(default = "default_rate")]
    rate: u32,

    log: String,
}

impl GenerateConfig for DemoLogsConfig {
    fn generate_config() -> String {
        format!(
            r#"
# Rate
#
rate: {}

# How many logs to produce.
#
# count: {}

# Log to produce
#
log: abc
"#,
            default_rate(),
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
        let message = self.log.clone();
        let mut output = cx.output;
        let rate = self.rate;
        let count = self.count;

        Ok(Box::pin(async move {
            let limiter = Arc::new(RateLimiter::direct(Quota::per_second(
                NonZeroU32::new(rate).unwrap(),
            )));
            let mut stream = futures::stream::repeat_with(move || {
                LogRecord::new(
                    tags!(
                        "source_type" => "demo_logs",
                    ),
                    fields!(
                        log_schema().message_key() => message.as_str()
                    ),
                )
            })
            .take(count)
            .take_until(cx.shutdown)
            .ratelimit_stream(&limiter)
            .ready_chunks(1024);

            while let Some(logs) = stream.next().await {
                if let Err(err) = output.send(logs).await {
                    error!(message = "Error sending logs", ?err);
                    return Err(());
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
