use kubernetes::{ObjectMeta, Resource};
use serde::Deserialize;

/// A single application container that you want to run within a pod.
#[cfg_attr(test, derive(Default))]
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
}

/// PodSpec implements k8s pod spec.
///
/// See https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.31/#podspec-v1-core
#[cfg_attr(test, derive(Default))]
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
}

/// PodIp represents a single IP address allocated to the pod
#[derive(Debug, Deserialize)]
pub struct PodIP {
    /// IP is the ip address assigned to the pod
    pub ip: String,
}

/// ContainerStatus contains details for the current status of this container.
#[cfg_attr(test, derive(Default))]
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
}

/// PodStatus represents information about the status of a pod. Status may trail
/// the actual state of a system, especially if the node that hosts the pod cannot
/// contact the control plane.
#[cfg_attr(test, derive(Default))]
#[derive(Debug, Deserialize)]
pub struct PodStatus {
    /// IP address allocated to the pod. Routable at least within the cluster. Empty if not
    /// yet allocated.
    #[serde(rename = "podIP")]
    pub pod_ip: Option<String>,

    /// PodIPs holds the IP addresses allocated to the pod. If this field is specified, the
    /// 0th entry must match the podIP field. Pods may be allocated at most 1 value for each
    /// IPv4 and IPv6. This list is empty if no IPs have been allocated yet.
    #[serde(default, rename = "podIPs")]
    pub pod_ips: Vec<PodIP>,

    /// The list has one entry per container in the manifest.
    ///
    /// More info: https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle#pod-and-container-status
    #[serde(rename = "containerStatuses")]
    pub container_statuses: Vec<ContainerStatus>,
}

/// Pod is a collection of containers that can run on a host. This resource
/// is created by clients and scheduled onto hosts.
#[cfg_attr(test, derive(Default))]
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
    }

    #[test]
    fn url() {
        assert_eq!(Pod::url_path(None), "/api/v1/pods");
        assert_eq!(Pod::url_path(Some("foo")), "/api/v1/namespaces/foo/pods");
    }
}
