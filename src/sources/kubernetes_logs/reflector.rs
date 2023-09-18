//! The `kube-rs`'s reflector only works with it's own store, which is almost
//! impossible to get an object by name or uid.

use std::hash::Hash;
use std::sync::Arc;

use ahash::AHashMap;
use futures::Stream;
use futures_util::TryStreamExt;
use kube::runtime::watcher;
use kube::Resource;
use parking_lot::RwLock;

#[derive(Clone)]
pub struct Store<K>
where
    K: Resource + 'static,
    K::DynamicType: Eq + Hash,
{
    cache: Arc<RwLock<AHashMap<String, Arc<K>>>>,
}

impl<K: 'static + Resource + Clone> Store<K>
where
    K::DynamicType: Eq + Hash + Clone,
{
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(AHashMap::new())),
        }
    }

    #[inline]
    pub fn get(&self, key: &str) -> Option<Arc<K>> {
        self.cache.read().get(key).cloned()
    }

    #[inline]
    pub fn state(&self) -> Vec<Arc<K>> {
        self.cache.read().values().cloned().collect()
    }
}

pub trait Applier<K>
where
    K: Resource + Clone + 'static,
{
    fn apply(&mut self, event: &watcher::Event<K>);
}

impl<K> Applier<K> for Store<K>
where
    K: Resource + Clone + 'static,
    K::DynamicType: Eq + Hash,
{
    fn apply(&mut self, event: &watcher::Event<K>) {
        match event {
            watcher::Event::Applied(obj) => {
                if let Some(key) = &obj.meta().uid {
                    self.cache
                        .write()
                        .insert(key.to_string(), Arc::new(obj.clone()));
                }
            }

            watcher::Event::Deleted(obj) => {
                if let Some(key) = &obj.meta().uid {
                    self.cache.write().remove(key);
                }
            }

            watcher::Event::Restarted(objs) => {
                let objs = objs
                    .iter()
                    .map(|obj| {
                        (
                            obj.meta().uid.as_ref().unwrap().clone(),
                            Arc::new(obj.clone()),
                        )
                    })
                    .collect::<AHashMap<_, _>>();
                *self.cache.write() = objs;
            }
        }
    }
}

/// Caches objects from `watcher::Event`s to a local `Store`
///
/// Keep in mind that the `Store` is just a cache, and may be out of date.
///
/// Note: It is a bad idea to feed a single `reflector` from multiple `watcher`s, since
/// the whole `Store` will be cleared whenever any of them emits a `Restarted` event.
pub fn reflector<K, W, S>(mut store: S, stream: W) -> impl Stream<Item = W::Item>
where
    K: Resource + Clone + 'static,
    W: Stream<Item = watcher::Result<watcher::Event<K>>>,
    S: Applier<K>,
{
    stream.inspect_ok(move |event| store.apply(event))
}
