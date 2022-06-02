use async_trait::async_trait;
use event::{fields, tags, LogRecord};
use framework::config::{
    DataType, GenerateConfig, Output, SourceConfig, SourceContext, SourceDescription,
};
use framework::Source;
use futures::StreamExt;
use futures_util::stream;
use k8s_openapi::api::core::v1::Event;
use kube::api::ListParams;
use kube::runtime::{watcher, WatchStreamExt};
use kube::{Api, Client};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    #[serde(default)]
    namespaces: Vec<String>,
}

impl GenerateConfig for Config {
    fn generate_config() -> String {
        todo!()
    }
}

inventory::submit! {
    SourceDescription::new::<Config>("kubernetes_events")
}

#[async_trait]
#[typetag::serde(name = "kubernetes_events")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        let client = Client::try_default().await?;
        let shutdown = cx.shutdown;
        let mut output = cx.output;

        // By default, all namespace is watched
        let apis: Vec<Api<Event>> = if self.namespaces.is_empty() {
            vec![Api::all(client)]
        } else {
            // TODO: bookmark!?

            // dedup namespaces
            BTreeSet::from_iter(self.namespaces.clone()) // dedup
                .iter()
                .map(|ns| Api::namespaced(client.clone(), ns))
                .collect()
        };

        Ok(Box::pin(async move {
            let watchers = apis
                .into_iter()
                .map(|api| {
                    watcher(api, ListParams::default())
                        .applied_objects()
                        .boxed()
                })
                .collect::<Vec<_>>();

            let mut combined = stream::select_all(watchers)
                .ready_chunks(1024)
                .take_until(shutdown);

            while let Some(evs) = combined.next().await {
                let message_key = log_schema::log_schema().message_key();

                let records = evs.into_iter().flatten().map(|ev| {
                    // TODO: add more tags and files
                    LogRecord::new(
                        tags!(
                            "reason" => ev.reason.unwrap_or_default(),
                            "action" => ev.action.unwrap_or_default(),
                            "type" => ev.type_.unwrap_or_default(),
                        ),
                        fields!(
                            message_key => ev.message.unwrap_or_default()
                        ),
                    )
                });

                if let Err(err) = output.send_batch(records).await {
                    error!(message = "Error sending kubernetes events", %err);

                    return Ok(());
                }
            }

            Ok(())
        }))
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }

    fn source_type(&self) -> &'static str {
        "kubernetes_events"
    }
}
