use serde::Deserialize;

use super::{ObjectMeta, Resource};

/// EndpointAddress implements k8s endpoint address.
///
/// See https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.31/#endpointaddress-v1-core
#[derive(Deserialize)]
pub struct EndpointAddress {
    pub hostname: String,
    pub ip: String,
    #[serde(rename = "nodeName")]
    pub node_name: String,
}

/// EndpointPort implements k8s endpoint port.
///
/// See https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.31/#endpointport-v1-discovery-k8s-io
#[derive(Deserialize)]
pub struct EndpointPort {
    #[serde(rename = "appProtocol")]
    pub app_protocol: Option<String>,
    pub name: String,
    pub port: i16,
    pub protocol: String,
}

/// EndpointSubset implements k8s endpoint subset.
///
/// See https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.31/#endpointsubset-v1-core
#[derive(Deserialize)]
pub struct EndpointSubset {
    pub addresses: Vec<EndpointAddress>,
    #[serde(default, rename = "notReadyAddresses")]
    pub not_ready_addresses: Vec<EndpointAddress>,
    pub ports: Vec<EndpointPort>,
}

/// Endpoints implements kubernetes endpoints
///
/// See https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.31/#endpoints-v1-core
#[derive(Deserialize)]
pub struct Endpoints {
    pub metadata: ObjectMeta,
    pub subsets: Vec<EndpointSubset>,
}

impl Resource for Endpoints {
    const GROUP: &'static str = "";
    const VERSION: &'static str = "v1";
    const PLURAL: &'static str = "endpoints";

    fn url_path(_namespace: Option<&str>) -> String {
        format!("/api/{}/endpoints", Self::VERSION)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::ObjectList;

    #[test]
    fn deserialize() {
        let input = r#"
{
  "kind": "EndpointsList",
  "apiVersion": "v1",
  "metadata": {
    "selfLink": "/api/v1/endpoints",
    "resourceVersion": "128055"
  },
  "items": [
    {
      "metadata": {
        "name": "kubernetes",
        "namespace": "default",
        "selfLink": "/api/v1/namespaces/default/endpoints/kubernetes",
        "uid": "0972c7d9-c267-4b93-a090-a417eeb9b385",
        "resourceVersion": "150",
        "creationTimestamp": "2020-03-16T20:44:25Z",
        "labels": {
          "foo": "bar"
        },
        "annotations": {
            "x": "y"
        }
      },
      "subsets": [
        {
          "addresses": [
            {
	      "hostname": "aaa.bbb",
	      "nodeName": "test-node",
              "ip": "172.17.0.2",
              "targetRef": {
                "kind": "Pod",
                "namespace": "kube-system",
                "name": "coredns-6955765f44-lnp6t",
                "uid": "cbddb2b6-5b85-40f1-8819-9a59385169bb",
                "resourceVersion": "124878"
              }
            }
          ],
          "ports": [
            {
              "name": "https",
              "port": 8443,
              "protocol": "TCP"
            }
          ]
        }
      ]
    }
  ]
}"#;

        let _ = serde_json::from_str::<ObjectList<Endpoints>>(input).unwrap();
    }

    #[test]
    fn url() {
        assert_eq!(Endpoints::url_path(None), "/api/v1/endpoints");
        assert_eq!(Endpoints::url_path(Some("foo")), "/api/v1/endpoints");
    }
}
