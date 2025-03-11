use std::fmt::Debug;

use async_trait::async_trait;
use configurable::NamedComponent;
use serde::{Deserialize, Serialize};

use super::{GlobalOptions, ProxyConfig, Resource, skip_serializing_if_default};
use crate::{Extension, ShutdownSignal};

#[derive(Clone)]
pub struct ExtensionContext {
    pub name: String,
    pub global: GlobalOptions,
    pub proxy: ProxyConfig,
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

#[derive(Debug, Deserialize, Serialize)]
pub struct ExtensionOuter {
    #[serde(default, skip_serializing_if = "skip_serializing_if_default")]
    pub(crate) proxy: ProxyConfig,

    #[serde(flatten)]
    pub(crate) inner: Box<dyn ExtensionConfig>,
}

impl ExtensionOuter {
    #[inline]
    pub fn proxy(&self) -> &ProxyConfig {
        &self.proxy
    }

    #[inline]
    pub fn component_name(&self) -> &'static str {
        self.inner.component_name()
    }

    #[inline]
    pub fn resources(&self) -> Vec<Resource> {
        self.inner.resources()
    }
}
