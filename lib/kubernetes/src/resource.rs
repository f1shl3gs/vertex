#![allow(dead_code)]

use std::collections::BTreeMap;

use serde::Deserialize;
use serde::de::DeserializeOwned;

/// An accessor trait for a kubernetes Resource.
pub trait Resource: DeserializeOwned {
    /// The group of the resource, or the empty string if the resource doesn't have a
    /// group.
    const GROUP: &'static str;

    /// The version of the resource.
    const VERSION: &'static str;

    /// The kind of the resource.
    ///
    /// This is the string used in the `kind` field of the resource's serialized form.
    const KIND: &'static str;

    /// The plural of this resource, which is used to construct URLS
    const PLURAL: &'static str;

    /// Creates a url path for http requests for this resource
    fn url_path(namespace: Option<&str>) -> String {
        let group = if Self::GROUP.is_empty() {
            "api"
        } else {
            "apis"
        };
        let api_version = if Self::GROUP.is_empty() {
            Self::VERSION.to_string()
        } else {
            format!("{}/{}", Self::GROUP, Self::VERSION)
        };
        let namespace = match namespace {
            Some(namespace) => format!("namespaces/{namespace}/"),
            None => String::new(),
        };
        let plural = Self::PLURAL;

        format!("/{group}/{api_version}/{namespace}{plural}")
    }
}

/// A generic Kubernetes object list
///
/// This is used instead of a full struct for `DeploymentList`, `PodList`, etc.
/// Kubernetes' API [always seem to expose list structs in this manner](https://docs.rs/k8s-openapi/0.10.0/k8s_openapi/apimachinery/pkg/apis/meta/v1/struct.ObjectMeta.html?search=List).
///
/// Note that this is only used internally within reflectors and informers.
/// and is generally produced from list/watch/delete collection queries on
/// an [`Resource`].
///
/// This is almost equivalent to [`k8s_openapi::List<T>`](k8s_openapi::List), but iterable.
#[derive(Deserialize)]
pub struct ObjectList<T> {
    /// ListMeta - only really used for its `resourceVersion`
    pub metadata: ListMeta,

    /// These items we are actually interested in.
    pub items: Vec<T>,
}

fn default_namespace() -> String {
    String::from("default")
}

/// OwnerReference contains enough information to let you identify an owning
/// object. An owning object must be in the same namespace as the dependent,
/// or be cluster-scoped, so there is no namespace field.
#[derive(Debug, Deserialize)]
pub struct OwnerReference {
    /// Name of the referent.
    ///
    /// More info: https://kubernetes.io/docs/concepts/overview/working-with-objects/names#names
    pub name: String,

    /// If true, this reference points to the managing controller.
    pub controller: Option<bool>,

    /// Kind of the referent.
    ///
    /// More info: https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#types-kinds
    pub kind: String,
}

/// ObjectMeta is metadata that all persisted resources must have,
/// which includes all objects users must create.
#[derive(Debug, Default, Deserialize)]
pub struct ObjectMeta {
    /// Name must be unique within a namespace. Is required when creating
    /// resources, although some resource may allow a client to request
    /// the generation of an appropriate name automatically. Name is
    /// primarily intended for creation idempotence and configuration
    /// definition. Cannot be updated.
    ///
    /// More info: https://kubernetes.io/docs/concepts/overview/working-with-objects/names#names
    #[serde(default)]
    pub name: String,

    /// Namespace defines the space within which each name must be unique.
    /// An empty namespace is equivalent to the "default" namespace, but
    /// "default" is the canonical representation. Not all objects are
    /// required to be scoped to a namespace - the value of this field for
    /// those objects will be empty.
    #[serde(default = "default_namespace")]
    pub namespace: String,

    /// UID is the unique in time and space value for this object. It is
    /// typically generated by the server on successful creation of a
    /// resource and is not allowed to change on PUT operations. Populated
    /// by the system.
    ///
    /// Read-only, more info: https://kubernetes.io/docs/concepts/overview/working-with-objects/names#uids
    pub uid: String,

    /// Map of string keys and values that can be used to organize and categorize
    /// (scope and select) objects. May match selectors of replication controllers
    /// and services.
    ///
    /// More info: https://kubernetes.io/docs/concepts/overview/working-with-objects/labels
    #[serde(default)]
    pub labels: BTreeMap<String, String>,

    /// Annotations is an unstructured key value map stored with a resource that
    /// may be set by external tools to store and retrieve arbitrary metadata.
    /// They are not queryable and should be preserved when modifying objects.
    ///
    /// More info: https://kubernetes.io/docs/concepts/overview/working-with-objects/annotations
    #[serde(default)]
    pub annotations: BTreeMap<String, String>,

    /// List of objects depended on this object. If ALL objects in the list have been
    /// deleted, this object will be garbage collected. If this object is managed by
    /// a controller, then an entry in this list will point to this controller, with
    /// the controller field set to true. There cannot be more than one managing
    /// controller.
    #[serde(rename = "ownerReferences")]
    pub owner_references: Option<Vec<OwnerReference>>,

    /// An opaque value that represents the internal version of this object that can be
    /// used by clients to determine when objects have changed. May be used for optimistic
    /// concurrency, change detection, and the watch operation on a resource or set of
    /// resources. Clients must treat these values as opaque and passed unmodified back
    /// to the server. They may only be valid for a particular resource or set of resources.
    ///
    /// Populated by the system. Read-only. Value must be treated as opaque by clients and.
    ///
    /// More info: https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#concurrency-control-and-consistency
    #[serde(rename = "resourceVersion")]
    pub resource_version: Option<String>,
}

/// ListMeta describes metadata that synthetic resources must have, including lists and
/// various status objects. A resource may have only one of {ObjectMeta, ListMeta}.
#[derive(Deserialize)]
pub struct ListMeta {
    /// continue may be set if the user set a limit on the number of items returned, and
    /// indicates that the server has more data available. The value is opaque and may be
    /// used to issue another request to the endpoint that served this list to retrieve
    /// the next set of available objects. Continuing a consistent list may not be possible
    /// if the server configuration has changed or more than a few minutes have passed.
    /// The resourceVersion field returned when using this continue value will be identical
    /// to the value in the first response, unless you have received this token from an
    /// error message.
    pub r#continue: Option<String>,

    /// String that identifies the server's internal version of this object that can be
    /// used by clients to determine when objects have changed. Value must be treated as
    /// opaque be clients and passed unmodified back to the server. Populated by the
    /// system. Read-only.
    ///
    /// More info: https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#concurrency-control-and-consistency
    #[serde(rename = "resourceVersion")]
    pub resource_version: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::ObjectList;

    #[test]
    fn deserialize() {
        #[derive(Deserialize)]
        struct DummySpec {
            name: String,
        }

        #[derive(Deserialize)]
        struct Dummy {
            metadata: ObjectMeta,
            spec: DummySpec,
        }

        let text = r#"
        {
            "metadata": {
                "continue": "111",
                "resourceVersion": "123"
            },
            "items": [
                {
                    "metadata": {
                        "creationTimestamp": "2024-07-21T10:36:25Z",
                        "name": "customer-8cdd87b54-9gf59.17e4340c67a08418",
                        "namespace": "default",
                        "resourceVersion": "707",
                        "uid": "23fc9c5d-6696-46e1-88e3-665b2093dd29"
                    },
                    "spec": {
                        "name": "dummy1"
                    }
                },
                {
                    "metadata": {
                        "creationTimestamp": "2024-07-21T10:36:25Z",
                        "name": "customer-8cdd87b54-9gf59.17e4340c7a09e487",
                        "namespace": "default",
                        "resourceVersion": "5886",
                        "uid": "79c20e1a-2777-43e1-91df-666b8c4d39d0"
                    },
                    "spec": {
                        "name": "dummy2"
                    }
                }
            ]
        }
        "#;

        let list = serde_json::from_str::<ObjectList<Dummy>>(text).unwrap();

        assert_eq!(list.metadata.r#continue, Some("111".into()));
        assert_eq!(list.metadata.resource_version, Some("123".into()));

        assert_eq!(list.items.len(), 2);

        let dummy1 = list.items.first().unwrap();
        assert_eq!(dummy1.spec.name, "dummy1");
        assert_eq!(
            dummy1.metadata.name,
            "customer-8cdd87b54-9gf59.17e4340c67a08418"
        );
        assert_eq!(dummy1.metadata.namespace, "default");
        assert_eq!(dummy1.metadata.resource_version, Some("707".into()));
        assert_eq!(dummy1.metadata.uid, "23fc9c5d-6696-46e1-88e3-665b2093dd29");

        let dummy2 = list.items.get(1).unwrap();
        assert_eq!(dummy2.spec.name, "dummy2");
        assert_eq!(
            dummy2.metadata.name,
            "customer-8cdd87b54-9gf59.17e4340c7a09e487"
        );
        assert_eq!(dummy2.metadata.namespace, "default");
        assert_eq!(dummy2.metadata.resource_version, Some("5886".into()));
        assert_eq!(dummy2.metadata.uid, "79c20e1a-2777-43e1-91df-666b8c4d39d0")
    }
}
