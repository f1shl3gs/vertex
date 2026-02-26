use std::collections::BTreeMap;

use framework::observe::Endpoint;
use kubernetes::{ObjectMeta, Resource};
use serde::Deserialize;
use value::value;

use super::Keyed;

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
    const KIND: &'static str = "Ingress";

    const PLURAL: &'static str = "ingresses";
}

impl Keyed for Ingress {
    fn key(&self) -> &str {
        self.metadata.uid.as_ref()
    }
}

impl From<Ingress> for Vec<Endpoint> {
    fn from(ingress: Ingress) -> Self {
        let name = ingress.metadata.name;
        let namespace = ingress.metadata.namespace;

        let mut labels = BTreeMap::new();
        for (k, v) in ingress.metadata.labels {
            labels.insert(k, v.into());
        }

        let mut annotations = BTreeMap::new();
        for (k, v) in ingress.metadata.annotations {
            annotations.insert(k, v.into());
        }

        let mut endpoints = Vec::new();
        for rule in ingress.spec.rules {
            let Some(HttpIngressRuleValue { paths }) = rule.http else {
                continue;
            };

            if paths.is_empty() {
                continue;
            }

            let scheme = if ingress
                .spec
                .tls
                .iter()
                .any(|tls| tls.hosts.contains(&rule.host))
            {
                "https"
            } else {
                "http"
            };

            for path in paths {
                let id = format!(
                    "{}/{}/{}://{}/{}",
                    namespace, name, scheme, rule.host, path.path
                );
                let target = format!("{}://{}/{}", scheme, rule.host, path.path);

                endpoints.push(Endpoint {
                    id,
                    typ: "ingress".into(),
                    target,
                    details: value!({
                        "ingress": name.clone(),
                        "namespace": namespace.clone(),
                        "address": rule.host.clone(),
                        "scheme": scheme,
                        "host": rule.host.clone(),
                        "path": path.path,
                        "labels": labels.clone(),
                        "annotations": annotations.clone(),
                        "ingress_class_name": ingress.spec.ingress_class_name.clone(),
                    }),
                });
            }
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
