//! Build watch request for k8s API and adapters for library types.

use k8s_openapi::{
    apimachinery::pkg::apis::meta::v1::ObjectMeta,
    http::{Request, StatusCode},
    Metadata, RequestError, ResponseBody, ResponseError, WatchOptional, WatchResponse,
};
use serde::de::DeserializeOwned;

/// Build a watch request for the k8s API
pub trait WatchRequestBuilder {
    /// The object type that's being watched
    type Object: Metadata<Ty = ObjectMeta> + DeserializeOwned;

    /// Build a watch request
    fn build(&self, watch_optional: WatchOptional<'_>) -> Result<Request<Vec<u8>>, RequestError>;
}

impl<F, T> WatchRequestBuilder for F
where
    T: Metadata<Ty = ObjectMeta> + DeserializeOwned,
    F: for<'w> Fn(
        WatchOptional<'w>,
    ) -> Result<
        (
            Request<Vec<u8>>,
            fn(StatusCode) -> ResponseBody<WatchResponse<T>>,
        ),
        RequestError,
    >,
{
    type Object = T;

    fn build(&self, watch_optional: WatchOptional<'_>) -> Result<Request<Vec<u8>>, RequestError> {
        let (req, _) = (self)(watch_optional)?;
        Ok(req)
    }
}

/// Wrapper for a namespace API
///
/// Specify the namespace and an API request building function.
pub struct Namespace<N, F>(pub N, pub F);

impl<N, F, T> WatchRequestBuilder for Namespace<N, F>
where
    N: AsRef<str>,
    T: Metadata<Ty = ObjectMeta> + DeserializeOwned,
    F: for<'w> Fn(
        &'w str,
        WatchOptional<'w>,
    ) -> Result<
        (
            Request<Vec<u8>>,
            fn(StatusCode) -> ResponseBody<WatchResponse<T>>,
        ),
        ResponseError,
    >,
{
    type Object = T;

    fn build(&self, watch_optional: WatchOptional<'_>) -> Result<Request<Vec<u8>>, RequestError> {
        let (req, _) = (self.1)(self.0.as_ref(), watch_optional)?;
        Ok(req)
    }
}
