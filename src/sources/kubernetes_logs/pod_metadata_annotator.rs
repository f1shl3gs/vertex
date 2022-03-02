//! Annotates events with pod metadata.

use evmap::ReadHandle;
use k8s_openapi::api::core::v1::Pod;
use serde::{Deserialize, Serialize};

use crate::kubernetes;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct FieldsSpec {
    pub name: String,
    pub namespace: String,
    pub uid: String,
    pub ip: String,
    pub ips: String,
    pub labels: String,
    pub annotations: String,
    pub node_name: String,
    pub owner: String,
    pub container_name: String,
    pub container_id: String,
    pub container_image: String,
}

impl Default for FieldsSpec {
    fn default() -> Self {
        Self {
            name: "kubernetes.pod_name".to_string(),
            namespace: "kubernetes.pod_namespace".to_string(),
            uid: "kubernetes.pod_uid".to_string(),
            ip: "kubernetes.pod_ip".to_string(),
            ips: "kubernetes.pod_ips".to_string(),
            labels: "kubernetes.pod_labels".to_string(),
            annotations: "kubernetes.pod_annotations".to_string(),
            node_name: "kubernetes.pod_node_name".to_string(),
            owner: "kubernetes.pod_owner".to_string(),
            container_name: "kubernetes.container_name".to_string(),
            container_id: "kubernetes.container_id".to_string(),
            container_image: "kubernetes.container_image".to_string(),
        }
    }
}

/// Annotate the event with pod metadata
pub struct PodMetadataAnnotator {
    pods_state_reader: ReadHandle<String, kubernetes::state::evmap::Value<Pod>>,
}
