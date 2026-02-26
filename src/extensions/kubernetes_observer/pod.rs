use std::collections::BTreeMap;

use framework::observe::Endpoint;
use kubernetes::{ObjectMeta, Resource};
use serde::Deserialize;
use value::value;

use super::Keyed;

fn default_protocol() -> String {
    String::from("TCP")
}

/// containerPort represents a network port in a single container.
#[derive(Debug, Deserialize)]
pub struct ContainerPort {
    /// If specified, this must be an IANA_SVC_NAME and unique within the pod.
    /// Each named port in a pod must have a unique name. Name for the port
    /// that can be referred to by services.
    pub name: Option<String>,

    /// Number of port to expose on the pod's IP address. This mut be a valid port
    /// number, 0 < x < 65536.
    #[serde(rename = "containerPort")]
    pub container_port: i32,

    /// Protocol for port. Must be UDP, TCP, or SCTP. Defaults to "TCP".
    #[serde(default = "default_protocol")]
    pub protocol: String,
}

/// A single application container that you want to run within a pod.
#[derive(Debug, Deserialize)]
pub struct Container {
    /// Name of the container specified as a DNS_LABEL. Each container in a pod
    /// must have a unique name (DNS_LABEL). Cannot be updated.
    pub name: String,

    /// Container image name. This field is optional to allow higher level
    /// config management to default or override container images in workload
    /// controllers like Deployments and StatefulSets.
    ///
    /// More info: https://kubernetes.io/docs/concepts/containers/images
    pub image: String,

    /// List of ports to expose from the container. Not specifying a port here
    /// DOES NOT prevent that port from being exposed. Any port which is listening
    /// on the default "0.0.0.0" address inside a container will be accessible
    /// from the network. Modifying this array with strategic merge patch may
    /// corrupt the data. Cannot be updated.
    ///
    /// For more information https://github.com/kubernetes/kubernetes/issues/108255.
    #[serde(default)]
    pub ports: Vec<ContainerPort>,
}

/// PodSpec implements k8s pod spec.
///
/// See https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.31/#podspec-v1-core
#[derive(Debug, Deserialize)]
pub struct PodSpec {
    /// NodeName is a request to schedule this pod onto a specific node. If it
    /// is non-empty, the scheduler simply schedules this pod onto that node,
    /// assuming that it fits resource requirements.
    #[serde(rename = "nodeName")]
    pub node_name: String,

    /// List of containers belonging to the pod. Containers cannot currently be
    /// added or removed. There must be at least one container in a Pod. Cannot
    /// be updated.
    pub containers: Vec<Container>,

    /// List of initialization containers belonging to the pod. Init containers
    /// are executed in order prior to containers being started. If any init
    /// container fails, the pod is considered to have failed and is handled
    /// according to its restartPolicy. The name for an init container or normal
    /// container must be unique among all containers. Init containers may not
    /// have Lifecycle actions, Liveness probes, or Startup probes. The
    /// resourceRequirements of an init container are taken into account during
    /// scheduling by finding the highest request/limit for each resource type,
    /// and then using the max of that value or the sum of the normal containers.
    /// Limits are applied to init containers in a similar fashion. Init containers
    /// cannot currently be added or removed. Cannot be updated.
    ///
    /// More info: https://kubernetes.io/docs/concepts/workloads/pods/init-containers/
    #[serde(default, rename = "initContainers")]
    pub init_containers: Vec<Container>,
}

/// PodCondition contains details for the current condition of this pod.
#[derive(Debug, Deserialize)]
pub struct PodCondition {
    /// Type is the type of the condition.
    ///
    /// More info: https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle#pod-conditions
    #[serde(rename = "type")]
    pub typ: String,

    /// Status is the status of the condition. Can be True, False, Unknown.
    ///
    /// More info: https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle#pod-conditions
    pub status: String,
}

/// ContainerStateTerminated is a terminated state of a container.
#[derive(Debug, Deserialize)]
pub struct ContainerStateTerminated {
    /// Exit status from the last termination of the container.
    #[serde(rename = "exitCode")]
    pub exit_code: i32,
}

/// ContainerState holds a possible state of container. Only one of its members
/// may be specified. If none of them is specified, the default one is
/// ContainerStateWaiting.
#[derive(Debug, Deserialize)]
pub struct ContainerState {
    pub terminated: Option<ContainerStateTerminated>,
}

/// ContainerStatus contains details for the current status of this container.
#[derive(Debug, Deserialize)]
pub struct ContainerStatus {
    /// Name is a DNS_LABEL representing the unique name of the container. Each container
    /// in a pod must have a unique name across all container types. Cannot be updated.
    pub name: String,

    /// ContainerID is the ID of the container in the format '<type>://<container_id>'. Where
    /// types is a container runtime identifier, returned from Version call of CRI API
    /// (for example "containerd").
    #[serde(rename = "containerID")]
    pub container_id: Option<String>,

    /// State holds details about the container's current condition.
    pub state: ContainerState,
}

/// PodStatus represents information about the status of a pod. Status may trail
/// the actual state of a system, especially if the node that hosts the pod cannot
/// contact the control plane.
#[derive(Debug, Deserialize)]
pub struct PodStatus {
    /// The phase of a Pod is a simple, high-level summary of where the Pod is in its
    /// lifecycle. The conditions array, the reason and message fields, and the
    /// individual container status arrays contains more detail about the pod's
    /// status. There are five possible phase values:
    ///
    /// - Pending: The pod has been accepted by the Kubernetes system, but one or more
    ///   of the container images has not been created. This includes time before being
    ///   scheduled as well as time spent downloading images over the network, which
    ///   could take a while.
    /// - Running: The pod has been bound to a node, and all of the containers have
    ///   been created. At least one container is still running, or is in the process of
    ///   starting or restarting.
    /// - Succeeded: All containers in the pod have terminated in success, and will not
    ///   be restarted.
    /// - Failed: All containers in the pod have terminated, and at least one container
    ///   has terminated in failure. The container either exited with non-zero status or
    ///   was terminated by the system.
    /// - Unknown: For some reason the state of the pod could not be obtained, typically
    ///   due to an error in communicating with the host of the pod.
    pub phase: String,

    /// IP address allocated to the pod. Routable at least within the cluster. Empty if not
    /// yet allocated.
    #[serde(rename = "podIP")]
    pub pod_ip: Option<String>,

    /// Ip address of the host to which the pod is assigned. Empty if not yet scheduled.
    #[serde(rename = "hostIP")]
    pub host_ip: Option<String>,

    /// Current service state of pod.
    ///
    /// More info: https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle#pod-conditions
    #[serde(default)]
    pub conditions: Vec<PodCondition>,

    /// The list has one entry per container in the manifest.
    ///
    /// More info: https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle#pod-and-container-status
    #[serde(rename = "containerStatuses")]
    pub container_statuses: Vec<ContainerStatus>,

    /// The list has one entry per init container in the manifest. The most recent successful
    /// init container will have ready = true, the most recently started container will have
    /// startTime set.
    ///
    /// More info: https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle#pod-and-container-status
    #[serde(rename = "initContainerStatuses")]
    pub init_container_statuses: Option<Vec<ContainerStatus>>,
}

/// Pod is a collection of containers that can run on a host. This resource
/// is created by clients and scheduled onto hosts.
#[derive(Debug, Deserialize)]
pub struct Pod {
    /// Standard object's metadata.
    ///
    /// More info: https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#metadata
    pub metadata: ObjectMeta,

    /// Specification of the desired behavior of the pod.
    ///
    /// More info: https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#spec-and-status
    pub spec: PodSpec,

    /// Most recently observed status of the pod. This data may not be up to date. Populated
    /// by the system. Read-only.
    ///
    /// More info: https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#spec-and-status
    pub status: PodStatus,
}

impl Resource for Pod {
    const GROUP: &'static str = "";
    const VERSION: &'static str = "v1";
    const KIND: &'static str = "Pod";
    const PLURAL: &'static str = "pods";
}

impl Keyed for Pod {
    fn key(&self) -> &str {
        self.metadata.uid.as_ref()
    }
}

impl From<Pod> for Vec<Endpoint> {
    fn from(pod: Pod) -> Self {
        let Some(pod_ip) = pod.status.pod_ip else {
            return vec![];
        };

        let mut labels = BTreeMap::new();
        for (k, v) in pod.metadata.labels {
            labels.insert(k, v.into());
        }

        let mut annotations = BTreeMap::new();
        for (k, v) in pod.metadata.annotations {
            annotations.insert(k, v.into());
        }

        let pod_info = {
            let phase = pod.status.phase;
            let node_name = pod.spec.node_name;
            let host_ip = pod.status.host_ip.unwrap_or_default();
            let ready = if pod
                .status
                .conditions
                .iter()
                .any(|condition| condition.typ == "Ready")
            {
                "ready"
            } else {
                "unknown"
            };

            value!({
                "name": pod.metadata.name.clone(),
                "namespace": pod.metadata.namespace.clone(),
                "ip": pod_ip.clone(),
                "ready": ready,
                "phase": phase,
                "node_name": node_name,
                "host_ip": host_ip,
                "uid": pod.metadata.uid,
            })
        };

        let mut endpoints =
            Vec::with_capacity(pod.spec.containers.len() + pod.spec.init_containers.len());

        pod.spec
            .containers
            .iter()
            .chain(pod.spec.init_containers.iter())
            .enumerate()
            // skip the terminated container
            .filter(|(_index, container)| {
                pod.status.container_statuses.iter().any(|status| {
                    status.name == container.name && status.state.terminated.is_none()
                })
            })
            .for_each(|(index, container)| {
                let init_container = index < pod.spec.containers.len() - 1;
                let image = container.image.clone();
                let name = container.name.clone();

                if container.ports.is_empty() {
                    let id = format!(
                        "{}/{}/containers/{}",
                        pod.metadata.namespace, pod.metadata.name, container.name
                    );

                    endpoints.push(Endpoint {
                        id,
                        typ: "pod".into(),
                        target: pod_ip.clone(),
                        details: value!({
                            "name": name,
                            "namespace": pod.metadata.namespace.clone(),
                            "labels": labels.clone(),
                            "annotations": annotations.clone(),

                            "image": image,
                            "init_container": init_container,
                            "pod": pod_info.clone(),
                        }),
                    });

                    return;
                }

                for port in &container.ports {
                    let id = format!(
                        "{}/{}/containers/{}/{}:{}",
                        pod.metadata.namespace,
                        pod.metadata.name,
                        container.name,
                        port.protocol,
                        port.container_port
                    );

                    endpoints.push(Endpoint {
                        id,
                        target: format!("{}:{}", pod_ip, port.container_port),
                        typ: "pod".into(),
                        details: value!({
                            "name": name.clone(),
                            "namespace": pod.metadata.namespace.clone(),
                            "labels": labels.clone(),
                            "annotations": annotations.clone(),

                            "image": image.clone(),
                            "init_container": init_container,
                            "pod": pod_info.clone(),
                        }),
                    })
                }
            });

        endpoints
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kubernetes::ObjectList;

    #[test]
    fn deserialize() {
        const INPUT: &str = r#"
{
   "kind":"PodList",
   "apiVersion":"v1",
   "metadata":{
      "selfLink":"/api/v1/pods",
      "resourceVersion":"72425"
   },
   "items":[
      {
         "metadata":{
            "name":"etcd-m01",
            "namespace":"kube-system",
            "selfLink":"/api/v1/namespaces/kube-system/pods/etcd-m01",
            "uid":"9d328156-75d1-411a-bdd0-aeacb53a38de",
            "resourceVersion":"22318",
            "creationTimestamp":"2020-03-16T20:44:30Z",
            "labels":{
               "component":"etcd",
               "tier":"control-plane"
            },
            "annotations":{
               "kubernetes.io/config.hash":"3ec997b76fb6ed3b78da8e0b5676dac4",
               "kubernetes.io/config.mirror":"3ec997b76fb6ed3b78da8e0b5676dac4",
               "kubernetes.io/config.seen":"2020-03-16T20:44:26.538136233Z",
               "kubernetes.io/config.source":"file"
            },
            "ownerReferences":[
               {
                  "apiVersion":"v1",
                  "kind":"Node",
                  "name":"m01",
                  "uid":"b48dd901-ead0-476a-b209-d2d908d65109",
                  "controller":true
               }
            ]
         },
         "spec":{
            "volumes":[
               {
                  "name":"etcd-certs",
                  "hostPath":{
                     "path":"/var/lib/minikube/certs/etcd",
                     "type":"DirectoryOrCreate"
                  }
               },
               {
                  "name":"etcd-data",
                  "hostPath":{
                     "path":"/var/lib/minikube/etcd",
                     "type":"DirectoryOrCreate"
                  }
               }
            ],
            "containers":[
               {
                  "name":"terminated-container",
                  "image":"terminated-image",
                  "ports":[
                     {
                        "name":"terminated-port",
                        "containerPort":4321,
                        "protocol":"TCP"
                     }
                  ]
               },
               {
                  "name":"etcd",
                  "image":"k8s.gcr.io/etcd:3.4.3-0",
                  "command":[
                     "etcd",
                     "--advertise-client-urls=https://172.17.0.2:2379",
                     "--cert-file=/var/lib/minikube/certs/etcd/server.crt",
                     "--client-cert-auth=true",
                     "--data-dir=/var/lib/minikube/etcd",
                     "--initial-advertise-peer-urls=https://172.17.0.2:2380",
                     "--initial-cluster=m01=https://172.17.0.2:2380",
                     "--key-file=/var/lib/minikube/certs/etcd/server.key",
                     "--listen-client-urls=https://127.0.0.1:2379,https://172.17.0.2:2379",
                     "--listen-metrics-urls=http://127.0.0.1:2381",
                     "--listen-peer-urls=https://172.17.0.2:2380",
                     "--name=m01",
                     "--peer-cert-file=/var/lib/minikube/certs/etcd/peer.crt",
                     "--peer-client-cert-auth=true",
                     "--peer-key-file=/var/lib/minikube/certs/etcd/peer.key",
                     "--peer-trusted-ca-file=/var/lib/minikube/certs/etcd/ca.crt",
                     "--snapshot-count=10000",
                     "--trusted-ca-file=/var/lib/minikube/certs/etcd/ca.crt"
                  ],
                  "resources":{

                  },
                  "ports":[
                     {
                        "name":"foobar",
                        "containerPort":1234,
                        "protocol":"TCP"
                     }
                  ],
                  "volumeMounts":[
                     {
                        "name":"etcd-data",
                        "mountPath":"/var/lib/minikube/etcd"
                     },
                     {
                        "name":"etcd-certs",
                        "mountPath":"/var/lib/minikube/certs/etcd"
                     }
                  ],
                  "livenessProbe":{
                     "httpGet":{
                        "path":"/health",
                        "port":2381,
                        "host":"127.0.0.1",
                        "scheme":"HTTP"
                     },
                     "initialDelaySeconds":15,
                     "timeoutSeconds":15,
                     "periodSeconds":10,
                     "successThreshold":1,
                     "failureThreshold":8
                  },
                  "terminationMessagePath":"/dev/termination-log",
                  "terminationMessagePolicy":"File",
                  "imagePullPolicy":"IfNotPresent"
               }
            ],
            "restartPolicy":"Always",
            "terminationGracePeriodSeconds":30,
            "dnsPolicy":"ClusterFirst",
            "nodeName":"test-node",
            "hostNetwork":true,
            "securityContext":{

            },
            "schedulerName":"default-scheduler",
            "tolerations":[
               {
                  "operator":"Exists",
                  "effect":"NoExecute"
               }
            ],
            "priorityClassName":"system-cluster-critical",
            "priority":2000000000,
            "enableServiceLinks":true
         },
         "status":{
            "phase":"Running",
            "conditions":[
               {
                  "type":"Initialized",
                  "status":"True",
                  "lastProbeTime":null,
                  "lastTransitionTime":"2020-03-20T13:30:29Z"
               },
               {
                  "type":"Ready",
                  "status":"True",
                  "lastProbeTime":null,
                  "lastTransitionTime":"2020-03-20T13:30:32Z"
               },
               {
                  "type":"ContainersReady",
                  "status":"True",
                  "lastProbeTime":null,
                  "lastTransitionTime":"2020-03-20T13:30:32Z"
               },
               {
                  "type":"PodScheduled",
                  "status":"True",
                  "lastProbeTime":null,
                  "lastTransitionTime":"2020-03-20T13:30:29Z"
               }
            ],
            "hostIP":"172.17.0.2",
            "podIP":"172.17.0.2",
            "podIPs":[
               {
                  "ip":"172.17.0.2"
               }
            ],
            "startTime":"2020-03-20T13:30:29Z",
            "containerStatuses":[
               {
                  "name":"terminated-container",
                  "state":{
                     "terminated":{
                        "exitCode":432
                     }
                  },
                  "containerID":"terminated-container-id"
               },
               {
                  "name":"etcd",
                  "state":{
                     "running":{
                        "startedAt":"2020-03-20T13:30:30Z"
                     }
                  },
                  "lastState":{
                     "terminated":{
                        "exitCode":0,
                        "reason":"Completed",
                        "startedAt":"2020-03-17T18:56:24Z",
                        "finishedAt":"2020-03-20T13:29:54Z",
                        "containerID":"docker://24eea6f192d4598fcc129b5f163a02d1457137f4ec34e8c80c6049a65604cb07"
                     }
                  },
                  "ready":true,
                  "restartCount":2,
                  "image":"k8s.gcr.io/etcd:3.4.3-0",
                  "imageID":"docker-pullable://k8s.gcr.io/etcd@sha256:4afb99b4690b418ffc2ceb67e1a17376457e441c1f09ab55447f0aaf992fa646",
                  "containerID":"docker://a28f0800855008485376c1eece1cf61de97cb7026b9188d138b0d55d92fc2f5c",
                  "started":true
               }
            ],
            "qosClass":"BestEffort"
         }
      }
   ]
}
"#;

        let pods = serde_json::from_str::<ObjectList<Pod>>(INPUT).unwrap();
        assert_eq!(pods.metadata.resource_version, Some("72425".into()));
        assert_eq!(pods.items.len(), 1);

        let pod = pods.items.first().unwrap();
        assert_eq!(pod.status.pod_ip.as_ref().unwrap(), "172.17.0.2");

        assert_eq!(pod.metadata.namespace, "kube-system");
        assert_eq!(pod.spec.node_name, "test-node");
        assert_eq!(pod.metadata.name, "etcd-m01");
        assert_eq!(pod.spec.containers[1].image, "k8s.gcr.io/etcd:3.4.3-0");
        assert_eq!(pod.spec.containers[1].name, "etcd");
        assert_eq!(pod.spec.containers[1].ports[0].name, Some("foobar".into()));
        assert_eq!(pod.spec.containers[1].ports[0].container_port, 1234);
        assert_eq!(pod.spec.containers[1].ports[0].protocol, "TCP");
    }

    #[test]
    fn url() {
        assert_eq!(Pod::url_path(None), "/api/v1/pods");
        assert_eq!(Pod::url_path(Some("foo")), "/api/v1/namespaces/foo/pods");
    }
}
