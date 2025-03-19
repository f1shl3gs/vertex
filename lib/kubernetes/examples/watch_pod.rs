use futures::StreamExt;
use kubernetes::{Client, Config, ObjectMeta, Resource, WatchEvent, WatchParams};
use serde::Deserialize;

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
    #[serde(rename = "initContainers")]
    pub init_containers: Option<Vec<Container>>,
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

#[tokio::main]
async fn main() {
    let config = Config::load().unwrap();

    let client = Client::new(config, None);

    let version = client.version().await.unwrap();
    println!("api server version: {}.{}", version.major, version.minor);

    let param = &WatchParams {
        label_selector: None,
        field_selector: None,
        timeout: None,
        bookmarks: true,
        send_initial_events: false,
    };

    // NB: This example is Streaming List which is implement in Kubernetes 1.27,
    // earlier version only support ListWatch with pagination
    let mut resource_version = "0".to_string();
    loop {
        let stream = client
            .watch::<Pod>(param, resource_version.clone())
            .await
            .unwrap();
        tokio::pin!(stream);

        while let Some(result) = stream.next().await {
            match result {
                Ok(watch_event) => match watch_event {
                    WatchEvent::Added(pod) => {
                        println!("add pod: {:?}", pod.metadata.uid);
                    }
                    WatchEvent::Modified(pod) => {
                        println!("modify pod: {:?}", pod.metadata.uid);
                    }
                    WatchEvent::Deleted(pod) => {
                        println!("delete pod: {:?}", pod.metadata.uid);
                    }
                    WatchEvent::Bookmark(bookmark) => {
                        resource_version = bookmark.metadata.resource_version;
                        println!("bookmark: {resource_version}");
                    }
                    WatchEvent::Error(err) => {
                        println!("error event: {:?}", err);
                    }
                },
                Err(err) => {
                    println!("poll next {:?}", err);
                }
            }
        }

        println!("watch timeout, re-watching pods...");
    }
}
