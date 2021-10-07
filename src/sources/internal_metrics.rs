use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use futures::{SinkExt, StreamExt};
use tokio_stream::wrappers::IntervalStream;
use event::Event;
use crate::{
    config::{DataType, deserialize_duration, serialize_duration, SourceConfig, SourceContext},
    shutdown::ShutdownSignal,
    sources::Source,
};
use internal::metric::{init_global, get_global, InternalRecorder};
use crate::pipeline::Pipeline;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InternalMetricConfig {
    #[serde(deserialize_with = "deserialize_duration", serialize_with = "serialize_duration")]
    interval: chrono::Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "internal_metrics")]
impl SourceConfig for InternalMetricConfig {
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
    let mut ticker = IntervalStream::new(interval)
        .take_until(shutdown);

    while let Some(now) = ticker.next().await {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as i64;
        let metrics = recorder.capture_metrics();

        let mut stream = futures::stream::iter(metrics)
            .map(|mut m| {
                m.timestamp = ts;
                Event::Metric(m)
            })
            .map(Ok);
        output.send_all(&mut stream).await;
    }

    Ok(())
}