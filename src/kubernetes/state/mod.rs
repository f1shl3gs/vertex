pub mod delayed_delete;
pub mod evmap;
pub mod hash_value;

use async_trait::async_trait;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::Metadata;
use tonic::codegen::BoxFuture;

/// Provides the interface for write access to the cached state.
/// Used by `super::reflector::Reflector`.
///
/// This abstraction allows easily stacking storage behaviour logic,
/// without exploding the complexity at the
/// `super::reflector::Reflector` level.
#[async_trait]
pub trait Write {
    /// A type of the k8s resource the state operates on.
    type Item: Metadata<Ty = ObjectMeta> + Send;

    /// Add an object to the state.
    async fn add(&mut self, item: Self::Item);

    /// Update an object at the state.
    async fn update(&mut self, item: Self::Item);

    /// Delete an object from the state
    async fn delete(&mut self, item: Self::Item);

    /// Notify the state that resync is in progress.
    async fn resync(&mut self);
}

/// An extension of the `Write` type that adds maintenance support.
#[async_trait]
pub trait MaintainedWrite: Write {
    /// A future that resolves when maintenance is required.
    ///
    /// Does not perform the maintenance itself, users must call
    /// `Self::perform_maintenance` to actually perform the maintenance.
    ///
    /// `None` if the state doesn't require maintenance, and
    /// `Self::perform_maintenance` shouldn't be called.
    /// `futures::future::FusedFuture` should've been used here, but it's
    /// not trivially implementable with `async/await` syntax, so `Option`
    /// wrapper is used instead for the same purpose.
    ///
    /// Circumstances of whether maintenance is required or not can change
    /// at runtime. for instance, whether the maintenance is required can
    /// depend on whether state is empty on not. Ultimately it's up to the
    /// state implementation to decide whether maintenance is needed or not.
    fn maintenance_request(&mut self) -> Option<BoxFuture<'_, ()>>;

    /// Perform the maintenance.
    ///
    /// If this function is called when no maintenance is required, this
    /// function should just return.
    ///
    /// Wrapper `MaintainedWrite` should always call the `perform_maintenance`
    /// of the wrapped state when `perform_maintenance` is called.
    async fn perform_maintenance(&mut self);
}
