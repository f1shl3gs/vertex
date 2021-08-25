use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use crate::sinks::Sink;
use crate::config::{SinkConfig, SinkContext, DataType};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct PrometheusExporterConfig {}

#[async_trait]
#[typetag::serde(name = "prometheus_exporter")]
impl SinkConfig for PrometheusExporterConfig {
    async fn build(&self, ctx: SinkContext) -> crate::Result<Sink> {
        todo!()
    }

    fn input_type(&self) -> DataType {
        todo!()
    }

    fn sink_type(&self) -> &'static str {
        todo!()
    }
}