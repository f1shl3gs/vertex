#![allow(dead_code)]

use framework::observe::Endpoint;
use kubernetes::{ObjectMeta, Resource};
use serde::Deserialize;

use super::Keyed;

/// NodeSpec represents NodeSpec from k8s API.
///
/// See https://kubernetes.io/docs/reference/kubernetes-api/cluster-resources/node-v1/#NodeSpec
#[derive(Deserialize)]
pub struct NodeSpec {
    /// ID of the node assigned by the cloud provider in the format: <ProviderName>://<ProviderSpecificNodeID>
    #[serde(rename = "providerID")]
    provider_id: String,
}

/// NodeAddress represents NodeAddress from k8s API.
///
/// See https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.17/#nodeaddress-v1-core
#[derive(Deserialize)]
pub struct NodeAddress {
    /// Node address type, one of Hostname, ExternalIP or InternalIP.
    pub r#type: String,
    /// The node address.
    pub address: String,
}

/// DaemonEndpoint represents DaemonEndpoint from k8s API.
///
/// See https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.17/#daemonendpoint-v1-core
#[derive(Deserialize)]
pub struct DaemonEndpoint {
    /// Port number of the given endpoint.
    pub port: i16,
}

/// NodeDaemonEndpoints represents NodeDaemonEndpoints from k8s API.
///
/// See https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.17/#nodedaemonendpoints-v1-core
#[derive(Deserialize)]
pub struct NodeDaemonEndpoints {
    /// Endpoint on which Kubelet is listening.
    pub kubelet_endpoint: DaemonEndpoint,
}

/// NodeStatus represents NodeStatus from k8s API.
///
/// See https://kubernetes.io/docs/reference/kubernetes-api/cluster-resources/node-v1/#NodeStatus
#[derive(Deserialize)]
pub struct NodeStatus {
    /// List of addresses reachable to the node. Queried from cloud provider, if available.
    ///
    /// More info: https://kubernetes.io/docs/concepts/nodes/node/#addresses
    ///
    /// Note: This field is declared as mergeable, but the merge key is not sufficiently unique,
    /// which can cause data corruption when it is merged. Callers should instead use a
    /// full-replacement patch. See https://pr.k8s.io/79391 for an example. Consumers should
    /// assume that addresses can change during the lifetime of a Node. However, there are some
    /// exceptions where this may not be possible, such as Pods that inherit a Node's address in
    /// its own status or consumers of the downward API (status.hostIP).
    #[serde(default)]
    addresses: Vec<NodeAddress>,

    /// Endpoints of daemons running on the Node.
    daemon_endpoints: Option<NodeDaemonEndpoints>,
}

/// Node represents Node from k8s API.
///
/// See https://kubernetes.io/docs/reference/kubernetes-api/cluster-resources/node-v1/
#[derive(Deserialize)]
pub struct Node {
    /// Standard object's metadata.
    ///
    /// More info: https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#metadata
    pub metadata: ObjectMeta,

    /// Spec defines the behavior of a node.
    ///
    /// https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#spec-and-status
    pub spec: NodeSpec,

    /// Most recently observed status of the node. Populated by the system. Read-only.
    ///
    /// More info: https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#spec-and-status
    pub status: NodeStatus,
}

impl Resource for Node {
    const GROUP: &'static str = "";
    const VERSION: &'static str = "v1";
    const KIND: &'static str = "Node";
    const PLURAL: &'static str = "nodes";

    fn url_path(_namespace: Option<&str>) -> String {
        format!("/api/{}/nodes", Self::VERSION)
    }
}

impl Keyed for Node {
    fn key(&self) -> &str {
        self.metadata.uid.as_ref()
    }
}

impl From<Node> for Vec<Endpoint> {
    fn from(_node: Node) -> Self {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kubernetes::ObjectList;

    #[test]
    fn deserialize() {
        let input = r#"{
  "kind": "NodeList",
  "apiVersion": "v1",
  "metadata": {
    "selfLink": "/api/v1/nodes",
    "resourceVersion": "22627"
  },
  "items": [
    {
      "metadata": {
        "name": "m01",
        "selfLink": "/api/v1/nodes/m01",
        "uid": "b48dd901-ead0-476a-b209-d2d908d65109",
        "resourceVersion": "22309",
        "creationTimestamp": "2020-03-16T20:44:23Z",
        "labels": {
          "beta.kubernetes.io/arch": "amd64",
          "beta.kubernetes.io/os": "linux",
          "kubernetes.io/arch": "amd64",
          "kubernetes.io/hostname": "m01",
          "kubernetes.io/os": "linux",
          "minikube.k8s.io/commit": "eb13446e786c9ef70cb0a9f85a633194e62396a1",
          "minikube.k8s.io/name": "minikube",
          "minikube.k8s.io/updated_at": "2020_03_16T22_44_27_0700",
          "minikube.k8s.io/version": "v1.8.2",
          "node-role.kubernetes.io/master": ""
        },
        "annotations": {
          "kubeadm.alpha.kubernetes.io/cri-socket": "/var/run/dockershim.sock",
          "node.alpha.kubernetes.io/ttl": "0",
          "volumes.kubernetes.io/controller-managed-attach-detach": "true"
        }
      },
      "spec": {
        "podCIDR": "10.244.0.0/24",
        "podCIDRs": [
          "10.244.0.0/24"
        ],
	"providerID": "aws:///foo-bar/baz"
      },
      "status": {
        "capacity": {
          "cpu": "4",
          "ephemeral-storage": "474705032Ki",
          "hugepages-1Gi": "0",
          "hugepages-2Mi": "0",
          "memory": "16314884Ki",
          "pods": "110"
        },
        "allocatable": {
          "cpu": "4",
          "ephemeral-storage": "437488156767",
          "hugepages-1Gi": "0",
          "hugepages-2Mi": "0",
          "memory": "16212484Ki",
          "pods": "110"
        },
        "conditions": [
          {
            "type": "MemoryPressure",
            "status": "False",
            "lastHeartbeatTime": "2020-03-20T13:30:38Z",
            "lastTransitionTime": "2020-03-16T20:44:18Z",
            "reason": "KubeletHasSufficientMemory",
            "message": "kubelet has sufficient memory available"
          },
          {
            "type": "DiskPressure",
            "status": "False",
            "lastHeartbeatTime": "2020-03-20T13:30:38Z",
            "lastTransitionTime": "2020-03-16T20:44:18Z",
            "reason": "KubeletHasNoDiskPressure",
            "message": "kubelet has no disk pressure"
          },
          {
            "type": "PIDPressure",
            "status": "False",
            "lastHeartbeatTime": "2020-03-20T13:30:38Z",
            "lastTransitionTime": "2020-03-16T20:44:18Z",
            "reason": "KubeletHasSufficientPID",
            "message": "kubelet has sufficient PID available"
          },
          {
            "type": "Ready",
            "status": "True",
            "lastHeartbeatTime": "2020-03-20T13:30:38Z",
            "lastTransitionTime": "2020-03-16T20:44:39Z",
            "reason": "KubeletReady",
            "message": "kubelet is posting ready status"
          }
        ],
        "addresses": [
          {
            "type": "InternalIP",
            "address": "172.17.0.2"
          },
          {
            "type": "Hostname",
            "address": "m01"
          }
        ],
        "daemonEndpoints": {
          "kubeletEndpoint": {
            "Port": 10250
          }
        },
        "nodeInfo": {
          "machineID": "e64aad27e586485b9a9cbd699840c0ab",
          "systemUUID": "4d9f5caa-25de-46c6-8d54-d1c5141b78cc",
          "bootID": "947ffc57-db48-4a03-b7c6-18ce0b85238d",
          "kernelVersion": "4.15.0-91-generic",
          "osImage": "Ubuntu 19.10",
          "containerRuntimeVersion": "docker://19.3.2",
          "kubeletVersion": "v1.17.3",
          "kubeProxyVersion": "v1.17.3",
          "operatingSystem": "linux",
          "architecture": "amd64"
        },
        "images": [
          {
            "names": [
              "k8s.gcr.io/etcd@sha256:4afb99b4690b418ffc2ceb67e1a17376457e441c1f09ab55447f0aaf992fa646",
              "k8s.gcr.io/etcd:3.4.3-0"
            ],
            "sizeBytes": 288426917
          },
          {
            "names": [
              "k8s.gcr.io/kube-apiserver@sha256:33400ea29255bd20714b6b8092b22ebb045ae134030d6bf476bddfed9d33e900",
              "k8s.gcr.io/kube-apiserver:v1.17.3"
            ],
            "sizeBytes": 170986003
          },
          {
            "names": [
              "k8s.gcr.io/kube-controller-manager@sha256:2f0bf4d08e72a1fd6327c8eca3a72ad21af3a608283423bb3c10c98e68759844",
              "k8s.gcr.io/kube-controller-manager:v1.17.3"
            ],
            "sizeBytes": 160918035
          },
          {
            "names": [
              "k8s.gcr.io/kube-proxy@sha256:3a70e2ab8d1d623680191a1a1f1dcb0bdbfd388784b1f153d5630a7397a63fd4",
              "k8s.gcr.io/kube-proxy:v1.17.3"
            ],
            "sizeBytes": 115964919
          },
          {
            "names": [
              "k8s.gcr.io/kube-scheduler@sha256:b091f0db3bc61a3339fd3ba7ebb06c984c4ded32e1f2b1ef0fbdfab638e88462",
              "k8s.gcr.io/kube-scheduler:v1.17.3"
            ],
            "sizeBytes": 94435859
          },
          {
            "names": [
              "kubernetesui/dashboard@sha256:fc90baec4fb62b809051a3227e71266c0427240685139bbd5673282715924ea7",
              "kubernetesui/dashboard:v2.0.0-beta8"
            ],
            "sizeBytes": 90835427
          },
          {
            "names": [
              "gcr.io/k8s-minikube/storage-provisioner@sha256:088daa9fcbccf04c3f415d77d5a6360d2803922190b675cb7fc88a9d2d91985a",
              "gcr.io/k8s-minikube/storage-provisioner:v1.8.1"
            ],
            "sizeBytes": 80815640
          },
          {
            "names": [
              "kindest/kindnetd@sha256:bc1833b3da442bb639008dd5a62861a0419d3f64b58fce6fb38b749105232555",
              "kindest/kindnetd:0.5.3"
            ],
            "sizeBytes": 78486107
          },
          {
            "names": [
              "k8s.gcr.io/coredns@sha256:7ec975f167d815311a7136c32e70735f0d00b73781365df1befd46ed35bd4fe7",
              "k8s.gcr.io/coredns:1.6.5"
            ],
            "sizeBytes": 41578211
          },
          {
            "names": [
              "kubernetesui/metrics-scraper@sha256:2026f9f7558d0f25cc6bab74ea201b4e9d5668fbc378ef64e13fddaea570efc0",
              "kubernetesui/metrics-scraper:v1.0.2"
            ],
            "sizeBytes": 40101552
          },
          {
            "names": [
              "k8s.gcr.io/pause@sha256:f78411e19d84a252e53bff71a4407a5686c46983a2c2eeed83929b888179acea",
              "k8s.gcr.io/pause:3.1"
            ],
            "sizeBytes": 742472
          }
        ]
      }
    }
  ]
}"#;

        let _ = serde_json::from_str::<ObjectList<Node>>(input).unwrap();
    }

    #[test]
    fn url_path() {
        assert_eq!(Node::url_path(None), "/api/v1/nodes".to_string());
        assert_eq!(Node::url_path(Some("foo")), "/api/v1/nodes".to_string());
    }
}
