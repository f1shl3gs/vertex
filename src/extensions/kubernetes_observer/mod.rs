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
use framework::observe::{Endpoint, Observer};
use framework::{Extension, ShutdownSignal};
use futures::StreamExt;
use kubernetes::{Client, Event, Resource, WatchConfig, watcher};
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

        let observer = Observer::register(cx.name);
        let namespaces = self.namespaces.clone();
        let label_selector = self.label_selector.clone();
        let field_selector = self.field_selector.clone();

        match self.resource {
            ResourceType::Node => Ok(Box::pin(watch_all::<node::Node>(
                client,
                namespaces,
                label_selector,
                field_selector,
                observer,
                cx.shutdown,
            ))),
            ResourceType::Service => Ok(Box::pin(watch_all::<service::Service>(
                client,
                namespaces,
                label_selector,
                field_selector,
                observer,
                cx.shutdown,
            ))),
            ResourceType::Pod => Ok(Box::pin(watch_all::<pod::Pod>(
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
            ResourceType::Ingress => Ok(Box::pin(watch_all::<ingress::Ingress>(
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

async fn watch_all<R>(
    client: Client,
    namespaces: Vec<String>,
    label_selector: Option<String>,
    field_selector: Option<String>,
    observer: Observer,
    mut shutdown: ShutdownSignal,
) -> Result<(), ()>
where
    R: Keyed + Resource + Send + Into<Vec<Endpoint>> + 'static,
{
    let cache = Arc::new(Cache::default());
    let mut tasks = JoinSet::new();

    if namespaces.is_empty() {
        tasks.spawn(watch::<R>(
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

            tasks.spawn(watch::<R>(
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

async fn watch<R>(
    client: Client,
    label_selector: Option<String>,
    field_selector: Option<String>,
    cache: Arc<Cache>,
    mut shutdown: ShutdownSignal,
) where
    R: Keyed + Resource + Into<Vec<Endpoint>> + 'static,
{
    debug!(message = "start watch kubernetes resources", kind = R::KIND);

    let config = WatchConfig {
        label_selector,
        field_selector,
        bookmark: true,
        ..Default::default()
    };

    let stream = watcher::<R>(client, config);
    tokio::pin!(stream);

    let mut new_cache = None;
    loop {
        let event = tokio::select! {
            _ = &mut shutdown => break,
            result = stream.next() => match result {
                Some(Ok(event)) => event,
                Some(Err(err)) => {
                    warn!(message = "watch event failed", ?err, resource = format!("{}.{}.{}", R::GROUP, R::VERSION, R::KIND));
                    break;
                },
                None => break,
            }
        };

        match event {
            Event::Apply(obj) => {
                cache.insert(obj);
            }
            Event::Deleted(obj) => cache.remove(obj.key()),
            Event::Init => {
                new_cache = Some(BTreeMap::new());
            }
            Event::InitApply(obj) => {
                if let Some(new_cache) = new_cache.as_mut() {
                    new_cache.insert(obj.key().to_string(), obj.into());
                }
            }
            Event::InitDone => {
                if let Some(new) = new_cache.take() {
                    cache.replace(new)
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
    fn insert<R: Keyed + Resource + Into<Vec<Endpoint>>>(&self, obj: R) {
        let key = obj.key().to_owned();
        let endpoints = obj.into();

        self.endpoints
            .lock()
            .unwrap()
            .insert(key, endpoints.clone());

        self.should_update.store(true, Ordering::Relaxed);
    }

    fn remove(&self, key: &str) {
        self.endpoints.lock().unwrap().remove(key);

        self.should_update.store(true, Ordering::Relaxed);
    }

    fn replace(&self, endpoints: BTreeMap<String, Vec<Endpoint>>) {
        *self.endpoints.lock().unwrap() = endpoints;
        self.should_update.store(true, Ordering::Relaxed);
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
