use configurable::configurable_component;

use crate::config::{provider::ProviderConfig, Builder};
use crate::SignalHandler;

#[configurable_component(provider, name = "file")]
#[derive(Clone, Debug)]
pub struct FileConfig {
    path: String,
}

#[async_trait::async_trait]
#[typetag::serde(name = "file")]
impl ProviderConfig for FileConfig {
    async fn build(&mut self, _signal_handler: &mut SignalHandler) -> Result<Builder, Vec<String>> {
        todo!()
    }
}
