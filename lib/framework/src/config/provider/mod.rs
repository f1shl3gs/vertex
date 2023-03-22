mod file;
mod http;

use std::fmt::Debug;

use async_trait::async_trait;
use configurable::NamedComponent;

use super::builder::Builder;
use crate::signal;

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait ProviderConfig: NamedComponent + Debug + Send + Sync {
    /// Builds a provider, returning a string containing the config. It's passed
    /// a signals channel to control reloading and shutdown, as applicable.
    async fn build(
        &mut self,
        signal_handler: &mut signal::SignalHandler,
    ) -> Result<Builder, Vec<String>>;
}
