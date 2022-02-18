use std::time::Duration;

use async_trait::async_trait;
use event::{Event, Metric};
use framework::config::{
    default_interval, deserialize_duration, serialize_duration, DataType, Output, SourceConfig,
    SourceContext,
};
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::Source;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::IntervalStream;

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub struct GeneratorConfig {
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
    #[serde(default = "default_interval")]
    pub interval: Duration,
}

impl GeneratorConfig {
    async fn inner(self, shutdown: ShutdownSignal, mut output: Pipeline) -> Result<(), ()> {
        let interval = tokio::time::interval(self.interval);
        let mut ticker = IntervalStream::new(interval).take_until(shutdown);

        while ticker.next().await.is_some() {
            let now = Some(chrono::Utc::now());
            let event = Event::Metric(Metric::gauge("ge", "", 6).with_timestamp(now));

            output
                .send(event)
                .await
                .map_err(|err| error!("error: {:?}", err))?;
        }

        Ok(())
    }
}

#[async_trait]
#[typetag::serde(name = "generator")]
impl SourceConfig for GeneratorConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        Ok(Box::pin(self.inner(ctx.shutdown, ctx.output)))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }

    fn source_type(&self) -> &'static str {
        "generator"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_config() {
        let cf: GeneratorConfig = serde_yaml::from_str(
            r#"
        interval: 14s
        "#,
        )
        .unwrap();

        assert_eq!(cf.interval, Duration::from_secs(14))
    }
}
