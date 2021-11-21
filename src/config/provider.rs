mod http;

use async_trait::async_trait;
use crate::{signal, SignalHandler};
use serde::{Deserialize, Serialize};
use super::builder::Builder;

pub type Result<T> = std::result::Result<T, Vec<String>>;

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait ProviderConfig: core::fmt::Debug + Send + Sync + dyn_clone::DynClone {
    /// Builds a provider, returning a string containing the config. It's passed
    /// a signals cahannel to control reloading and shutdown, as applicable.
    async fn build(&mut self, signal_handler: &mut signal::SignalHandler) -> Result<Builder>;

    fn provider_type(&self) -> &'static str;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FileConfig {
    path: String,
}

#[async_trait]
#[typetag::serde(name = "file")]
impl ProviderConfig for FileConfig {
    async fn build(&mut self, _signal_handler: &mut SignalHandler) -> Result<Builder> {
        todo!()
    }

    fn provider_type(&self) -> &'static str {
        todo!()
    }
}