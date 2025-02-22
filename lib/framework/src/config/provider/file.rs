use configurable::configurable_component;

use crate::SignalHandler;
use crate::config::{Builder, provider::ProviderConfig};

#[configurable_component(provider, name = "file")]
#[derive(Clone)]
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
