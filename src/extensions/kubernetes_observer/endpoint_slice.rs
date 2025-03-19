use kubernetes::{ObjectMeta, Resource};
use serde::Deserialize;

use super::{Keyed, default_protocol};

/// EndpointConditions implements kubernetes endpoint condition.
///
/// See https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.31/#endpointconditions-v1-discovery-k8s-io
#[derive(Deserialize)]
pub struct EndpointConditions {
    /// ready indicates that this endpoint is prepared to receive traffic, according to whatever
    /// system is managing the endpoint. A nil value indicates an unknown state. In most cases
    /// consumers should interpret this unknown state as ready. For compatibility reasons, ready
    /// should never be "true" for terminating endpoints, except when the normal readiness behavior
    /// is being explicitly overridden, for example when the associated Service has set the
    /// publishNotReadyAddresses flag.
    pub ready: bool,
}

/// Endpoint implements kubernetes object endpoint for endpoint slice.
///
/// See https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.31/#endpoint-v1-discovery-k8s-io
#[derive(Deserialize)]
pub struct Endpoint {
    /// addresses of this endpoint. The contents of this field are interpreted according to
    /// the corresponding EndpointSlice addressType field. Consumers must handle different
    /// types of addresses in the context of their own capabilities. This must contain at
    /// least one address but no more than 100. These are all assumed to be fungible and
    /// clients may choose to only use the first element.
    ///
    /// Refer to: https://issue.k8s.io/106267
    pub addresses: Vec<String>,

    /// conditions contains information about the current status of the endpoint.
    pub conditions: EndpointConditions,

    /// hostname of this endpoint. This field may be used by consumers of endpoints to
    /// distinguish endpoints from each other (e.g. in DNS names). Multiple endpoints
    /// which use the same hostname should be considered fungible (e.g. multiple A values in DNS).
    /// Must be lowercase and pass DNS Label (RFC 1123) validation.
    pub hostname: Option<String>,
}

/// EndpointPort implements k8s endpoint port.
///
/// See https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.31/#endpointport-v1-discovery-k8s-io
#[derive(Deserialize)]
pub struct EndpointPort {
    /// The application protocol for this port. This is used as a hint for implementations
    /// to offer richer behavior for protocols that they understand. This field follows
    /// standard Kubernetes label syntax. Valid values are either:
    /// * Un-prefixed protocol names - reserved for IANA standard service names
    ///   (as per RFC-6335 and https://www.iana.org/assignments/service-names).
    /// * Kubernetes-defined prefixed names:
    /// * 'kubernetes.io/h2c' - HTTP/2 prior knowledge over cleartext as described
    ///   in https://www.rfc-editor.org/rfc/rfc9113.html#name-starting-http-2-with-prior-
    /// * 'kubernetes.io/ws' - WebSocket over cleartext as described in https://www.rfc-editor.org/rfc/rfc6455
    /// * 'kubernetes.io/wss' - WebSocket over TLS as described in https://www.rfc-editor.org/rfc/rfc6455
    /// * Other protocols should use implementation-defined prefixed names such as mycompany.com/my-custom-protocol.
    #[serde(rename = "appProtocol")]
    pub app_protocol: Option<String>,

    /// name represents the name of this port. All ports in an EndpointSlice must have a
    /// unique name. If the EndpointSlice is derived from a Kubernetes service, this
    /// corresponds to the Service.ports[].name. Name must either be an empty string or
    /// pass DNS_LABEL validation: * must be no more than 63 characters long.
    /// * must consist of lower case alphanumeric characters or '-'.
    /// * must start and end with an alphanumeric character.
    ///
    /// Default is empty string.
    pub name: String,

    /// port represents the port number of the endpoint. If this is not specified, ports
    /// are not restricted and must be interpreted in the context of the specific consumer.
    pub port: i16,

    /// protocol represents the IP protocol for this port. Must be UDP, TCP, or SCTP. Default is TCP.
    #[serde(default = "default_protocol")]
    pub protocol: String,
}

/// EndpointSlice - implements kubernetes endpoint slice.
///
/// See https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.31/#endpointslice-v1-discovery-k8s-io
#[derive(Deserialize)]
pub struct EndpointSlice {
    pub metadata: ObjectMeta,
    /// addressType specifies the type of address carried by this EndpointSlice. All addresses
    /// in this slice must be the same type. This field is immutable after creation. The
    /// following address types are currently supported:
    /// * IPv4: Represents an IPv4 Address.
    /// * IPv6: Represents an IPv6 Address.
    /// * FQDN: Represents a Fully Qualified Domain Name.
    ///
    #[serde(rename = "addressType")]
    pub address_type: String,

    /// endpoints is a list of unique endpoints in this slice. Each slice may include a
    /// maximum of 1000 endpoints.
    pub endpoints: Vec<Endpoint>,

    /// ports specifies the list of network ports exposed by each endpoint in this slice.
    /// Each port must have a unique name. When ports is empty, it indicates that there
    /// are no defined ports. When a port is defined with a nil port value, it indicates
    /// "all ports". Each slice may include a maximum of 100 ports.
    pub ports: Vec<EndpointPort>,
}

impl Resource for EndpointSlice {
    const GROUP: &'static str = "discovery.k8s.io";
    const VERSION: &'static str = "v1";
    const KIND: &'static str = "EndpointSlice";
    const PLURAL: &'static str = "endpointslices";
}

impl Keyed for EndpointSlice {
    fn key(&self) -> &str {
        self.metadata.uid.as_ref()
    }
}

impl From<EndpointSlice> for Vec<framework::observe::Endpoint> {
    fn from(_slice: EndpointSlice) -> Self {
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
  "kind": "EndpointSliceList",
  "apiVersion": "discovery.k8s.io/v1",
  "metadata": {
    "selfLink": "/apis/discovery.k8s.io/v1/endpointslices",
    "resourceVersion": "1177"
  },
  "items": [
    {
      "metadata": {
        "name": "kubernetes",
        "namespace": "default",
        "selfLink": "/apis/discovery.k8s.io/v1/namespaces/default/endpointslices/kubernetes",
        "uid": "a60d9173-5fe4-4bc3-87a6-269daee71f8a",
        "resourceVersion": "159",
        "generation": 1,
        "creationTimestamp": "2020-09-07T14:27:22Z",
        "labels": {
          "kubernetes.io/service-name": "kubernetes"
        },
        "managedFields": [
          {
            "manager": "kube-apiserver",
            "operation": "Update",
            "apiVersion": "discovery.k8s.io/v1",
            "time": "2020-09-07T14:27:22Z",
            "fieldsType": "FieldsV1",
            "fieldsV1": {"f:addressType":{},"f:endpoints":{},"f:metadata":{"f:labels":{".":{},"f:kubernetes.io/service-name":{}}},"f:ports":{}}
          }
        ]
      },
      "addressType": "IPv4",
      "endpoints": [
        {
          "addresses": [
            "172.18.0.2"
          ],
          "conditions": {
            "ready": true
          }
        }
      ],
      "ports": [
        {
          "name": "https",
          "protocol": "TCP",
          "port": 6443
        }
      ]
    },
    {
      "metadata": {
        "name": "kube-dns-22mvb",
        "generateName": "kube-dns-",
        "namespace": "kube-system",
        "selfLink": "/apis/discovery.k8s.io/v1/namespaces/kube-system/endpointslices/kube-dns-22mvb",
        "uid": "7c95c854-f34c-48e1-86f5-bb8269113c11",
        "resourceVersion": "604",
        "generation": 5,
        "creationTimestamp": "2020-09-07T14:27:39Z",
        "labels": {
          "endpointslice.kubernetes.io/managed-by": "endpointslice-controller.k8s.io",
          "kubernetes.io/service-name": "kube-dns"
        },
        "annotations": {
          "endpoints.kubernetes.io/last-change-trigger-time": "2020-09-07T14:28:35Z"
        },
        "ownerReferences": [
          {
            "apiVersion": "v1",
            "kind": "Service",
            "name": "kube-dns",
            "uid": "509e80d8-6d05-487b-bfff-74f5768f1024",
            "controller": true,
            "blockOwnerDeletion": true
          }
        ],
        "managedFields": [
          {
            "manager": "kube-controller-manager",
            "operation": "Update",
            "apiVersion": "discovery.k8s.io/v1",
            "time": "2020-09-07T14:28:35Z",
            "fieldsType": "FieldsV1",
            "fieldsV1": {"f:addressType":{},"f:endpoints":{},"f:metadata":{"f:annotations":{".":{},"f:endpoints.kubernetes.io/last-change-trigger-time":{}},"f:generateName":{},"f:labels":{".":{},"f:endpointslice.kubernetes.io/managed-by":{},"f:kubernetes.io/service-name":{}},"f:ownerReferences":{".":{},"k:{\"uid\":\"509e80d8-6d05-487b-bfff-74f5768f1024\"}":{".":{},"f:apiVersion":{},"f:blockOwnerDeletion":{},"f:controller":{},"f:kind":{},"f:name":{},"f:uid":{}}}},"f:ports":{}}
          }
        ]
      },
      "addressType": "IPv4",
      "endpoints": [
        {
          "addresses": [
            "10.244.0.3"
          ],
          "conditions": {
            "ready": true
          },
          "targetRef": {
            "kind": "Pod",
            "namespace": "kube-system",
            "name": "coredns-66bff467f8-z8czk",
            "uid": "36a545ff-dbba-4192-a5f6-1dbb0c21c73d",
            "resourceVersion": "603"
          },
          "topology": {
            "kubernetes.io/hostname": "kind-control-plane"
          }
        }
      ],
      "ports": [
        {
          "name": "metrics",
          "protocol": "TCP",
          "port": 9153
        },
        {
          "name": "dns",
          "protocol": "UDP",
          "port": 53
        }
      ]
    }
  ]
}"#;

        let _ = serde_json::from_str::<ObjectList<EndpointSlice>>(input).unwrap();
    }

    #[test]
    fn url() {
        assert_eq!(
            EndpointSlice::url_path(None),
            "/apis/discovery.k8s.io/v1/endpointslices"
        );
        assert_eq!(
            EndpointSlice::url_path(Some("foo")),
            "/apis/discovery.k8s.io/v1/namespaces/foo/endpointslices"
        );
    }
}
