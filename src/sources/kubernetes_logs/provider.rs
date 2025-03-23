use std::path::PathBuf;

use kubernetes::ObjectMeta;
use tail::provider::Provider;

use super::pod::Pod;
use super::store::Store;

/// The root directory for pod logs.
const K8S_LOGS_DIR: &str = "/var/log/pods";
/// The delimiter used in the log path.
const LOG_PATH_DELIMITER: &str = "_";

/// A paths provider implementation that uses the state obtained from the
/// k8s API.
pub struct KubernetesPathsProvider {
    exclude_paths: Vec<glob::Pattern>,
    store: Store,
}

impl KubernetesPathsProvider {
    /// Create a new KubernetesPathsProvider
    pub fn new(store: Store, exclude_paths: Vec<glob::Pattern>) -> Self {
        Self {
            exclude_paths,
            store,
        }
    }
}

impl Provider for KubernetesPathsProvider {
    fn scan(&self) -> Vec<PathBuf> {
        self.store
            .inner()
            .iter()
            .flat_map(|entry| {
                let pod = entry.value();

                let pod_paths = list_pod_log_paths(pod, |pattern| {
                    glob::glob_with(
                        pattern,
                        glob::MatchOptions {
                            require_literal_separator: true,
                            ..Default::default()
                        },
                    )
                    .expect("the pattern is supposed to always be correct")
                    .flat_map(|paths| paths.into_iter())
                });

                exclude_paths(pod_paths, &self.exclude_paths).collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    }
}

fn list_pod_log_paths<'a, G, GI>(
    pod: &'a Pod,
    mut glob_impl: G,
) -> impl Iterator<Item = PathBuf> + 'a
where
    G: FnMut(&str) -> GI + 'a,
    GI: Iterator<Item = PathBuf> + 'a,
{
    extract_pod_logs_directory(pod)
        .into_iter()
        .flat_map(move |dir| {
            let dir = dir
                .to_str()
                .expect("non-utf8 path to pod logs dir is not supported");

            // Run the glob to get a list of unfiltered paths
            let path_iter = glob_impl(
                // We seek to match the paths like
                // `<pod_logs_dir>/<container_name>/<n>.log` - paths managed by
                // the `kubelet` as part of Kubernetes core logging architecture.
                // In some setups, there will also be paths like
                // `<pod_logs_dir>/<hash>.log` - those we want to skip.
                &[dir, "*/*.log*"].join("/"),
            );

            // Extract the containers to exclude, then build patterns from
            // them and cache the results into a Vec.
            let excluded_containers = extract_excluded_containers_for_pod(pod);
            let exclusion_patterns: Vec<_> =
                build_container_exclusion_patterns(dir, excluded_containers).collect();

            // Return paths filtered with container exclusion.
            exclude_paths(path_iter, exclusion_patterns)
        })
}

fn build_container_exclusion_patterns<'a>(
    pod_logs_dir: &'a str,
    containers: impl Iterator<Item = &'a str> + 'a,
) -> impl Iterator<Item = glob::Pattern> + 'a {
    containers.filter_map(move |container| {
        let escaped_container_name = glob::Pattern::escape(container);

        glob::Pattern::new(&[pod_logs_dir, &escaped_container_name, "**"].join("/")).ok()
    })
}

fn exclude_paths<'a>(
    iter: impl Iterator<Item = PathBuf> + 'a,
    patterns: impl AsRef<[glob::Pattern]> + 'a,
) -> impl Iterator<Item = PathBuf> + 'a {
    iter.filter(move |path| {
        !patterns.as_ref().iter().any(|pattern| {
            pattern.matches_path_with(
                path,
                glob::MatchOptions {
                    require_literal_separator: true,
                    ..Default::default()
                },
            )
        })
    })
}

/// Extract the static pod config hashsum from the mirror pod annotations
///
/// This part of Kubernetes changed a bit over time, so we're implementing
/// support up to 1.14, which is an MSKV at this time.
///
/// See: <https://github.com/kubernetes/kubernetes/blob/cea1d4e20b4a7886d8ff65f34c6d4f95efcb4742/pkg/kubelet/pod/mirror_client.go#L80-L81>
pub fn extract_static_pod_config_hashsum(meta: &ObjectMeta) -> Option<&str> {
    if meta.annotations.is_empty() {
        return None;
    }

    meta.annotations
        .get("kubernetes.io/config.mirror")
        .map(String::as_str)
}

/// This function takes a `Pod` resource and return the path to where the logs
/// for the said `Pod` are expected to be found.
///
/// In the common case, the effective path is built using the `namespace`,
/// `name` and `uid` of the Pod. However, there's a special case for
/// `Static Pod`s: they keep their logs at the path that consists of config
/// hashsum instead of the `Pod` or `uid`. The reason for this is `kubelet`
/// is locally authoritative over those `Pod`s, and the API only has `Monitor Pod`s
/// the "dummy" entries useful for discovery and association. Their UIDs are
/// generated at the Kubernetes API side, and do not represent the actual
/// config hashsum as one would expect.
///
/// To work around this, we use the mirror pod annotations(if any) to obtain
/// the effective config hashsum, see `extract_static_pod_config_hashsum`
/// function that does this.
///
/// See https://github.com/kubernetes/kubernetes/blob/ef3337a443b402756c9f0bfb1f844b1b45ce289d/pkg/kubelet/pod/pod_manager.go#L30-L44
/// See https://github.com/kubernetes/kubernetes/blob/cea1d4e20b4a7886d8ff65f34c6d4f95efcb4742/pkg/kubelet/pod/mirror_client.go#L80-L81
fn extract_pod_logs_directory(pod: &Pod) -> Option<PathBuf> {
    let metadata = &pod.metadata;
    let name = metadata.name.as_str();
    let namespace = metadata.namespace.as_str();

    if name.is_empty() || namespace.is_empty() {
        return None;
    }

    let uid = if let Some(static_pod_config_hashsum) = extract_static_pod_config_hashsum(metadata) {
        // If there's a static pod config hashsum - use it instead of uid
        static_pod_config_hashsum
    } else {
        // In the common case - just fallback to the real pod uid
        if metadata.uid.is_empty() {
            return None;
        }

        metadata.uid.as_str()
    };

    Some(build_pod_logs_directory(namespace, name, uid))
}

/// Builds absolute log directory path for a pod sandbox.
///
/// Based on https://github.com/kubernetes/kubernetes/blob/31305966789525fca49ec26c289e565467d1f1c4/pkg/kubelet/kuberuntime/helpers.go#L178
pub fn build_pod_logs_directory(namespace: &str, name: &str, uid: &str) -> PathBuf {
    [
        K8S_LOGS_DIR,
        &[namespace, name, uid].join(LOG_PATH_DELIMITER),
    ]
    .join("/")
    .into()
}

const CONTAINER_EXCLUSION_ANNOTATION_KEY: &str = "vertex.io/exclude-containers";

fn extract_excluded_containers_for_pod(pod: &Pod) -> impl Iterator<Item = &str> {
    let meta = &pod.metadata;

    let mut containers = vec![];
    for (key, value) in &meta.annotations {
        if key != CONTAINER_EXCLUSION_ANNOTATION_KEY {
            continue;
        }

        containers.extend(value.split(',').map(|container| container.trim()));
    }

    containers.into_iter()
}

#[cfg(test)]
mod tests {
    use super::super::pod::{PodSpec, PodStatus};
    use super::*;

    use kubernetes::ObjectMeta;

    #[test]
    fn test_extract_pod_logs_directory() {
        let cases = vec![
            // Empty pod
            (Pod::default(), None),
            // Happy path
            (
                Pod {
                    metadata: ObjectMeta {
                        namespace: "test-ns".to_owned(),
                        name: "test-name".to_owned(),
                        uid: "test-uid".to_owned(),
                        ..Default::default()
                    },
                    spec: PodSpec::default(),
                    status: PodStatus::default(),
                },
                Some("/var/log/pods/test-ns_test-name_test-uid"),
            ),
            // No uid
            (
                Pod {
                    metadata: ObjectMeta {
                        name: "name".to_owned(),
                        namespace: "ns".to_owned(),
                        ..Default::default()
                    },
                    spec: PodSpec::default(),
                    status: PodStatus::default(),
                },
                None,
            ),
            // No name
            (
                Pod {
                    metadata: ObjectMeta {
                        namespace: "ns".to_owned(),
                        uid: "uid".to_owned(),
                        ..Default::default()
                    },
                    ..Pod::default()
                },
                None,
            ),
            // No namespace
            (
                Pod {
                    metadata: ObjectMeta {
                        name: "name".to_owned(),
                        uid: "uid".to_owned(),
                        ..ObjectMeta::default()
                    },
                    ..Pod::default()
                },
                None,
            ),
            // Static pod config hashsum as uid
            (
                Pod {
                    metadata: ObjectMeta {
                        namespace: "ns".to_owned(),
                        name: "name".to_owned(),
                        uid: "uid".to_owned(),
                        annotations: vec![(
                            "kubernetes.io/config.mirror".to_owned(),
                            "config-hashsum".to_owned(),
                        )]
                        .into_iter()
                        .collect(),
                        ..ObjectMeta::default()
                    },
                    ..Pod::default()
                },
                Some("/var/log/pods/ns_name_config-hashsum"),
            ),
        ];

        for (input, want) in cases {
            assert_eq!(
                extract_pod_logs_directory(&input),
                want.map(PathBuf::from),
                "{:#?}",
                input
            );
        }
    }

    #[test]
    fn test_extract_excluded_containers_for_pod() {
        let cases = vec![
            // No annotations
            (Pod::default(), vec![]),
            // Empty annotations
            (
                Pod {
                    metadata: ObjectMeta {
                        annotations: vec![].into_iter().collect(),
                        ..ObjectMeta::default()
                    },
                    ..Pod::default()
                },
                vec![],
            ),
            // Irrelevant annotation
            (
                Pod {
                    metadata: ObjectMeta {
                        annotations: vec![("some-other".to_owned(), "some_value".to_owned())]
                            .into_iter()
                            .collect(),

                        ..ObjectMeta::default()
                    },
                    ..Pod::default()
                },
                vec![],
            ),
            // Proper annotation without space
            (
                Pod {
                    metadata: ObjectMeta {
                        annotations: vec![(
                            CONTAINER_EXCLUSION_ANNOTATION_KEY.to_owned(),
                            "container1,container4".to_owned(),
                        )]
                        .into_iter()
                        .collect(),

                        ..ObjectMeta::default()
                    },
                    ..Pod::default()
                },
                vec!["container1", "container4"],
            ),
            // Proper annotation with space
            (
                Pod {
                    metadata: ObjectMeta {
                        annotations: vec![(
                            CONTAINER_EXCLUSION_ANNOTATION_KEY.to_owned(),
                            " container1, container4 ".to_owned(),
                        )]
                        .into_iter()
                        .collect(),

                        ..ObjectMeta::default()
                    },
                    ..Pod::default()
                },
                vec!["container1", "container4"],
            ),
        ];

        for (input, want) in cases {
            let got: Vec<&str> = extract_excluded_containers_for_pod(&input).collect();
            assert_eq!(got, want);
        }
    }

    #[test]
    fn test_list_pod_log_paths() {
        let cases = vec![
            // Empty pod
            (Pod::default(), vec![], vec![]),
            // Pod exists and has some containers that write logs, and some of
            // the containers are excluded.
            (
                Pod {
                    metadata: ObjectMeta {
                        namespace: "ns".to_owned(),
                        name: "name".to_owned(),
                        uid: "uid".to_owned(),
                        annotations: vec![(
                            CONTAINER_EXCLUSION_ANNOTATION_KEY.to_owned(),
                            "excluded1,excluded2".to_owned(),
                        )]
                        .into_iter()
                        .collect(),
                        ..ObjectMeta::default()
                    },
                    ..Pod::default()
                },
                // Calls to the glob mock
                vec![(
                    // The pattern to expect at the mock
                    "/var/log/pods/ns_name_uid/*/*.log*",
                    // The paths to return from the mock
                    vec![
                        "/var/log/pods/ns_name_uid/container1/qwe.log",
                        "/var/log/pods/ns_name_uid/container2/qwe.log",
                        "/var/log/pods/ns_name_uid/excluded1/qwe.log",
                        "/var/log/pods/ns_name_uid/container3/qwe.log",
                        "/var/log/pods/ns_name_uid/excluded2/qwe.log",
                    ],
                )],
                // Expected result
                vec![
                    "/var/log/pods/ns_name_uid/container1/qwe.log",
                    "/var/log/pods/ns_name_uid/container2/qwe.log",
                    "/var/log/pods/ns_name_uid/container3/qwe.log",
                ],
            ),
            // Pod has proper metadata, but doesn't have log files
            (
                Pod {
                    metadata: ObjectMeta {
                        namespace: "ns".to_owned(),
                        name: "name".to_owned(),
                        uid: "uid".to_owned(),
                        ..ObjectMeta::default()
                    },
                    ..Pod::default()
                },
                vec![("/var/log/pods/ns_name_uid/*/*.log*", vec![])],
                vec![],
            ),
        ];

        for (input, want_calls, want_paths) in cases {
            // Prepare the mock fn
            let mut want_calls = want_calls.into_iter();
            let mock_glob = move |pattern: &str| {
                let (want_pattern, paths_to_return) = want_calls
                    .next()
                    .expect("implementation did a call that wasn't expected");

                assert_eq!(pattern, want_pattern);
                paths_to_return.into_iter().map(PathBuf::from)
            };

            let got_paths = list_pod_log_paths(&input, mock_glob).collect::<Vec<_>>();
            let want_paths = want_paths
                .into_iter()
                .map(PathBuf::from)
                .collect::<Vec<_>>();
            assert_eq!(got_paths, want_paths);
        }
    }

    #[test]
    fn test_exclude_paths() {
        let cases = vec![
            // No exclusion pattern allows everything
            (
                vec![
                    "/var/log/pods/a.log",
                    "/var/log/pods/b.log",
                    "/var/log/pods/c.log.foo",
                    "/var/log/pods/d.logbar",
                ],
                vec![],
                vec![
                    "/var/log/pods/a.log",
                    "/var/log/pods/b.log",
                    "/var/log/pods/c.log.foo",
                    "/var/log/pods/d.logbar",
                ],
            ),
            // A filter that doesn't apply to anything
            (
                vec![
                    "/var/log/pods/a.log",
                    "/var/log/pods/b.log",
                    "/var/log/pods/c.log",
                ],
                vec!["notmatched"],
                vec![
                    "/var/log/pods/a.log",
                    "/var/log/pods/b.log",
                    "/var/log/pods/c.log",
                ],
            ),
            // Multiple filters
            (
                vec![
                    "/var/log/pods/a.log",
                    "/var/log/pods/b.log",
                    "/var/log/pods/c.log",
                ],
                vec!["notmatched", "**/b.log", "**/c.log"],
                vec!["/var/log/pods/a.log"],
            ),
            // Requires literal path separator (* does not include dirs).
            (
                vec![
                    "/var/log/pods/a.log",
                    "/var/log/pods/b.log",
                    "/var/log/pods/c.log",
                ],
                vec!["*/b.log", "**/c.log"],
                vec!["/var/log/pods/a.log", "/var/log/pods/b.log"],
            ),
            // Filtering by container name with a real-life-like file pattern
            (
                vec![
                    "/var/log/pods/ns_name_uid/container1/1.log",
                    "/var/log/pods/ns_name_uid/container1/2.log",
                    "/var/log/pods/ns_name_uid/container2/1.log",
                ],
                vec!["**/container1/**"],
                vec!["/var/log/pods/ns_name_uid/container2/1.log"],
            ),
        ];

        for (input_paths, str_patterns, want_paths) in cases {
            let patterns = str_patterns
                .iter()
                .map(|pattern| glob::Pattern::new(pattern).unwrap())
                .collect::<Vec<_>>();

            let got_paths = exclude_paths(input_paths.into_iter().map(Into::into), &patterns)
                .collect::<Vec<_>>();

            let want_paths = want_paths
                .into_iter()
                .map(PathBuf::from)
                .collect::<Vec<_>>();

            assert_eq!(
                got_paths, want_paths,
                "failed for patterns {:?}",
                &str_patterns
            );
        }
    }

    #[test]
    fn test_build_container_exclusion_patterns() {
        let cases = vec![
            // No excluded containers - no exclusion patterns
            ("/var/log/pods/ns_name_uid", vec![], vec![]),
            // Ensure the paths are concatenated correctly and look good
            (
                "/var/log/pods/ns_name_uid",
                vec!["container1", "container2"],
                vec![
                    "/var/log/pods/ns_name_uid/container1/**",
                    "/var/log/pods/ns_name_uid/container2/**",
                ],
            ),
            // Ensure control characters are escaped properly
            (
                "/var/log/pods/ns_name_uid",
                vec!["*[]"],
                vec!["/var/log/pods/ns_name_uid/[*][[][]]/**"],
            ),
        ];

        for (pod_logs_dir, containers, want_patterns) in cases {
            let got =
                build_container_exclusion_patterns(pod_logs_dir, containers.clone().into_iter())
                    .collect::<Vec<_>>();
            let want = want_patterns
                .into_iter()
                .map(|pattern| glob::Pattern::new(pattern).unwrap())
                .collect::<Vec<_>>();

            assert_eq!(
                got, want,
                "failed for dir {:?} and container {:?}",
                &pod_logs_dir, &containers
            );
        }
    }
}
