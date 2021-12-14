use event::Event;
use futures::{SinkExt, StreamExt};
use internal::metric::{get_global, init_global, InternalRecorder};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use tokio_stream::wrappers::IntervalStream;

use crate::config::{default_interval, GenerateConfig, SourceDescription};

use crate::{
    config::{deserialize_duration, serialize_duration, DataType, SourceConfig, SourceContext},
    pipeline::Pipeline,
    shutdown::ShutdownSignal,
    sources::Source,
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InternalMetricsConfig {
    #[serde(default = "default_interval")]
    #[serde(
        deserialize_with = "deserialize_duration",
        serialize_with = "serialize_duration"
    )]
    interval: chrono::Duration,
}

impl GenerateConfig for InternalMetricsConfig {
    fn generate_config() -> Value {
        serde_yaml::to_value(Self {
            interval: default_interval(),
        })
        .unwrap()
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
            self.interval.to_std().unwrap(),
            ctx.shutdown,
            ctx.out,
        )))
    }

    fn output_type(&self) -> DataType {
        DataType::Metric
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

    while let Some(_) = ticker.next().await {
        let timestamp = Some(chrono::Utc::now());
        let metrics = recorder.capture_metrics();

        let mut stream = futures::stream::iter(metrics)
            .map(|mut m| {
                m.timestamp = timestamp;
                Event::Metric(m)
            })
            .map(Ok);
        output.send_all(&mut stream).await;
    }

    Ok(())
}
