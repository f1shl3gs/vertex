use std::sync::Arc;

use dashmap::DashMap;
use dashmap::mapref::one::Ref;

use super::pod::Pod;

#[derive(Clone)]
pub struct Store {
    cache: Arc<DashMap<String, Pod>>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
        }
    }

    #[inline]
    pub fn inner(&self) -> Arc<DashMap<String, Pod>> {
        Arc::clone(&self.cache)
    }

    pub fn apply(&self, pod: Pod) {
        let key = pod.metadata.uid.clone();

        self.cache.insert(key, pod);
    }

    pub fn delete(&self, key: &str) {
        self.cache.remove(key);
    }

    pub fn get(&self, key: &str) -> Option<Ref<String, Pod>> {
        self.cache.get(key)
    }
}
