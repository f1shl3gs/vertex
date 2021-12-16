mod client;

use event::Metric;
use serde::{Deserialize, Serialize};

use crate::config::{
    default_std_interval, deserialize_std_duration, serialize_std_duration,
    DataType, SourceConfig, SourceContext,
};
use crate::sources::Source;
use crate::tls::TlsConfig;

#[derive(Debug, Deserialize, Serialize)]
struct ConsulSourceConfig {
    #[serde(default)]
    tls: Option<TlsConfig>,

    endpoints: Vec<String>,

    #[serde(
        default = "default_std_interval",
        deserialize_with = "deserialize_std_duration",
        serialize_with = "serialize_std_duration"
    )]
    interval: std::time::Duration,
}

#[async_trait::async_trait]
#[typetag::serde(name = "consul")]
impl SourceConfig for ConsulSourceConfig {
    async fn build(&self, ctx: SourceContext) -> crate::Result<Source> {
        /*let proxy = ctx.proxy.clone();
        let tls = MaybeTlsSettings::from_config(&self.tls, false)?;
        let interval = tokio::time::interval(self.interval.into());
        let ticker = IntervalStream::new(interval).take_until(ctx.shutdown);
        let output = ctx.out.sink_map_err(|err| {
            error!(
                message = "Error sending consul metrics",
                %err
            )
        });

        Ok(Box::pin(async move {
            while ticker.next().await.is_some() {}

            Ok(())
        }))*/

        todo!()
    }

    fn output_type(&self) -> DataType {
        DataType::Metric
    }

    fn source_type(&self) -> &'static str {
        "consul"
    }
}

async fn gather() -> Vec<Metric> {
    todo!()
}
