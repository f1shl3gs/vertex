use std::fmt::Debug;

use configurable::NamedComponent;
use serde::{Deserialize, Serialize};

use super::{
    skip_serializing_if_default, ComponentKey, GlobalOptions, Output, ProxyConfig, Resource,
};
use crate::{Pipeline, ShutdownSignal};

pub struct SourceContext {
    pub key: ComponentKey,
    pub output: Pipeline,
    pub shutdown: ShutdownSignal,
    pub globals: GlobalOptions,
    pub proxy: ProxyConfig,
    pub acknowledgements: bool,
}

impl SourceContext {
    #[inline]
    pub fn acknowledgements(&self) -> bool {
        self.acknowledgements || self.globals.acknowledgements
    }

    #[cfg(any(test, feature = "test-util"))]
    pub fn new_test(output: Pipeline) -> Self {
        Self {
            key: "default".into(),
            output,
            shutdown: ShutdownSignal::noop(),
            globals: Default::default(),
            proxy: Default::default(),
            acknowledgements: false,
        }
    }

    #[cfg(test)]
    pub fn new_shutdown(
        key: &ComponentKey,
        output: Pipeline,
    ) -> (Self, crate::ShutdownCoordinator) {
        let mut shutdown = crate::ShutdownCoordinator::default();
        let (shutdown_signal, _) = shutdown.register_source(key);

        (
            Self {
                key: key.clone(),
                globals: GlobalOptions::default(),
                shutdown: shutdown_signal,
                output,
                proxy: Default::default(),
                acknowledgements: false,
            },
            shutdown,
        )
    }
}

/// Generalized trait for describing and building source components.
#[async_trait::async_trait]
#[typetag::serde(tag = "type")]
pub trait SourceConfig: NamedComponent + Debug + Send + Sync {
    /// Builds the source with the given context.
    ///
    /// If the source is built successfully, `Ok(...)` is returned containing the source.
    ///
    /// # Errors
    ///
    /// If an error occurs while building the source, an error variant explaining the
    /// issue is returned.
    async fn build(&self, cx: SourceContext) -> crate::Result<crate::Source>;

    /// Gets the list of outputs exposed by this source.
    fn outputs(&self) -> Vec<Output>;

    /// Gets the list of resources, if any, used by this source.
    ///
    /// Resources represent dependencies -- network ports, file descriptors, and so
    /// on -- that cannot be shared between components at runtime. This ensures that
    /// components can not be configured in a way that would deadlock the spawning
    /// of a topology, and as well, allows Vertex to determine the correct order
    /// for rebuilding a topology during configuration reload when resources must
    /// first be reclaimed before being reassigned, and so on.
    fn resources(&self) -> Vec<Resource> {
        Vec::new()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SourceOuter {
    #[serde(default, skip_serializing_if = "skip_serializing_if_default")]
    pub proxy: ProxyConfig,

    #[serde(flatten)]
    pub inner: Box<dyn SourceConfig>,

    #[serde(default, skip)]
    pub acknowledgements: bool,
}

impl SourceOuter {
    pub fn new(source: impl SourceConfig + 'static) -> Self {
        Self {
            inner: Box::new(source),
            proxy: Default::default(),
            acknowledgements: false,
        }
    }

    pub fn component_name(&self) -> &'static str {
        self.inner.component_name()
    }

    pub fn resources(&self) -> Vec<Resource> {
        self.inner.resources()
    }
}
