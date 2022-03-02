//! A wrapper to implement hash for k8s resource objects.

use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::Metadata;

/// A wrapper that provides a [`Hash`] implementation for any k8s resource
/// object.
/// Delegates to object uid for hashing and equality.
#[derive(Debug)]
pub struct HashValue<T: Metadata<Ty = ObjectMeta>>(T);

/// Used to determine what `Metadata` value should be used as the key
/// in `evmap`.
#[derive(Clone, Copy)]
pub enum HashKey {
    /// metadata.uid
    Uid,
    /// metadata.name
    Name,
}

impl<T> HashValue<T>
where
    T: Metadata<Ty = ObjectMeta>,
{
    /// Crate a new `HashValue` by wrapping a value of `T`
    pub fn new(value: T) -> Self {
        Self(value)
    }
}
