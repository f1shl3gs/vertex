use std::path::PathBuf;

use evmap::ReadHandle;
use futures_util::StreamExt;
use k8s_openapi::api::core::v1::{Namespace, Pod};
use tail::provider::Provider;

use crate::kubernetes;

/// A paths provider implementation that uses the state obtained from the
/// k8s API.
pub struct KubernetesPathsProvider {
    pods_state_reader: ReadHandle<String, kubernetes::state::evmap::Value<Pod>>,
    namespace_state_reader: ReadHandle<String, kubernetes::state::evmap::Value<Namespace>>,
}

impl KubernetesPathsProvider {
    pub fn new() -> Self {
        todo!()
    }
}

impl Provider for KubernetesPathsProvider {
    fn scan(&self) -> Vec<PathBuf> {
        vec![]

        /*let read_ref = match self.pods_state_reader.read() {
            Some(v) => v,
            None => {
                // The state is not initialized or gone, fallback to using an empty
                // array.

                // TODO: consider `panic`ing here instead - fail-fast approach
                // is always better if possible, but it's not clear if it's
                // a sane strategy here.
                warn!(message = "Unable to read the state of the pods");

                return Vec::new();
            }
        };

        // filter out pods where we haven't fetched the namespace metadata yet
        // they will be picked up on a later run
        read_ref
            .into_iter()
            .filter(|(uid, values)| {
                let pod: &Pod = values
                    .get_one()
                    .expect("we are supposed to be woring with single-item values only")
                    .as_ref();

                trace!(message = "Verifying Namespace metadata for pod", ?uid);

                if let Some(namespace) = pod.metadata.namespace.as_ref() {
                    self.namespace_state_reader.get(namespace).is_some()
                } else {
                    false
                }
            })
            .flat_map(|(uid, values)| {
                let pod = values
                    .get_one()
                    .expect("we are supposed to be working with single-item values only");
                trace!(message = "Providing log paths for pod", ?uid);

                let paths_iter = list_pod_log_paths(real_glob, pod);
                exclude_paths(paths_iter, &self.exclude_paths)
            })
            .collect()*/
    }
}
/*

fn list_pod_log_paths<'a, G, GI>(
    mut glob_impl: G,
    pod: &'a Pod,
) -> impl Iterator<Item = PathBuf> + 'a
where
    G: FnMut(&str) -> GI + 'a,
    GI: Iterator<Item = PathBuf> + 'a,
{
    todo!()
}

/// This function takes a `Pod` resource and return the path to where the logs
/// for the said `Pod` are expected to be found.
///
/// In the common case, the effective path is built using the `namespace`,
/// `name` and `uid` of the Pod. However, there's a special case for
/// `Static Pod`s: they keep their logs at the path that consists of config
/// hashsum instead of the `Pod`

*/