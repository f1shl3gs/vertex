use std::borrow::Cow;
use std::fmt::Debug;

use async_trait::async_trait;
use configurable::configurable_component;
use event::{Trace, tags};
use framework::Source;
use framework::config::{Output, SourceConfig, SourceContext};
use framework::trace::SpanSubscription;
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
    service: String,
}

#[async_trait]
#[typetag::serde(name = "internal_traces")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        let subscription = SpanSubscription::subscribe();
        let shutdown = cx.shutdown;
        let mut output = cx.output;
        let service: Cow<'static, str> = self.service.clone().into();
        let hostname = hostname::get().expect("get hostname success");
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
                        log_schema().source_type_key().to_string() => "internal_traces"
                    ),
                    spans,
                );

                trace.insert_tag("hostname", hostname.clone());
                trace.insert_tag("version", version.clone());

                if let Err(err) = output.send(trace).await {
                    warn!(message = "Sending internal trace failed", %err);
                }
            }

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::traces()]
    }

    fn can_acknowledge(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>()
    }
}
