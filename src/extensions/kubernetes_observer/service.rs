use std::collections::BTreeMap;

use bytes::Bytes;
use framework::observe::Endpoint;
use kubernetes::{ObjectMeta, Resource};
use serde::Deserialize;
use value::{Value, value};

use super::Keyed;

fn default_protocol() -> String {
    String::from("TCP")
}

/// ServicePort is k8s service port.
///
/// See https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.17/#serviceport-v1-core
#[derive(Deserialize)]
pub struct ServicePort {
    /// The name of this port within the service. This must be a DNS_LABEL. All ports within a
    /// ServiceSpec must have unique names. When considering the endpoints for a Service, this
    /// must match the 'name' field in the EndpointPort. Optional if only one ServicePort is
    /// defined on this service.
    pub name: String,

    /// The IP protocol for this port. Supports "TCP", "UDP", and "SCTP". Default is TCP.
    #[serde(default = "default_protocol")]
    pub protocol: String,

    /// The port that will be exposed by this service.
    pub port: u16,
}

/// ServiceSpec is k8s service spec.
///
/// See https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.17/#servicespec-v1-core
#[derive(Deserialize)]
pub struct ServiceSpec {
    /// clusterIP is the IP address of the service and is usually assigned randomly. If an address
    /// is specified manually, is in-range (as per system configuration), and is not in use, it
    /// will be allocated to the service; otherwise creation of the service will fail. This field
    /// may not be changed through updates unless the type field is also being changed to
    /// ExternalName (which requires this field to be blank) or the type field is being changed
    /// from ExternalName (in which case this field may optionally be specified, as describe above).
    /// Valid values are "None", empty string (""), or a valid IP address. Setting this to "None"
    /// makes a "headless service" (no virtual IP), which is useful when direct endpoint
    /// connections are preferred and proxying is not required. Only applies to types ClusterIP,
    /// NodePort, and LoadBalancer. If this field is specified when creating a Service of type
    /// ExternalName, creation will fail. This field will be wiped when updating a Service to type
    /// ExternalName.
    ///
    /// More info: https://kubernetes.io/docs/concepts/services-networking/service/#virtual-ips-and-service-proxies
    #[serde(rename = "clusterIP")]
    cluster_ip: String,

    /// externalName is the external reference that discovery mechanisms will return as an alias
    /// for this service (e.g. a DNS CNAME record). No proxying will be involved. Must be a
    /// lowercase RFC-1123 hostname (https://tools.ietf.org/html/rfc1123) and requires `type`
    /// to be "ExternalName".
    external_name: Option<String>,

    /// type determines how the Service is exposed. Defaults to ClusterIP. Valid options are
    /// ExternalName, ClusterIP, NodePort, and LoadBalancer. "ClusterIP" allocates a
    /// cluster-internal IP address for load-balancing to endpoints. Endpoints are determined by
    /// the selector or if that is not specified, by manual construction of an Endpoints object
    /// or EndpointSlice objects. If clusterIP is "None", no virtual IP is allocated and the
    /// endpoints are published as a set of endpoints rather than a virtual IP. "NodePort" builds
    /// on ClusterIP and allocates a port on every node which routes to the same endpoints as the
    /// clusterIP. "LoadBalancer" builds on NodePort and creates an external load-balancer (if
    /// supported in the current cloud) which routes to the same endpoints as the clusterIP.
    /// "ExternalName" aliases this service to the specified externalName. Several other fields
    /// do not apply to ExternalName services.
    ///
    /// More info: https://kubernetes.io/docs/concepts/services-networking/service/#publishing-services-service-types
    #[serde(rename = "type")]
    typ: String,

    /// The list of ports that are exposed by this service.
    ///
    /// More info: https://kubernetes.io/docs/concepts/services-networking/service/#virtual-ips-and-service-proxies
    ports: Vec<ServicePort>,
}

/// Service is k8s service.
///
/// See https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.17/#service-v1-core
#[derive(Deserialize)]
pub struct Service {
    pub metadata: ObjectMeta,
    pub spec: ServiceSpec,
}

impl Resource for Service {
    const GROUP: &'static str = "";
    const VERSION: &'static str = "v1";
    const KIND: &'static str = "Service";
    const PLURAL: &'static str = "services";
}

impl Keyed for Service {
    fn key(&self) -> &str {
        self.metadata.uid.as_ref()
    }
}

impl From<Service> for Vec<Endpoint> {
    fn from(service: Service) -> Self {
        let mut labels = BTreeMap::new();
        for (key, value) in service.metadata.labels {
            labels.insert(key, Value::Bytes(Bytes::from(value)));
        }

        let mut annotations = BTreeMap::new();
        for (key, value) in service.metadata.annotations {
            annotations.insert(key, Value::Bytes(Bytes::from(value)));
        }

        let service_info = value!({
            "namespace": service.metadata.namespace.clone(),
            "name": service.metadata.name.clone(),
            "labels": labels,
            "annotations": annotations,
        });

        let mut endpoints = Vec::with_capacity(service.spec.ports.len());
        for port in service.spec.ports {
            let id = format!(
                "{}/{}/{}:{}",
                service.metadata.namespace, service.metadata.name, port.protocol, port.port
            );
            let target = format!(
                "{}.{}.svc:{}",
                service.metadata.name, service.metadata.namespace, port.port
            );
            let mut details = value!({
                "service": service_info.clone(),
                "name": port.name,
                "port": port.port,
                "protocol": port.protocol,
            });

            if service.spec.typ == "ExternalName" {
                if let Some(external_name) = service.spec.external_name.as_ref() {
                    details.insert("external_name", external_name.clone());
                }
            } else {
                details.insert("cluster_ip", service.spec.cluster_ip.clone());
            }

            endpoints.push(Endpoint {
                id,
                target,
                typ: "service".into(),
                details,
            });
        }

        endpoints
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kubernetes::ObjectList;

    #[test]
    fn deserialize() {
        let input = r#"{
  "kind": "ServiceList",
  "apiVersion": "v1",
  "metadata": {
    "selfLink": "/api/v1/services",
    "resourceVersion": "60485"
  },
  "items": [
    {
      "metadata": {
        "name": "kube-dns",
        "namespace": "kube-system",
        "selfLink": "/api/v1/namespaces/kube-system/services/kube-dns",
        "uid": "38a396f1-17fe-46c2-a5f4-3b225c18dcdf",
        "resourceVersion": "177",
        "creationTimestamp": "2020-03-16T20:44:26Z",
        "labels": {
          "k8s-app": "kube-dns",
          "kubernetes.io/cluster-service": "true",
          "kubernetes.io/name": "KubeDNS"
        },
        "annotations": {
          "prometheus.io/port": "9153",
          "prometheus.io/scrape": "true"
        }
      },
      "spec": {
        "ports": [
          {
            "name": "dns",
            "protocol": "UDP",
            "port": 53,
            "targetPort": 53
          },
          {
            "name": "dns-tcp",
            "protocol": "TCP",
            "port": 53,
            "targetPort": 53
          },
          {
            "name": "metrics",
            "protocol": "TCP",
            "port": 9153,
            "targetPort": 9153
          }
        ],
        "selector": {
          "k8s-app": "kube-dns"
        },
        "clusterIP": "10.96.0.10",
        "type": "ClusterIP",
        "sessionAffinity": "None"
      },
      "status": {
        "loadBalancer": {

        }
      }
    }
  ]
}"#;

        let _ = serde_json::from_str::<ObjectList<Service>>(input).unwrap();
    }

    #[test]
    fn url() {
        assert_eq!(Service::url_path(None), "/api/v1/services");
        assert_eq!(
            Service::url_path(Some("foo")),
            "/api/v1/namespaces/foo/services"
        );
    }
}
