mod event;

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use chrono::Utc;
use configurable::configurable_component;
use framework::config::{Output, SourceConfig, SourceContext};
use framework::{Pipeline, ShutdownSignal, Source};
use futures::StreamExt;
use kubernetes::{Client, WatchEvent, WatchParams};
use tokio::task::JoinSet;
use value::value;

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
        let client = Client::new(None)?;
        let data_dir = cx.globals.make_subdir(cx.key.id())?;
        let (checkpointer, resource_version) = Checkpointer::load(data_dir.join("bookmark.txt"))?;

        let shutdown = cx.shutdown;
        let output = cx.output;
        let namespaces = self.namespaces.clone();

        Ok(Box::pin(async move {
            let mut tasks = JoinSet::default();

            if namespaces.is_empty() {
                tasks.spawn(watch(
                    client,
                    checkpointer,
                    resource_version,
                    output,
                    shutdown,
                ));
            } else {
                for namespace in namespaces {
                    let mut client = client.clone();
                    client.set_namespace(Some(namespace));

                    tasks.spawn(watch(
                        client,
                        checkpointer.clone(),
                        resource_version.clone(),
                        output.clone(),
                        shutdown.clone(),
                    ));
                }
            }

            while tasks.join_next().await.is_some() {}

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

async fn watch(
    client: Client,
    checkpointer: Checkpointer,
    mut resource_version: String,
    mut output: Pipeline,
    mut shutdown: ShutdownSignal,
) {
    let params = WatchParams {
        label_selector: None,
        field_selector: None,
        timeout: None,
        bookmarks: false,
        send_initial_events: false,
    };

    loop {
        let stream = match client
            .watch::<event::Event>(&params, resource_version.clone())
            .await
        {
            Ok(stream) => stream,
            Err(err) => {
                warn!(message = "watch event failed", ?err);

                // backoff
                tokio::time::sleep(Duration::from_secs(5)).await;

                continue;
            }
        };

        tokio::pin!(stream);

        loop {
            let result = tokio::select! {
                _ = &mut shutdown => return,
                result = stream.next() => match result {
                    Some(result) => result,
                    None => break, // watch timeout
                },
            };

            match result {
                Ok(watch_event) => match watch_event {
                    WatchEvent::Added(ev) => {
                        let log = transform(ev);
                        if let Err(_err) = output.send(log).await {
                            return;
                        }
                    }
                    WatchEvent::Bookmark(bookmark) => {
                        resource_version = bookmark.metadata.resource_version;

                        if let Err(err) = checkpointer.persist(&resource_version) {
                            warn!(
                                message = "persisting kubernetes resource version failed",
                                ?err
                            );
                        }
                    }
                    WatchEvent::Error(err) => {
                        error!(message = "watch event failed", ?err);
                    }
                    _ => {}
                },
                Err(err) => {
                    warn!(message = "retrieved event failed", ?err);

                    break;
                }
            }
        }
    }
}

fn transform(ev: event::Event) -> ::event::LogRecord {
    let timestamp = ev.timestamp().unwrap_or_else(Utc::now);
    let message = ev.message.unwrap_or_default();

    let value = value!({
        "timestamp": timestamp,
        "message": message,
        "reason": ev.reason.unwrap_or_default(),
        "action": ev.action.unwrap_or_default(),
        "type": ev.typ.unwrap_or_default(),
        "name": ev.metadata.name,
        "namespace": ev.metadata.namespace,
        "uid": ev.metadata.uid,
    });

    value.into()
}

#[derive(Clone)]
struct Checkpointer {
    path: PathBuf,
}

impl Checkpointer {
    fn load(path: PathBuf) -> std::io::Result<(Checkpointer, String)> {
        let mut point = std::fs::read_to_string(&path)?;

        if point.is_empty() {
            point = "0".to_string();
        }

        Ok((Checkpointer { path }, point))
    }

    #[inline]
    fn persist(&self, data: &str) -> std::io::Result<()> {
        std::fs::write(&self.path, data)
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
