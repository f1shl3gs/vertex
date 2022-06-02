use async_trait::async_trait;
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
        let shutdown = cx.shutdown;
        let client = Client::try_default().await?;

        // By default, all namespace is watched
        let apis: Vec<Api<Event>> = if self.namespaces.is_empty() {
            vec![Api::all(client)]
        } else {
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
                info!(message = "receive", size = evs.len())
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
