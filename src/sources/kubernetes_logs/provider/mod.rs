mod kubelet;
mod kubernetes;
mod pod;

use configurable::Configurable;
use pod::{Container, Pod};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;
use tail::Provider;
use value::{OwnedValuePath, Value, owned_value_path};

pub use kubelet::KubeletProvider;
pub use kubernetes::KubernetesProvider;

fn default_kubelet_interval() -> Duration {
    Duration::from_secs(10)
}

#[derive(Configurable, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum ProviderConfig {
    Kubernetes {
        /// The `name` of the Kubernetes `Node` that Vertex runs at.
        /// Required to filter the `Pod`s to only include the ones with the
        /// log files accessible locally.
        node_name: Option<String>,

        /// Specifies the label selector to filter `Pod`s with, to be used in
        /// addition to the built-in `vertex.io/exclude` filter
        ///
        /// See: https://kubernetes.io/docs/concepts/overview/working-with-objects/labels/#label-selectors
        #[serde(default)]
        label_selector: Option<String>,

        /// Specifies the field selector to filter `Pod`s with, to be used in
        /// addition to the built-in `Node` filter.
        ///
        /// See: https://kubernetes.io/docs/concepts/overview/working-with-objects/field-selectors/#list-of-supported-fields
        #[serde(default)]
        field_selector: Option<String>,
    },

    Kubelet {
        /// HTTP endpoint of the Kubelet's API
        endpoint: Option<String>,

        /// The interval of between each scan
        #[serde(
            default = "default_kubelet_interval",
            with = "humanize::duration::serde"
        )]
        interval: Duration,
    },
}

impl ProviderConfig {
    pub fn build(&self, fields: FieldsConfig) -> Result<KubeProvider, crate::Error> {
        match self {
            Self::Kubelet { endpoint, interval } => {
                let provider = KubeletProvider::new(endpoint.as_ref(), *interval, fields)?;
                Ok(KubeProvider::Kubelet(provider))
            }
            Self::Kubernetes {
                node_name,
                label_selector,
                field_selector,
            } => {
                let node_name = match &node_name {
                    Some(node_name) => node_name.to_string(),
                    None => std::env::var("NODE_NAME").map_err(|_err| {
                        std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            "default environment variable `NODE_NAME` not set",
                        )
                    })?,
                };
                let field_selector = match &field_selector {
                    Some(extra) => format!("spec.nodeName={node_name},{extra}"),
                    None => format!("spec.nodeName={node_name}"),
                };

                KubernetesProvider::new(label_selector.clone(), Some(field_selector), fields)
                    .map(KubeProvider::Kubernetes)
                    .map_err(crate::Error::from)
            }
        }
    }
}

#[allow(clippy::large_enum_variant)]
pub enum KubeProvider {
    Kubelet(KubeletProvider),
    Kubernetes(KubernetesProvider),
}

impl Provider for KubeProvider {
    type Metadata = Value;

    async fn scan(&mut self) -> std::io::Result<Vec<(PathBuf, Self::Metadata)>> {
        match self {
            Self::Kubelet(provider) => provider.scan().await,
            Self::Kubernetes(provider) => provider.scan().await,
        }
    }
}

#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
pub struct PodFieldsConfig {
    name: Option<OwnedValuePath>,
    namespace: Option<OwnedValuePath>,
    uid: Option<OwnedValuePath>,
    ip: Option<OwnedValuePath>,
    ips: Option<OwnedValuePath>,
    labels: Option<OwnedValuePath>,
    annotations: Option<OwnedValuePath>,
    node_name: Option<OwnedValuePath>,
}

impl Default for PodFieldsConfig {
    fn default() -> Self {
        Self {
            name: Some(owned_value_path!("pod", "name")),
            namespace: Some(owned_value_path!("pod", "namespace")),
            uid: Some(owned_value_path!("pod", "uid")),
            ip: Some(owned_value_path!("pod", "ip")),
            ips: Some(owned_value_path!("pod", "ips")),
            labels: Some(owned_value_path!("pod", "labels")),
            annotations: Some(owned_value_path!("pod", "annotations")),
            node_name: Some(owned_value_path!("pod", "node_name")),
        }
    }
}

#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
pub struct ContainerFieldsConfig {
    name: Option<OwnedValuePath>,
    image: Option<OwnedValuePath>,
}

impl Default for ContainerFieldsConfig {
    fn default() -> Self {
        ContainerFieldsConfig {
            name: Some(owned_value_path!("container", "name")),
            image: Some(owned_value_path!("container", "image")),
        }
    }
}

#[derive(Clone, Configurable, Debug, Default, Deserialize, Serialize)]
pub struct FieldsConfig {
    pod: PodFieldsConfig,
    container: ContainerFieldsConfig,
}

impl FieldsConfig {
    pub fn build(&self, pod: &Pod, container: &Container) -> Value {
        let mut value = Value::Object(Default::default());

        // pod info
        if let Some(path) = &self.pod.name {
            value.insert(path, pod.metadata.name.clone());
        }
        if let Some(path) = &self.pod.namespace {
            value.insert(path, pod.metadata.namespace.clone());
        }
        if let Some(path) = &self.pod.uid {
            value.insert(path, pod.metadata.uid.clone());
        }
        if let Some(path) = &self.pod.ip {
            value.insert(path, pod.status.pod_ip.clone());
        }
        if let Some(path) = &self.pod.ips {
            value.insert(
                path,
                pod.status
                    .pod_ips
                    .iter()
                    .map(|item| item.ip.clone())
                    .collect::<Vec<_>>(),
            );
        }
        if let Some(path) = &self.pod.labels {
            value.insert(
                path,
                pod.metadata
                    .labels
                    .iter()
                    .map(|(key, value)| (key.clone(), Value::from(value)))
                    .collect::<BTreeMap<_, _>>(),
            );
        }
        if let Some(path) = &self.pod.annotations {
            value.insert(
                path,
                pod.metadata
                    .annotations
                    .iter()
                    .map(|(key, value)| (key.clone(), Value::from(value)))
                    .collect::<BTreeMap<_, _>>(),
            );
        }
        if let Some(path) = &self.pod.node_name {
            value.insert(path, pod.spec.node_name.clone());
        }

        // container
        if let Some(path) = &self.container.name {
            value.insert(path, container.name.clone());
        }
        if let Some(path) = &self.container.image {
            value.insert(path, container.image.clone());
        }

        value
    }
}

fn generate<'a>(
    fields: &FieldsConfig,
    pods: impl Iterator<Item = &'a Pod>,
) -> Vec<(PathBuf, Value)> {
    let mut paths = Vec::new();

    for pod in pods {
        for container in &pod.spec.containers {
            // When the kubelet creates a static Pod based on a given manifest,
            // it attaches this annotation to the static Pod.
            //
            // https://kubernetes.io/docs/reference/labels-annotations-taints/#kubernetes-io-config-hash
            let uid = match pod.metadata.annotations.get("kubernetes.io/config.hash") {
                Some(value) => value,
                None => &pod.metadata.uid,
            };
            let path = format!(
                "/var/log/pods/{}_{}_{}/{}",
                pod.metadata.namespace, pod.metadata.name, uid, container.name,
            );

            match std::fs::read_dir(&path) {
                Ok(dirs) => {
                    for entry in dirs.flatten() {
                        let path = entry.path();

                        let Some(ext) = path.extension() else {
                            continue;
                        };

                        if ext != "log" {
                            continue;
                        }

                        paths.push((path, fields.build(pod, container)));
                    }
                }
                Err(err) => {
                    warn!(
                        message = "error reading container log directory",
                        ?path,
                        ?err,
                    );
                }
            }
        }
    }

    paths
}
