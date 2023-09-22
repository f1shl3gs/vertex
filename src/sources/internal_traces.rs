use std::borrow::Cow;
use std::fmt::Debug;

use async_trait::async_trait;
use configurable::configurable_component;
use event::{tags, Trace};
use framework::config::{DataType, Output, SourceConfig, SourceContext};
use framework::trace::SpanSubscription;
use framework::Source;
use futures::StreamExt;
use log_schema::log_schema;

const MAX_CHUNK_SIZE: usize = 128;

pub fn default_service() -> String {
    "vertex".into()
}

/// Exposes Vertex's own internal traces, allowing you to collect, process,
/// and route.
#[configurable_component(source, name = "internal_traces")]
#[serde(deny_unknown_fields)]
struct Config {
    #[serde(default = "default_service")]
    pub service: String,
}

#[async_trait]
#[typetag::serde(name = "internal_traces")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        let subscription = SpanSubscription::subscribe();
        let shutdown = cx.shutdown;
        let mut output = cx.output;
        let service: Cow<'static, str> = self.service.clone().into();
        let hostname = crate::hostname().expect("get hostname success");
        let version = crate::get_version();

        Ok(Box::pin(async move {
            let mut rx = subscription
                .into_stream()
                .ready_chunks(MAX_CHUNK_SIZE)
                .take_until(shutdown);

            while let Some(spans) = rx.next().await {
                let mut trace = Trace::new(
                    service.clone(),
                    tags!(
                        log_schema().source_type_key() => "internal_traces"
                    ),
                    spans,
                );

                trace.insert_tag("hostname", hostname.clone());
                trace.insert_tag("version", version.clone());

                if let Err(err) = output.send(trace).await {
                    warn!(message = "Sending internal trace failed", ?err);
                }
            }

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Trace)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }
}
