use std::borrow::Cow;
use std::fmt::Debug;

use async_trait::async_trait;
use configurable::configurable_component;
use event::{tags, Trace};
use framework::config::{DataType, Output, SourceConfig, SourceContext};
use framework::Source;
use futures::StreamExt;
use futures_util::stream;
use log_schema::log_schema;

pub fn default_service() -> String {
    "vertex".into()
}

#[configurable_component(source, name = "internal_traces")]
#[derive(Debug)]
#[serde(deny_unknown_fields)]
struct InternalTracesConfig {
    #[serde(default = "default_service")]
    pub service: String,
}

#[async_trait]
#[typetag::serde(name = "internal_traces")]
impl SourceConfig for InternalTracesConfig {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        let subscription = framework::trace::subscribe_spans();
        let shutdown = cx.shutdown;
        let mut output = cx.output;
        let service: Cow<'static, str> = self.service.clone().into();
        let hostname = crate::hostname().unwrap();
        let version = crate::get_version();

        Ok(Box::pin(async move {
            let mut rx = stream::iter(vec![])
                .map(Ok)
                .chain(tokio_stream::wrappers::BroadcastStream::new(
                    subscription.receiver,
                ))
                .filter_map(|span| futures::future::ready(span.ok()))
                .take_until(shutdown);

            while let Some(span) = rx.next().await {
                let mut trace = Trace::new(
                    service.clone(),
                    tags!(
                        log_schema().source_type_key() => "internal_traces"
                    ),
                    vec![span],
                );

                trace.insert_tag("hostanme", hostname.clone());
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
        crate::testing::test_generate_config::<InternalTracesConfig>()
    }
}
