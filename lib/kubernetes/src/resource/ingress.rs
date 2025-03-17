use serde::Deserialize;

use super::{ObjectMeta, Resource};

/// IngressTLS represents ingress TLS spec in k8s.
///
/// See https://v1-21.docs.kubernetes.io/docs/reference/generated/kubernetes-api/v1.21/#ingresstls-v1-networking-k8s-io
#[derive(Deserialize)]
pub struct IngressTLS {
    pub hosts: Vec<String>,
}

/// HTTPIngressPath represents HTTP ingress path in k8s.
///
/// See https://v1-21.docs.kubernetes.io/docs/reference/generated/kubernetes-api/v1.21/#httpingresspath-v1-networking-k8s-io
#[derive(Deserialize)]
pub struct HttpIngressPath {
    pub path: String,
}

/// HTTPIngressRuleValue represents HTTP ingress rule value in k8s.
///
/// See https://v1-21.docs.kubernetes.io/docs/reference/generated/kubernetes-api/v1.21/#httpingressrulevalue-v1-networking-k8s-io
#[derive(Deserialize)]
pub struct HttpIngressRuleValue {
    paths: Vec<HttpIngressPath>,
}

/// IngressRule represents ingress rule in k8s.
///
/// See https://v1-21.docs.kubernetes.io/docs/reference/generated/kubernetes-api/v1.21/#ingressrule-v1-networking-k8s-io
#[derive(Deserialize)]
pub struct IngressRule {
    pub host: String,
    pub http: Option<HttpIngressRuleValue>,
}

/// IngressSpec represents ingress spec in k8s.
///
/// See https://v1-21.docs.kubernetes.io/docs/reference/generated/kubernetes-api/v1.21/#ingressspec-v1-networking-k8s-io
#[derive(Deserialize)]
pub struct IngressSpec {
    #[serde(default)]
    tls: Vec<IngressTLS>,
    rules: Vec<IngressRule>,
    #[serde(rename = "ingressClassName")]
    ingress_class_name: String,
}

/// Ingress represents ingress in k8s.
///
/// See https://v1-21.docs.kubernetes.io/docs/reference/generated/kubernetes-api/v1.21/#ingress-v1-networking-k8s-io
#[derive(Deserialize)]
pub struct Ingress {
    /// Standard object's metadata.
    ///
    /// More info: https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#metadata
    pub metadata: ObjectMeta,

    pub spec: IngressSpec,
}

impl Resource for Ingress {
    const GROUP: &'static str = "networking.k8s.io";
    const VERSION: &'static str = "v1";
    const PLURAL: &'static str = "ingresses";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::ObjectList;

    #[test]
    fn deserialize() {
        let data = r#"
{
  "kind": "IngressList",
  "apiVersion": "extensions/v1",
  "metadata": {
    "selfLink": "/apis/extensions/v1/ingresses",
    "resourceVersion": "351452"
  },
  "items": [
    {
      "metadata": {
        "name": "test-ingress",
        "namespace": "default",
        "selfLink": "/apis/extensions/v1/namespaces/default/ingresses/test-ingress",
        "uid": "6d3f38f9-de89-4bc9-b273-c8faf74e8a27",
        "resourceVersion": "351445",
        "generation": 1,
        "creationTimestamp": "2020-04-13T16:43:52Z",
        "annotations": {
          "kubectl.kubernetes.io/last-applied-configuration": "{\"apiVersion\":\"networking.k8s.io/v1\",\"kind\":\"Ingress\",\"metadata\":{\"annotations\":{},\"name\":\"test-ingress\",\"namespace\":\"default\"},\"spec\":{\"backend\":{\"serviceName\":\"testsvc\",\"servicePort\":80}}}\n"
        }
      },
      "spec": {
        "backend": {
          "serviceName": "testsvc",
          "servicePort": 80
        },
	"rules": [
	  {
            "host": "foobar"
          }
	],
	"ingressClassName": "foo-class"
      },
      "status": {
        "loadBalancer": {
          "ingress": [
            {
              "ip": "172.17.0.2"
            }
          ]
        }
      }
    }
  ]
}"#;

        let _ = serde_json::from_str::<ObjectList<Ingress>>(data).unwrap();
    }

    #[test]
    fn url() {
        assert_eq!(
            Ingress::url_path(None),
            "/apis/networking.k8s.io/v1/ingresses"
        );
        assert_eq!(
            Ingress::url_path(Some("foo")),
            "/apis/networking.k8s.io/v1/namespaces/foo/ingresses"
        );
    }
}
