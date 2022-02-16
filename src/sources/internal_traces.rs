use std::borrow::Cow;
use std::fmt::Debug;

use async_trait::async_trait;
use event::Trace;
use framework::config::{
    DataType, GenerateConfig, Output, SourceConfig, SourceContext, SourceDescription,
};
use framework::Source;
use futures::StreamExt;
use futures_util::stream;
use serde::{Deserialize, Serialize};

pub fn default_service() -> String {
    "vertex".into()
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InternalTracesConfig {
    #[serde(default = "default_service")]
    pub service: String,
}

impl GenerateConfig for InternalTracesConfig {
    fn generate_config() -> String {
        r#"{}"#.into()
    }
}

inventory::submit! {
    SourceDescription::new::<InternalTracesConfig>("internal_traces")
}

#[async_trait]
#[typetag::serde(name = "internal_traces")]
impl SourceConfig for InternalTracesConfig {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        let subscription = framework::trace::subscribe_spans();
        let shutdown = cx.shutdown;
        let mut output = cx.output;
        let service: Cow<'static, str> = self.service.clone().into();

        Ok(Box::pin(async move {
            let mut rx = stream::iter(vec![])
                .map(Ok)
                .chain(tokio_stream::wrappers::BroadcastStream::new(
                    subscription.receiver,
                ))
                .filter_map(|span| futures::future::ready(span.ok()))
                .take_until(shutdown);

            while let Some(span) = rx.next().await {
                let trace = Trace::new(service.clone(), vec![span]);
                if let Err(err) = output.send(trace.into()).await {
                    warn!(message = "Sending internal trace failed", ?err);
                }
            }

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Trace)]
    }

    fn source_type(&self) -> &'static str {
        "internal_traces"
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
