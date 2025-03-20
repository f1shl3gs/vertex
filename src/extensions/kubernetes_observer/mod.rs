// mod endpoint_slice;
// mod endpoints;
mod ingress;
mod node;
mod pod;
mod service;

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use configurable::{Configurable, configurable_component};
use framework::config::{ExtensionConfig, ExtensionContext};
use framework::observe::{Endpoint, Observer, register};
use framework::{Extension, ShutdownSignal};
use futures::StreamExt;
use kubernetes::{Client, Resource, WatchEvent, WatchParams};
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;

fn default_protocol() -> String {
    String::from("TCP")
}

pub trait Keyed {
    fn key(&self) -> &str;
}

#[derive(Copy, Clone, Configurable, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum ResourceType {
    Node,
    Service,
    Pod,
    // Endpoints,
    // EndpointSlice,
    Ingress,
}

#[configurable_component(extension, name = "kubernetes_observer")]
struct Config {
    /// Optional namespace discovery. If not provided, all namespaces are used.
    #[serde(default)]
    namespaces: Vec<String>,

    /// The Kubernetes role of entities that should be discovered
    resource: ResourceType,

    /// Optional label and field selectors to limit the discovery process to a
    /// subset of available resources.
    ///
    /// See https://kubernetes.io/docs/concepts/overview/working-with-objects/field-selectors/
    /// and https://kubernetes.io/docs/concepts/overview/working-with-objects/labels/
    /// to learn more about the possible filters that can be used.
    label_selector: Option<String>,
    field_selector: Option<String>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "kubernetes_observer")]
impl ExtensionConfig for Config {
    async fn build(&self, cx: ExtensionContext) -> crate::Result<Extension> {
        let client = Client::new(None)?;

        let observer = register(cx.name);
        let namespaces = self.namespaces.clone();
        let label_selector = self.label_selector.clone();
        let field_selector = self.field_selector.clone();

        match self.resource {
            ResourceType::Node => Ok(Box::pin(watch::<node::Node>(
                client,
                namespaces,
                label_selector,
                field_selector,
                observer,
                cx.shutdown,
            ))),
            ResourceType::Service => Ok(Box::pin(watch::<service::Service>(
                client,
                namespaces,
                label_selector,
                field_selector,
                observer,
                cx.shutdown,
            ))),
            ResourceType::Pod => Ok(Box::pin(watch::<pod::Pod>(
                client,
                namespaces,
                label_selector,
                field_selector,
                observer,
                cx.shutdown,
            ))),
            // ResourceType::Endpoints => Ok(Box::pin(watch::<endpoints::Endpoints>(
            //     client,
            //     namespaces,
            //     label_selector,
            //     field_selector,
            //     observer,
            //     cx.shutdown,
            // ))),
            // ResourceType::EndpointSlice => Ok(Box::pin(watch::<endpoint_slice::EndpointSlice>(
            //     client,
            //     namespaces,
            //     label_selector,
            //     field_selector,
            //     observer,
            //     cx.shutdown,
            // ))),
            ResourceType::Ingress => Ok(Box::pin(watch::<ingress::Ingress>(
                client,
                namespaces,
                label_selector,
                field_selector,
                observer,
                cx.shutdown,
            ))),
        }
    }
}

async fn watch<R>(
    client: Client,
    namespaces: Vec<String>,
    label_selector: Option<String>,
    field_selector: Option<String>,
    observer: Observer,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()>
where
    R: Keyed + Resource + Into<Vec<Endpoint>> + 'static,
{
    let cache = Arc::new(Cache::default());
    let mut tasks = JoinSet::new();

    if namespaces.is_empty() {
        tasks.spawn(stream_watch::<R>(
            client,
            label_selector,
            field_selector,
            Arc::clone(&cache),
            shutdown.clone(),
        ));
    } else {
        for namespace in namespaces {
            let mut client = client.clone();
            client.set_namespace(Some(namespace));

            tasks.spawn(stream_watch::<R>(
                client,
                label_selector.clone(),
                field_selector.clone(),
                Arc::clone(&cache),
                shutdown.clone(),
            ));
        }
    }

    let mut ticker = tokio::time::interval(Duration::from_secs(1));
    loop {
        tokio::select! {
            _ = ticker.tick() => {},
            _ = &mut shutdown => break,
        }

        if let Ok(true) =
            cache
                .should_update
                .compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed)
        {
            let _ = observer.publish(cache.endpoints());
        }
    }

    while tasks.join_next().await.is_some() {}

    Ok(())
}

async fn stream_watch<R>(
    client: Client,
    label_selector: Option<String>,
    field_selector: Option<String>,
    cache: Arc<Cache>,
    mut shutdown: ShutdownSignal,
) where
    R: Keyed + Resource + Into<Vec<Endpoint>>,
{
    debug!(message = "start watch kubernetes resources", kind = R::KIND);

    let params = WatchParams {
        label_selector,
        field_selector,
        timeout: None,
        bookmarks: true,
        send_initial_events: false,
    };
    let mut version = "0".to_string();
    loop {
        let stream = match client.watch::<R>(&params, version.clone()).await {
            Ok(stream) => stream.ready_chunks(32),
            Err(err) => {
                warn!(
                    message = "Unable to watch kubernetes version.",
                    ?err,
                    version
                );
                continue;
            }
        };

        tokio::pin!(stream);
        loop {
            let events = tokio::select! {
                _ = &mut shutdown => return,
                result = stream.next() => match result {
                    Some(next) => {
                        match next.into_iter()
                            .collect::<Result<Vec<_>, _>>() {
                            Ok(events) => events,
                            Err(err) => {
                                warn!(
                                    message = "watch kubernetes resource failed",
                                    kind = R::KIND,
                                    ?err,
                                );

                                break
                            }
                        }
                    }
                    None => {break}
                }
            };

            for event in events {
                match event {
                    WatchEvent::Bookmark(bookmark) => {
                        version = bookmark.metadata.resource_version;
                    }
                    WatchEvent::Error(err) => {
                        warn!(
                            message = "Watch resource failed",
                            status = err.status,
                            resp = err.message,
                            reason = err.reason,
                        );

                        break;
                    }
                    event => cache.apply(event),
                }
            }
        }
    }
}

#[derive(Default)]
struct Cache {
    should_update: Arc<AtomicBool>,
    // key is the uid of resource
    endpoints: Arc<Mutex<BTreeMap<String, Vec<Endpoint>>>>,
}

impl Cache {
    fn apply<R: Keyed + Resource + Into<Vec<Endpoint>>>(&self, event: WatchEvent<R>) {
        match event {
            WatchEvent::Added(obj) | WatchEvent::Modified(obj) => {
                let key = obj.key().to_owned();
                let endpoints = obj.into();

                self.endpoints.lock().unwrap().insert(key, endpoints);

                self.should_update.store(true, Ordering::Relaxed);
            }
            WatchEvent::Deleted(obj) => {
                let key = obj.key();

                self.endpoints.lock().unwrap().remove(key);
                self.should_update.store(true, Ordering::Relaxed);
            }
            _ => unreachable!(),
        }
    }

    fn endpoints(&self) -> Vec<Endpoint> {
        self.endpoints
            .lock()
            .unwrap()
            .values()
            .flatten()
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }
}
