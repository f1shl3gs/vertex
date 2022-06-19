mod built_info;
#[cfg(target_os = "linux")]
mod linux;

use std::fmt::Debug;

use event::Metric;
use framework::pipeline::Pipeline;
use framework::shutdown::ShutdownSignal;
use framework::{
    config::{
        default_interval, DataType, GenerateConfig, Output, SourceConfig, SourceContext,
        SourceDescription,
    },
    Source,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::IntervalStream;
use tracing::Instrument;

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
struct SelfStatConfig {}

#[async_trait::async_trait]
#[typetag::serde(name = "selfstat")]
impl SourceConfig for SelfStatConfig {
    async fn build(&self, cx: SourceContext) -> crate::Result<Source> {
        let ss = SelfStat {
            interval: cx.interval,
        };

        Ok(Box::pin(ss.run(cx.shutdown, cx.output)))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Metric)]
    }

    fn source_type(&self) -> &'static str {
        "selfstat"
    }
}

impl GenerateConfig for SelfStatConfig {
    fn generate_config() -> String {
        format!(
            r#"
# The interval between scrapes.
#
interval: {}
"#,
            humanize::duration(&default_interval())
        )
    }
}

inventory::submit! {
    SourceDescription::new::<SelfStatConfig>("selfstat")
}

struct SelfStat {
    interval: std::time::Duration,
}

impl SelfStat {
    async fn run(self, shutdown: ShutdownSignal, mut out: Pipeline) -> Result<(), ()> {
        let interval = tokio::time::interval(self.interval);
        let mut ticker = IntervalStream::new(interval).take_until(shutdown);

        while ticker.next().await.is_some() {
            match gather().instrument(info_span!("selfstat.gather")).await {
                Ok(mut metrics) => {
                    let now = Some(chrono::Utc::now());
                    metrics.iter_mut().for_each(|m| m.timestamp = now);

                    if let Err(err) = out.send(metrics).await {
                        error!(
                            message = "Error sending selfstat metrics",
                            %err
                        );

                        return Err(());
                    }
                }
                Err(err) => {
                    warn!(
                        message = "gather selfstat failed",
                        %err
                    );
                }
            }
        }

        Ok(())
    }
}

async fn gather() -> Result<Vec<Metric>, std::io::Error> {
    #[cfg(target_os = "linux")]
    let mut metrics = linux::proc_info().await?;
    #[cfg(not(target_os = "linux"))]
    let mut metrics = vec![];

    metrics.push(built_info::built_info());

    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<SelfStatConfig>()
    }
}
