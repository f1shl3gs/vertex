use k8s_openapi::api::core::v1::Pod;
use kube::api::ListParams;
use kube::runtime::reflector::Store;
use kube::runtime::watcher;
use kube::{runtime::reflector, Api, Client};
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::{Discoverer, TargetGroup};

const POD_NAME: &str = "__meta_kubernetes_pod_name";
const POD_IP: &str = "__meta_kubernetes_pod_ip";
const POD_CONTAINER_NAME: &str = "__meta_kubernetes_pod_container_name";
const POD_CONTAINER_PORT_NAME: &str = "__meta_kubernetes_pod_container_port_name";
const POD_CONTAINER_PORT_NUMBER: &str = "__meta_kubernetes_pod_container_port_number";
const POD_CONTAINER_PORT_PROTOCOL: &str = "__meta_kubernetes_pod_container_port_+protocol";
const POD_CONTAINER_IS_INIT: &str = "__meta_kubernetes_pod_container_init";
const POD_READY: &str = "__meta_kubernetes_pod_ready";
const POD_PHASE: &str = "__meta_kubernetes_pod_phase";
const POD_NODE_NAME: &str = "__meta_kubernetes_pod_node_name";
const POD_HOST_IP: &str = "__meta_kubernetes_pod_host_ip";
const POD_UID: &str = "__meta_kubernetes_pod_uid";
const POD_CONTROLLER_KIND: &str = "__meta_kubernetes_pod_controller_kind";
const POD_CONTROLLER_NAME: &str = "__meta_kubernetes_pod_controller_name";

const POD_LABEL_PREFIX: &str = "__meta_kubernetes_pod_label_";
const POD_LABEL_PRESENT_PREFIX: &str = "__meta_kubernetes_pod_labelpresent_";
const POD_ANNOTATION_PREFIX: &str = "__meta_kubernetes_pod_annotation_";
const POD_ANNOTATION_PRESENT_PREFIX: &str = "__meta_kubernetes_pod_annotationpresent_";

pub struct PodDiscovery {
    store: Store<Pod>,
}

impl PodDiscovery {
    pub async fn new(namespace: Option<String>) -> Self {
        let client = Client::try_default().await.unwrap();
        let namespace = namespace.unwrap_or_else(|| "default".into());

        let api: Api<Pod> = Api::namespaced(client, &namespace);
        let store_writer = reflector::store::Writer::default();
        let store = store_writer.as_reader();

        let _ = reflector(store_writer, watcher(api, ListParams::default()));

        Self { store }
    }
}

impl Discoverer for PodDiscovery {
    fn targets(&self) -> Vec<TargetGroup> {
        let targets = self
            .store
            .state()
            .iter()
            .map(|pod| {
                let mut map = BTreeMap::new();
                map.insert(
                    POD_NAME.to_string(),
                    pod.metadata
                        .name
                        .as_ref()
                        .map_or("".to_string(), |s| s.to_string()),
                );
                map.insert(
                    POD_IP.to_string(),
                    pod.status
                        .as_ref()
                        .map(|s| s.pod_ip.as_ref())
                        .unwrap_or_default()
                        .map_or_else(|| "".to_string(), |s| s.to_string()),
                );
                map.insert(
                    POD_READY.to_string(),
                    pod_ready(pod).unwrap_or_else(|| "unknown".to_string()),
                );

                map
            })
            .collect::<Vec<_>>();

        vec![TargetGroup {
            targets,
            labels: Default::default(),
            source: Some("kubernetes".to_string()),
        }]
    }
}

fn pod_ready(pod: &Arc<Pod>) -> Option<String> {
    pod.status
        .as_ref()?
        .conditions
        .as_ref()?
        .iter()
        .any(|cond| cond.type_ == "Ready")
        .then(|| "Ready".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kube::config::KubeConfigOptions;

    #[tokio::test]
    async fn list_pods() {
        let config = kube::Config::from_kubeconfig(&KubeConfigOptions::default())
            .await
            .unwrap();
        let client = Client::try_from(config).unwrap();
        let namespace = "kube-system".to_string();

        let api: Api<Pod> = Api::namespaced(client, &namespace);
        let store_writer = reflector::store::Writer::default();
        let store: Store<Pod> = store_writer.as_reader();

        let _ = reflector(store_writer, watcher(api, ListParams::default()));

        store.state().iter().for_each(|pod| {
            println!("{:?}", pod.metadata.name);
        });

        println!("done");
    }
}
