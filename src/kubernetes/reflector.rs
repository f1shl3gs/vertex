//! Watch and cache the remote Kubernetes API resources.

use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::Metadata;
use std::time::Duration;

use super::state;

/// Watches remote Kubernetes resources and maintains a local representation
/// of the remote state. "Reflects" the remote state locally.
///
/// Does not expose evented API, but keeps track of the resource version
/// and will automatically resume on desync.
pub struct Reflector<W, S>
where
    W: Watcher,
    <W as Watcher>::Object: Metadata<Ty = ObjectMeta> + Send,
    S: state::MaintainedWrite<Item = <W as Watcher>::Object>,
{
    watcher: W,
    state_writer: S,
    field_selector: Option<String>,
    label_selector: Option<String>,
    resource_version: resource_version::State,
    pause_between_requests: Duration,
}

impl<W, S> Reflector<W, S>
where
    W: Watcher,
    <W as Watcher>::Object: Metadata<Ty = ObjectMeta> + Send,
    S: state::MaintainedWrite<Item = <W as Watcher>::Object>,
{
    /// Create a new `Reflector`
    pub fn new(
        watcher: W,
        state_writer: S,
        field_selector: Option<String>,
        label_selector: Option<String>,
        pause_between_requests: Duration,
    ) -> Self {
        let resource_version = resource_version::State::new();

        Self {
            watcher,
            state_writer,
            field_selector,
            label_selector,
            resource_version,
            pause_between_requests,
        }
    }
}

// TODO
