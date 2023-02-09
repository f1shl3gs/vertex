use std::collections::BTreeSet;

use async_trait::async_trait;
use chrono::Utc;
use configurable::configurable_component;
use event::{fields, tags, LogRecord};
use framework::config::{DataType, Output, SourceConfig, SourceContext};
use framework::Source;
use futures::StreamExt;
use futures_util::stream;
use k8s_openapi::api::core::v1::Event;
use kube::api::ListParams;
use kube::runtime::{watcher, WatchStreamExt};
use kube::{Api, Client};

#[configurable_component(source, name = "kubernetes_events")]
#[derive(Debug)]
struct Config {
    /// Namespaces to watch for, if this field is empty, all namespaces will
    /// be watched.
    #[serde(default)]
    #[configurable(required)]
    namespaces: Vec<String>,
}

#[async_trait]
#[typetag::serde(name = "kubernetes_events")]
impl SourceConfig for Config {
    async fn build(&self, cx: SourceContext) -> framework::Result<Source> {
        let client = Client::try_default().await?;
        let shutdown = cx.shutdown;
        let mut output = cx.output;
        let start_time = Utc::now();

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
                let message_key = log_schema::log_schema().message_key();
                let timestamp_key = log_schema::log_schema().timestamp_key();
                let source_type_key = log_schema::log_schema().source_type_key();

                let records = evs
                    .into_iter()
                    .filter_map(|result| {
                        // Allow events with event_time(event_time/last_timestamp/first_timestamp)
                        // not older than the receiver start time so that event flood can be avoided
                        // upon startup.
                        let ev = match result {
                            Ok(ev) => ev,
                            Err(_) => return None,
                        };

                        if let Some(ref event_time) = ev.event_time {
                            return if event_time.0 >= start_time {
                                Some(ev)
                            } else {
                                None
                            };
                        }

                        Some(ev)
                    })
                    .map(|ev| {
                        let timestamp = match ev.event_time {
                            Some(ts) => ts.0,
                            None => Utc::now(),
                        };

                        LogRecord::new(
                            tags!(
                                "reason" => ev.reason.unwrap_or_default(),
                                "action" => ev.action.unwrap_or_default(),
                                "type" => ev.type_.unwrap_or_default(),
                                "name" => ev.metadata.name.unwrap_or_default(),
                                "namespace" => ev.metadata.namespace.unwrap_or_default(),
                                "uid" => ev.metadata.uid.unwrap_or_default(),
                                source_type_key => "kubernetes_events",
                            ),
                            fields!(
                                message_key => ev.message.unwrap_or_default(),
                                timestamp_key => timestamp,
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
}
