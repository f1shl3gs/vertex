use std::fmt::Debug;

use async_trait::async_trait;
use configurable::NamedComponent;

use crate::config::{GlobalOptions, Resource};
use crate::{Extension, ShutdownSignal};

#[derive(Clone)]
pub struct ExtensionContext {
    pub name: String,
    pub global: GlobalOptions,
    pub shutdown: ShutdownSignal,
}

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait ExtensionConfig: NamedComponent + Debug + Send + Sync {
    async fn build(&self, cx: ExtensionContext) -> crate::Result<Extension>;

    fn resources(&self) -> Vec<Resource> {
        Vec::new()
    }
}
