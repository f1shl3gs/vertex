use std::borrow::Cow;

use configurable::Configurable;
use event::LogRecord;
use k8s_openapi::{
    api::core::v1::{Container, ContainerStatus, Pod, PodSpec, PodStatus},
    apimachinery::pkg::apis::meta::v1::ObjectMeta,
};
use serde::{Deserialize, Serialize};

use super::reflector::Store;

/// The delimiter used in the log path.
const LOG_PATH_DELIMITER: &str = "_";

pub type Key = Cow<'static, str>;

#[derive(Configurable, Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields, default)]
pub struct FieldsSpec {
    /// Event field for the Pod's name.
    pub pod_name: Key,
    /// Event field for the Pod's namespace.
    pub pod_namespace: Key,
    /// Event field for the Pod's uid.
    pub pod_uid: Key,
    /// Event field for the Pod's IPv4 address.
    pub pod_ip: Key,
    /// Event field for the Pod's IPv4 and IPv6 addresses.
    pub pod_ips: Key,
    /// Event field for the `Pod`'s labels.
    pub pod_labels: Key,
    /// Event field for the Pod's annotations.
    pub pod_annotations: Key,
    /// Event field for the Pod's node_name.
    pub pod_node_name: Key,
    /// Event field for the Pod's owner reference.
    pub pod_owner: Key,
    /// Event field for the Container's name.
    pub container_name: Key,
    /// Event field for the Container's ID.
    pub container_id: Key,
    /// Event field for the Container's image.
    pub container_image: Key,
}

impl Default for FieldsSpec {
    fn default() -> Self {
        Self {
            pod_name: "kubernetes.pod_name".into(),
            pod_namespace: "kubernetes.pod_namespace".into(),
            pod_uid: "kubernetes.pod_uid".into(),
            pod_ip: "kubernetes.pod_ip".into(),
            pod_ips: "kubernetes.pod_ips".into(),
            pod_labels: "kubernetes.pod_labels".into(),
            pod_annotations: "kubernetes.pod_annotations".into(),
            pod_node_name: "kubernetes.pod_node_name".into(),
            pod_owner: "kubernetes.pod_owner".into(),
            container_name: "kubernetes.container_name".into(),
            container_id: "kubernetes.container_id".into(),
            container_image: "kubernetes.container_image".into(),
        }
    }
}

/// Contains the information extracted from the pod log file path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LogFileInfo<'a> {
    pub pod_namespace: &'a str,
    pub pod_name: &'a str,
    pub pod_uid: &'a str,
    pub container_name: &'a str,
}

/// Parses pod log file path and returns the log file info.
///
/// Assumes the input is a valid pod log file name.
///
/// Inspired by https://github.com/kubernetes/kubernetes/blob/31305966789525fca49ec26c289e565467d1f1c4/pkg/kubelet/kuberuntime/helpers.go#L186
pub(super) fn parse_log_file_path(path: &str) -> Option<LogFileInfo<'_>> {
    let mut components = path.rsplit('/');

    let _log_file_name = components.next()?;
    let container_name = components.next()?;
    let pod_dir = components.next()?;

    let mut pod_dir_components = pod_dir.rsplit(LOG_PATH_DELIMITER);

    let pod_uid = pod_dir_components.next()?;
    let pod_name = pod_dir_components.next()?;
    let pod_namespace = pod_dir_components.next()?;

    Some(LogFileInfo {
        pod_namespace,
        pod_name,
        pod_uid,
        container_name,
    })
}

pub struct PodMetadataAnnotator {
    store: Store<Pod>,
    fields_spec: FieldsSpec,
}

impl PodMetadataAnnotator {
    pub const fn new(store: Store<Pod>, fields_spec: FieldsSpec) -> Self {
        Self { store, fields_spec }
    }

    /// Annotates an event with the information from the `Pod::metadata`.
    /// The log has to be obtained from kubernetes log file, and have a
    /// `FILE_KEY` field set with a file that the line came from.
    pub fn annotate<'a>(&self, log: &mut LogRecord, path: &'a str) -> Option<LogFileInfo<'a>> {
        let file_info = parse_log_file_path(path)?;
        let pod = self.store.get(file_info.pod_uid)?;
        let fields_spec = &self.fields_spec;

        annotate_from_file_info(log, fields_spec, &file_info);
        annotate_from_metadata(log, fields_spec, &pod.metadata);

        let container;
        if let Some(ref pod_spec) = pod.spec {
            annotate_from_pod_spec(log, fields_spec, pod_spec);

            container = pod_spec
                .containers
                .iter()
                .find(|c| c.name == file_info.container_name);
            if let Some(container) = container {
                annotate_from_container(log, fields_spec, container);
            }
        }

        if let Some(ref pod_status) = pod.status {
            annotate_from_pod_status(log, fields_spec, pod_status);
            if let Some(ref container_statuses) = pod_status.container_statuses {
                let container_status = container_statuses
                    .iter()
                    .find(|c| c.name == file_info.container_name);
                if let Some(container_status) = container_status {
                    annotate_from_container_status(log, fields_spec, container_status);
                }
            }
        }

        Some(file_info)
    }
}

fn annotate_from_file_info(
    log: &mut LogRecord,
    fields_spec: &FieldsSpec,
    file_info: &LogFileInfo<'_>,
) {
    log.insert_tag(
        fields_spec.container_name.clone(),
        file_info.container_name.to_owned(),
    );
}

fn annotate_from_metadata(log: &mut LogRecord, fields_spec: &FieldsSpec, metadata: &ObjectMeta) {
    if let Some(name) = &metadata.name {
        log.insert_tag(fields_spec.pod_name.clone(), name);
    }

    if let Some(namespace) = &metadata.namespace {
        log.insert_tag(fields_spec.pod_namespace.clone(), namespace);
    }

    if let Some(uid) = &metadata.uid {
        log.insert_tag(fields_spec.pod_uid.clone(), uid);
    }

    if let Some(owner_references) = &metadata.owner_references {
        log.insert_tag(
            fields_spec.pod_owner.clone(),
            format!("{}/{}", owner_references[0].kind, owner_references[0].name),
        );
    }

    if let Some(labels) = &metadata.labels {
        for (key, value) in labels.iter() {
            let key = format!("kubernetes_pod_labels_{}", key);
            log.insert_tag(key, value);
        }
    }

    if let Some(annotations) = &metadata.annotations {
        for (key, value) in annotations.iter() {
            let key = format!("kubernetes_pod_annotations_{}", key);
            log.insert_tag(key, value);
        }
    }
}

fn annotate_from_pod_spec(log: &mut LogRecord, fields_spec: &FieldsSpec, pod_spec: &PodSpec) {
    if let Some(node_name) = &pod_spec.node_name {
        log.insert_tag(fields_spec.pod_node_name.clone(), node_name);
    }
}

fn annotate_from_container(log: &mut LogRecord, fields_spec: &FieldsSpec, container: &Container) {
    if let Some(image) = &container.image {
        log.insert_tag(fields_spec.container_image.clone(), image);
    }
}

fn annotate_from_pod_status(log: &mut LogRecord, fields_spec: &FieldsSpec, pod_status: &PodStatus) {
    if let Some(pod_id) = &pod_status.pod_ip {
        log.insert_tag(fields_spec.pod_ip.clone(), pod_id);
    }

    if let Some(pod_ips) = &pod_status.pod_ips {
        let value: Vec<String> = pod_ips
            .iter()
            .filter_map(|v| v.ip.clone())
            .collect::<Vec<String>>();
        log.insert_tag(fields_spec.pod_ips.clone(), value);
    }
}

fn annotate_from_container_status(
    log: &mut LogRecord,
    fields_spec: &FieldsSpec,
    container_status: &ContainerStatus,
) {
    if let Some(container_id) = &container_status.container_id {
        log.insert_tag(fields_spec.container_id.clone(), container_id);
    }
}

#[cfg(test)]
mod tests {
    use k8s_openapi::api::core::v1::PodIP;
    use testify::assert_event_data_eq;

    use super::*;

    #[test]
    fn test_annotate_from_file_info() {
        let cases = vec![
            (
                FieldsSpec::default(),
                "/var/log/pods/sandbox0-ns_sandbox0-name_sandbox0-uid/sandbox0-container0-name/1.log",
                {
                    let mut log = LogRecord::default();
                    log.insert_tag("kubernetes.container_name", "sandbox0-container0-name");
                    log
                },
            ),
            (
                FieldsSpec {
                    container_name: "container_name".into(),
                    ..Default::default()
                },
                "/var/log/pods/sandbox0-ns_sandbox0-name_sandbox0-uid/sandbox0-container0-name/1.log",
                {
                    let mut log = LogRecord::default();
                    log.insert_tag("container_name", "sandbox0-container0-name");
                    log
                },
            )
        ];

        for (fields_spec, file, expected) in cases.into_iter() {
            let mut log = LogRecord::default();
            let file_info = parse_log_file_path(file).unwrap();
            annotate_from_file_info(&mut log, &fields_spec, &file_info);
            assert_event_data_eq!(log, expected);
        }
    }

    #[test]
    fn test_annotate_from_pod_spec() {
        let cases = vec![
            (
                FieldsSpec::default(),
                PodSpec::default(),
                LogRecord::default(),
            ),
            (
                FieldsSpec::default(),
                PodSpec {
                    node_name: Some("sandbox0-node-name".to_owned()),
                    ..Default::default()
                },
                {
                    let mut log = LogRecord::default();
                    log.insert_tag("kubernetes.pod_node_name", "sandbox0-node-name");
                    log
                },
            ),
            (
                FieldsSpec {
                    pod_node_name: "node_name".into(),
                    ..Default::default()
                },
                PodSpec {
                    node_name: Some("sandbox0-node-name".to_owned()),
                    ..Default::default()
                },
                {
                    let mut log = LogRecord::default();
                    log.insert_tag("node_name", "sandbox0-node-name");
                    log
                },
            ),
        ];

        for (fields_spec, pod_spec, expected) in cases.into_iter() {
            let mut log = LogRecord::default();
            annotate_from_pod_spec(&mut log, &fields_spec, &pod_spec);
            assert_event_data_eq!(log, expected);
        }
    }

    #[test]
    fn test_parse_log_file_path() {
        let cases = vec![
            // Valid inputs.
            (
                "/var/log/pods/sandbox0-ns_sandbox0-name_sandbox0-uid/sandbox0-container0-name/1.log",
                Some(LogFileInfo {
                    pod_namespace: "sandbox0-ns",
                    pod_name: "sandbox0-name",
                    pod_uid: "sandbox0-uid",
                    container_name: "sandbox0-container0-name",
                }),
            ),
            // Invalid inputs.
            ("/var/log/pods/other", None),
            ("qwe", None),
            ("", None),
        ];

        for (input, want) in cases.into_iter() {
            assert_eq!(parse_log_file_path(input), want);
        }
    }

    #[test]
    fn test_annotate_from_container() {
        let cases = vec![
            (
                FieldsSpec::default(),
                Container::default(),
                LogRecord::default(),
            ),
            (
                FieldsSpec::default(),
                Container {
                    image: Some("sandbox0-container-image".to_owned()),
                    ..Default::default()
                },
                {
                    let mut log = LogRecord::default();
                    log.insert_tag("kubernetes.container_image", "sandbox0-container-image");
                    log
                },
            ),
            (
                FieldsSpec {
                    container_image: "container_image".into(),
                    ..Default::default()
                },
                Container {
                    image: Some("sandbox0-container-image".to_owned()),
                    ..Default::default()
                },
                {
                    let mut log = LogRecord::default();
                    log.insert_tag("container_image", "sandbox0-container-image");
                    log
                },
            ),
        ];

        for (fields_spec, container, expected) in cases.into_iter() {
            let mut log = LogRecord::default();
            annotate_from_container(&mut log, &fields_spec, &container);
            assert_event_data_eq!(log, expected);
        }
    }

    #[test]
    fn test_annotate_from_container_status() {
        let cases = vec![
            (
                FieldsSpec::default(),
                ContainerStatus::default(),
                LogRecord::default(),
            ),
            (
                FieldsSpec {
                    ..FieldsSpec::default()
                },
                ContainerStatus {
                    container_id: Some("container_id_foo".to_owned()),
                    ..ContainerStatus::default()
                },
                {
                    let mut log = LogRecord::default();
                    log.insert_tag("kubernetes.container_id", "container_id_foo");
                    log
                },
            ),
        ];
        for (fields_spec, container_status, expected) in cases.into_iter() {
            let mut log = LogRecord::default();
            annotate_from_container_status(&mut log, &fields_spec, &container_status);
            assert_event_data_eq!(log, expected);
        }
    }

    #[test]
    fn test_annotate_from_pod_status() {
        let cases = vec![
            (
                FieldsSpec::default(),
                PodStatus::default(),
                LogRecord::default(),
            ),
            (
                FieldsSpec::default(),
                PodStatus {
                    pod_ip: Some("192.168.1.2".to_owned()),
                    ..Default::default()
                },
                {
                    let mut log = LogRecord::default();
                    log.insert_tag("kubernetes.pod_ip", "192.168.1.2");
                    log
                },
            ),
            (
                FieldsSpec::default(),
                PodStatus {
                    pod_ips: Some(vec![PodIP {
                        ip: Some("192.168.1.2".to_owned()),
                    }]),
                    ..Default::default()
                },
                {
                    let mut log = LogRecord::default();
                    let ips_vec = vec!["192.168.1.2"];
                    log.insert_tag("kubernetes.pod_ips", ips_vec);
                    log
                },
            ),
            (
                FieldsSpec {
                    pod_ip: "kubernetes.custom_pod_ip".into(),
                    pod_ips: "kubernetes.custom_pod_ips".into(),
                    ..FieldsSpec::default()
                },
                PodStatus {
                    pod_ip: Some("192.168.1.2".to_owned()),
                    pod_ips: Some(vec![
                        PodIP {
                            ip: Some("192.168.1.2".to_owned()),
                        },
                        PodIP {
                            ip: Some("192.168.1.3".to_owned()),
                        },
                    ]),
                    ..Default::default()
                },
                {
                    let mut log = LogRecord::default();
                    log.insert_tag("kubernetes.custom_pod_ip", "192.168.1.2");
                    let ips_vec = vec!["192.168.1.2", "192.168.1.3"];
                    log.insert_tag("kubernetes.custom_pod_ips", ips_vec);
                    log
                },
            ),
            (
                FieldsSpec {
                    pod_node_name: "node_name".into(),
                    ..FieldsSpec::default()
                },
                PodStatus {
                    pod_ip: Some("192.168.1.2".to_owned()),
                    pod_ips: Some(vec![
                        PodIP {
                            ip: Some("192.168.1.2".to_owned()),
                        },
                        PodIP {
                            ip: Some("192.168.1.3".to_owned()),
                        },
                    ]),
                    ..Default::default()
                },
                {
                    let mut log = LogRecord::default();
                    log.insert_tag("kubernetes.pod_ip", "192.168.1.2");
                    let ips_vec = vec!["192.168.1.2", "192.168.1.3"];
                    log.insert_tag("kubernetes.pod_ips", ips_vec);
                    log
                },
            ),
        ];

        for (fields_spec, pod_status, expected) in cases.into_iter() {
            let mut log = LogRecord::default();
            annotate_from_pod_status(&mut log, &fields_spec, &pod_status);
            assert_event_data_eq!(log, expected);
        }
    }
}
