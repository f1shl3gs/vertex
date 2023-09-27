use std::fmt::Debug;

use async_trait::async_trait;
use buffers::BufferType;
use configurable::NamedComponent;
use serde::{Deserialize, Serialize};

use super::{
    default_true, skip_serializing_if_default, ComponentKey, DataType, GlobalOptions, ProxyConfig,
    Resource,
};

#[derive(Debug, Clone)]
pub struct SinkContext {
    pub globals: GlobalOptions,
    pub proxy: ProxyConfig,
    pub health_check: bool,
}

impl SinkContext {
    pub const fn proxy(&self) -> &ProxyConfig {
        &self.proxy
    }

    #[cfg(any(test, feature = "test-util"))]
    pub fn new_test() -> Self {
        Self {
            globals: Default::default(),
            proxy: Default::default(),
            health_check: true,
        }
    }
}

/// Generalized interface for describing and building sink components.
#[async_trait]
#[typetag::serde(tag = "type")]
pub trait SinkConfig: NamedComponent + Debug + Send + Sync {
    /// Builds the sink with the given context.
    async fn build(&self, cx: SinkContext) -> crate::Result<(crate::Sink, crate::Healthcheck)>;

    /// Gets the input configuration for this sink
    fn input_type(&self) -> DataType;

    /// Gets the list of resources, if any, used by this sink.
    ///
    /// Resources represent dependencies -- network ports, file descriptors, and
    /// so on -- that cannot be shared between components at runtime. This ensures
    /// that components can not be configured in a way that would deadlock the
    /// spawning of a topology, and as well, allows vertex to determine the correct
    /// order for rebuilding a topology during configuration reload when resources
    /// must first be reclaimed before being reassigned, and so on.
    fn resources(&self) -> Vec<Resource> {
        Vec::new()
    }

    /// Gets the acknowledgements configuration for this sink.
    fn acknowledgements(&self) -> bool {
        false
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SinkOuter<T> {
    pub inputs: Vec<T>,

    #[serde(flatten)]
    pub inner: Box<dyn SinkConfig>,

    #[serde(default)]
    pub buffer: buffers::BufferConfig,

    #[serde(default = "default_true")]
    pub health_check: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "skip_serializing_if_default")]
    proxy: ProxyConfig,
}

impl<T> SinkOuter<T> {
    pub fn new(inputs: Vec<T>, inner: Box<dyn SinkConfig>) -> Self {
        Self {
            inner,
            inputs,
            buffer: Default::default(),
            proxy: Default::default(),
            health_check: true,
        }
    }

    pub fn component_name(&self) -> &'static str {
        self.inner.component_name()
    }

    pub fn resources(&self, id: &ComponentKey) -> Vec<Resource> {
        let mut resources = self.inner.resources();

        for stage in self.buffer.stages() {
            match stage {
                BufferType::Memory { .. } => {}
                BufferType::Disk { .. } => resources.push(Resource::DiskBuffer(id.to_string())),
            }
        }

        resources
    }

    #[inline]
    pub const fn health_check(&self) -> bool {
        self.health_check
    }

    pub const fn proxy(&self) -> &ProxyConfig {
        &self.proxy
    }

    pub fn with_inputs<U>(self, inputs: Vec<U>) -> SinkOuter<U> {
        SinkOuter {
            inputs,
            inner: self.inner,
            buffer: self.buffer,
            health_check: self.health_check,
            proxy: self.proxy,
        }
    }
}
