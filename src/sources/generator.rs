use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};

use crate::config::{deserialize_duration, serialize_duration, SourceConfig, SourceContext, DataType};
use crate::sources::Source;

use tokio_stream::wrappers::IntervalStream;
use crate::event::{Event, DataPoint, Metric, Kind, MetricValue};
use crate::shutdown::ShutdownSignal;
use crate::pipeline::Pipeline;
use std::collections::BTreeMap;

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub struct GeneratorConfig {
    #[serde(deserialize_with = "deserialize_duration", serialize_with = "serialize_duration")]
    #[serde(default = "default_interval")]
    pub interval: chrono::Duration,
}

fn default_interval() -> chrono::Duration {
    chrono::Duration::seconds(15)
}

impl GeneratorConfig {
    async fn inner(self, shutdown: ShutdownSignal, mut out: Pipeline) -> Result<(), ()> {
        let interval = tokio::time::interval(self.interval.to_std().unwrap());
        let mut ticker = IntervalStream::new(interval)
            .take_until(shutdown);

        while ticker.next().await.is_some() {
            let now = chrono::Utc::now();
            let points = vec![
                DataPoint {
                    tags: Default::default(),
                    timestamp: 0,
                    value: MetricValue::Gauge(0.0),
                }
            ];

            let event = Event::Metric(
                Metric {
                    name: "ge".into(),
                    description: None,
                    unit: None,
                    points,
                }
            );

            out.send(event).await.map_err(|err| {
                error!("error: {:?}", err)
            })?;
        }

        Ok(())
    }
}

#[async_trait]
#[typetag::serde(name = "generator")]
impl SourceConfig for GeneratorConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        Ok(Box::pin(self.inner(ctx.shutdown, ctx.out)))
    }

    fn output_type(&self) -> DataType {
        DataType::Metric
    }

    fn source_type(&self) -> &'static str {
        "generator"
    }
}

#[cfg(all(test, feature = "sources-generator"))]
mod tests {
    use super::*;

    #[test]
    fn parse_config() {
        let cf: GeneratorConfig = serde_yaml::from_str(r#"
        interval: 14s
        "#).unwrap();

        assert_eq!(cf.interval, chrono::Duration::seconds(14))
    }
}