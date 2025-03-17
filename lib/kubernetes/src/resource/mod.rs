#![allow(dead_code)]

#[cfg(feature = "endpoint_slice")]
pub mod endpoint_slice;
#[cfg(feature = "endpoints")]
pub mod endpoints;
#[cfg(feature = "event")]
pub mod event;
#[cfg(feature = "ingress")]
pub mod ingress;
pub mod metadata;
#[cfg(feature = "node")]
pub mod node;
#[cfg(feature = "pod")]
pub mod pod;
#[cfg(feature = "service")]
pub mod service;

use serde::Deserialize;
use serde::de::DeserializeOwned;
use metadata::ObjectMeta;

/// An accessor trait for a kubernetes Resource.
pub trait Resource: DeserializeOwned {
    /// The group of the resource, or the empty string if the resource doesn't have a
    /// group.
    const GROUP: &'static str;

    /// The version of the resource.
    const VERSION: &'static str;

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
            Some(namespace) => format!("namespaces/{}/", namespace),
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
    pub metadata: metadata::ListMeta,

    /// These items we are actually interested in.
    pub items: Vec<T>,
}
