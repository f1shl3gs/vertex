mod kubelet;
mod kubernetes;
mod pod;

use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::Duration;

use configurable::Configurable;
use kubelet::KubeletProvider;
use kubernetes::KubernetesProvider;
use pod::{Container, Pod};
use serde::{Deserialize, Serialize};
use tail::{CheckpointsView, Fingerprint, Provider};

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
    pub fn build(&self, checkpoints: CheckpointsView) -> Result<KubeProvider, crate::Error> {
        let inner = match self {
            Self::Kubelet { endpoint, interval } => {
                let provider = KubeletProvider::new(endpoint.as_ref(), *interval)?;
                Inner::Kubelet(provider)
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

                KubernetesProvider::new(label_selector.clone(), Some(field_selector))
                    .map(Inner::Kubernetes)?
            }
        };

        Ok(KubeProvider { inner, checkpoints })
    }
}

#[derive(Debug)]
pub struct Metadata {
    pub pod: Pod,
    pub container: Container,
}

#[allow(clippy::large_enum_variant)]
enum Inner {
    Kubelet(KubeletProvider),
    Kubernetes(KubernetesProvider),
}

pub struct KubeProvider {
    inner: Inner,
    checkpoints: CheckpointsView,
}

impl Provider for KubeProvider {
    type Metadata = Metadata;

    async fn scan(&mut self) -> std::io::Result<Vec<(PathBuf, Self::Metadata)>> {
        loop {
            let result: std::io::Result<Vec<Pod>> = match &mut self.inner {
                Inner::Kubelet(provider) => provider.list().await,
                Inner::Kubernetes(provider) => provider.list().await,
            };

            let Ok(pods) = result else { continue };

            let mut paths = Vec::with_capacity(pods.len() * 2);
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
                            let candidates = dirs
                                .flatten()
                                .map(|entry| entry.path())
                                .filter(|path| {
                                    let Some(ext) = path.extension() else {
                                        return false;
                                    };

                                    ext == "log"
                                })
                                .collect::<Vec<_>>();

                            if candidates.is_empty() {
                                continue;
                            }

                            paths.push((
                                sort_and_select(candidates, &self.checkpoints),
                                Metadata {
                                    pod: pod.clone(),
                                    container: container.clone(),
                                },
                            ));
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

            return Ok(paths);
        }
    }
}

fn sort_and_select(mut paths: Vec<PathBuf>, checkpoints: &CheckpointsView) -> PathBuf {
    paths.sort_by(|a, b| {
        let a = a.file_name().unwrap();
        let b = b.file_name().unwrap();
        b.cmp(a)
    });

    let mut previous = PathBuf::new();
    for (index, path) in paths.into_iter().enumerate() {
        let Ok(stat) = path.metadata() else { continue };

        let fingerprint = Fingerprint::from(&stat);
        match checkpoints.get(&fingerprint) {
            Some(offset) => {
                if offset.load(Ordering::Acquire) != stat.size() {
                    // there are some data to catch up
                    return path;
                }

                return if index == 0 {
                    // latest file
                    path
                } else {
                    // previous unconsumed file
                    previous
                };
            }
            None => {
                previous = path;
            }
        }
    }

    previous
}

#[cfg(test)]
mod tests {
    #![allow(warnings)]

    use std::io::Write;
    use std::sync::atomic::Ordering;

    use super::*;
    use tail::Checkpointer;

    #[test]
    fn select() {
        let root = testify::temp_dir();
        let mut paths = Vec::new();
        for i in 0..5 {
            let path = root.join(format!("{i}.log"));
            let mut file = std::fs::File::create(&path).unwrap();
            file.write_all(b"hello").unwrap();

            paths.push(path);
        }

        let f0 = paths[0].clone();
        let f1 = paths[1].clone();
        let f2 = paths[2].clone();
        let f3 = paths[3].clone();
        let f4 = paths[4].clone();

        let checkpointer = Checkpointer::load(root.clone()).unwrap();
        let checkpoints = checkpointer.view();

        let got = sort_and_select(paths.clone(), &checkpoints);
        assert_eq!(got, root.join("0.log"));

        let o0 = checkpointer.insert((&f0.metadata().unwrap()).into(), 0);
        o0.fetch_add(2, Ordering::Relaxed);
        let got = sort_and_select(paths.clone(), &checkpoints);
        assert_eq!(got, root.join("0.log"));

        o0.fetch_add(2, Ordering::Relaxed);
        let got = sort_and_select(paths.clone(), &checkpoints);
        assert_eq!(got, root.join("0.log"));

        o0.fetch_add(1, Ordering::Relaxed);
        let got = sort_and_select(paths.clone(), &checkpoints);
        assert_eq!(got, root.join("1.log"));

        let o1 = checkpointer.insert((&f1.metadata().unwrap()).into(), 0);
        let got = sort_and_select(paths.clone(), &checkpoints);
        assert_eq!(got, root.join("1.log"));

        o1.fetch_add(5, Ordering::Relaxed);
        let got = sort_and_select(paths.clone(), &checkpoints);
        assert_eq!(got, root.join("2.log"));

        let o2 = checkpointer.insert((&f2.metadata().unwrap()).into(), 0);
        let got = sort_and_select(paths.clone(), &checkpoints);
        assert_eq!(got, root.join("2.log"));

        o2.fetch_add(2, Ordering::Relaxed);
        let got = sort_and_select(paths.clone(), &checkpoints);
        assert_eq!(got, root.join("2.log"));

        o2.fetch_add(2, Ordering::Relaxed);
        let got = sort_and_select(paths.clone(), &checkpoints);
        assert_eq!(got, root.join("2.log"));

        o2.fetch_add(1, Ordering::Relaxed);
        let got = sort_and_select(paths.clone(), &checkpoints);
        assert_eq!(got, root.join("3.log"));

        let o3 = checkpointer.insert((&f3.metadata().unwrap()).into(), 0);
        let got = sort_and_select(paths.clone(), &checkpoints);
        assert_eq!(got, root.join("3.log"));

        o3.fetch_add(5, Ordering::Relaxed);
        let got = sort_and_select(paths.clone(), &checkpoints);
        assert_eq!(got, root.join("4.log"));

        let o4 = checkpointer.insert((&f4.metadata().unwrap()).into(), 2);
        let got = sort_and_select(paths.clone(), &checkpoints);
        assert_eq!(got, root.join("4.log"));

        o4.fetch_add(3, Ordering::Relaxed);
        let got = sort_and_select(paths.clone(), &checkpoints);
        assert_eq!(got, root.join("4.log"));
    }
}
