use framework::config::{default_interval, GenerateConfig, Output, SourceDescription};
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::{
    config::{deserialize_duration, serialize_duration, DataType, SourceConfig, SourceContext},
    Source,
};
use futures::StreamExt;
use internal::metric::{get_global, init_global, InternalRecorder};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio_stream::wrappers::IntervalStream;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InternalMetricsConfig {
    #[serde(default = "default_interval")]
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
    interval: Duration,
}

impl GenerateConfig for InternalMetricsConfig {
    fn generate_config() -> String {
        r#"
# The interval between scrapes.
#
# Default: 15s
# interval: 15s
"#
        .into()
    }
}

inventory::submit! {
    SourceDescription::new::<InternalMetricsConfig>("internal_metrics")
}

#[async_trait::async_trait]
#[typetag::serde(name = "internal_metrics")]
impl SourceConfig for InternalMetricsConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        init_global()?;
        let recorder = get_global()?;

        Ok(Box::pin(run(
            recorder,
            self.interval,
            ctx.shutdown,
            ctx.output,
        )))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }

    fn source_type(&self) -> &'static str {
        "internal_metrics"
    }
}

async fn run(
    recorder: &InternalRecorder,
    interval: std::time::Duration,
    shutdown: ShutdownSignal,
    mut output: Pipeline,
) -> Result<(), ()> {
    let interval = tokio::time::interval(interval);
    let mut ticker = IntervalStream::new(interval).take_until(shutdown);

    while ticker.next().await.is_some() {
        let timestamp = Some(chrono::Utc::now());
        let mut metrics = recorder.capture_metrics().collect::<Vec<_>>();
        metrics
            .iter_mut()
            .for_each(|metric| metric.timestamp = timestamp);

        if let Err(err) = output.send(metrics).await {
            error!(
                message = "Error sending internal metrics",
                %err
            );

            return Err(());
        }
    }

    Ok(())
}
