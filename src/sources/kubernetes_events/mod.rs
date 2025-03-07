use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use chrono::Utc;
use configurable::configurable_component;
use event::LogRecord;
use framework::Source;
use framework::config::{Output, SourceConfig, SourceContext};
use futures::StreamExt;
use futures_util::stream;
use k8s_openapi::api::core::v1::Event;
use kube::runtime::{WatchStreamExt, watcher};
use kube::{Api, Client};
use log_schema::log_schema;

/// The Kubernetes events source collects events from the Kubernetes API server.
/// It collects all the new or updated events that come in.
///
/// Kubernetes version >= 1.22 is required.
#[configurable_component(source, name = "kubernetes_events")]
struct Config {
    /// Namespaces to watch for, if this field is empty, all namespaces will
    /// be watched.
    #[serde(default)]
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
                    watcher(api, watcher::Config::default())
                        .applied_objects()
                        .boxed()
                })
                .collect::<Vec<_>>();

            let mut combined = stream::select_all(watchers)
                .ready_chunks(1024)
                .take_until(shutdown);

            while let Some(evs) = combined.next().await {
                let message_key = log_schema().message_key();
                let timestamp_key = log_schema().timestamp_key();

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
                        let mut map = BTreeMap::new();

                        let timestamp = match ev.event_time {
                            Some(ts) => ts.0,
                            None => Utc::now(),
                        };
                        map.insert(timestamp_key.to_string(), timestamp.into());

                        map.insert(
                            message_key.to_string(),
                            ev.message.unwrap_or_default().into(),
                        );
                        map.insert("reason".to_string(), ev.reason.unwrap_or_default().into());
                        map.insert("action".to_string(), ev.action.unwrap_or_default().into());
                        map.insert("type".to_string(), ev.type_.unwrap_or_default().into());
                        map.insert(
                            "name".to_string(),
                            ev.metadata.name.unwrap_or_default().into(),
                        );
                        map.insert(
                            "namespace".to_string(),
                            ev.metadata.namespace.unwrap_or_default().into(),
                        );
                        map.insert(
                            "uid".to_string(),
                            ev.metadata.uid.unwrap_or_default().into(),
                        );

                        LogRecord::from(map)
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
        vec![Output::logs()]
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
