use serde::{Deserialize, Serialize};

use crate::SignalHandler;
use crate::config::{
    Builder,
    provider::ProviderConfig
};


#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FileConfig {
    path: String,
}

#[async_trait::async_trait]
#[typetag::serde(name = "file")]
impl ProviderConfig for FileConfig {
    async fn build(&mut self, _signal_handler: &mut SignalHandler) -> Result<Builder, Vec<String>> {
        todo!()
    }

    fn provider_type(&self) -> &'static str {
        "file"
    }
}