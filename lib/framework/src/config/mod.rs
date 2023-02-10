mod builder;
mod diff;
mod format;
mod global;
mod graph;
mod helper;
mod id;
mod loading;
mod provider;
mod proxy;
mod resource;
mod uri;
mod validation;
#[cfg(all(unix, not(target_os = "macos")))]
pub mod watcher;

// re-export
pub use configurable::{
    component::{
        ExampleError, ExtensionDescription, ProviderDescription, SinkDescription,
        SourceDescription, TransformDescription,
    },
    GenerateConfig,
};
pub use diff::ConfigDiff;
pub use format::{Format, FormatHint};
pub use helper::*;
pub use id::{ComponentKey, OutputId};
pub use loading::{load, load_builder_from_paths, load_from_str, merge_path_lists, process_paths};
pub use proxy::ProxyConfig;
pub use uri::*;
pub use validation::warnings;

use async_trait::async_trait;
use std::collections::HashSet;
use std::fmt::Debug;
use std::path::PathBuf;
use std::time::Duration;
// IndexMap preserves insertion order, allowing us to output errors in the same order they are present in the file
use ::serde::{Deserialize, Serialize};
use indexmap::IndexMap;

use crate::shutdown::ShutdownSignal;

pub use resource::{Protocol, Resource};

pub use builder::Builder;

use crate::{transform::Noop, Extension, Pipeline};
use buffers::{Acker, BufferType};
use configurable::NamedComponent;
pub use global::GlobalOptions;
pub use helper::{deserialize_regex, serialize_regex, skip_serializing_if_default};
pub use loading::load_from_paths_with_provider;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(default)]
pub struct HealthcheckOptions {
    pub enabled: bool,
    pub require_healthy: bool,
}

impl HealthcheckOptions {
    pub fn set_require_healthy(&mut self, require_healthy: impl Into<Option<bool>>) {
        if let Some(require_healthy) = require_healthy.into() {
            self.require_healthy = require_healthy;
        }
    }

    fn merge(&mut self, other: Self) {
        self.enabled &= other.enabled;
        self.require_healthy |= other.require_healthy;
    }
}

impl Default for HealthcheckOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            require_healthy: false,
        }
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum DataType {
    Any,
    Log,
    Metric,
    Trace,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Output {
    pub port: Option<String>,
    pub typ: DataType,
}

impl Output {
    /// Create a default `Output` of the given data type
    ///
    /// A default output is one without a port identifier (i.e. not a named output)
    /// and the default output consumers will receive if they declare the component
    /// itself as an input
    pub const fn default(typ: DataType) -> Self {
        Self { port: None, typ }
    }

    /// Set the port name for this `Output`
    pub fn with_port(mut self, name: impl Into<String>) -> Self {
        self.port = Some(name.into());
        self
    }
}

impl<T: Into<String>> From<(T, DataType)> for Output {
    fn from((name, typ): (T, DataType)) -> Self {
        Self {
            port: Some(name.into()),
            typ,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub enum ExpandType {
    /// Chain components together one after another. Components will be named according
    /// to this order (e.g. component_name.0 and so on). If alias is set to true,
    /// then a Noop transform will be added as the last component and given the raw
    /// component_name identifier so that it can be used as an input for other components.
    Parallel { aggregates: bool },
    /// This ways of expanding will take all the components and chain then in order.
    /// The first node will be renamed `component_name.0` and so on.
    /// If `alias` is set to `true, then a `Noop` transform will be added as the
    /// last component and named `component_name` so that it can be used as an input.
    Serial { alias: bool },
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub sources: Vec<String>,

    pub transforms: Vec<String>,

    pub sinks: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub extensions: Vec<String>,
    pub pipelines: Vec<PipelineConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SourceOuter {
    #[serde(default, skip_serializing_if = "skip_serializing_if_default")]
    pub proxy: ProxyConfig,

    #[serde(flatten)]
    pub(super) inner: Box<dyn SourceConfig>,

    #[serde(default, skip)]
    pub(super) acknowledgements: bool,

    #[serde(default)]
    #[serde(with = "humanize::duration::serde_option")]
    pub(super) interval: Option<Duration>,
}

impl SourceOuter {
    pub fn new(source: impl SourceConfig + 'static) -> Self {
        Self {
            inner: Box::new(source),
            proxy: Default::default(),
            acknowledgements: false,
            interval: None,
        }
    }

    pub fn component_name(&self) -> &'static str {
        self.inner.component_name()
    }

    pub fn resources(&self) -> Vec<Resource> {
        self.inner.resources()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TransformOuter<T> {
    pub inputs: Vec<T>,

    #[serde(flatten)]
    pub inner: Box<dyn TransformConfig>,
}

impl<T> TransformOuter<T> {
    fn with_inputs<U>(self, inputs: Vec<U>) -> TransformOuter<U> {
        TransformOuter {
            inputs,
            inner: self.inner,
        }
    }
}

impl TransformOuter<String> {
    pub(crate) fn expand(
        mut self,
        key: ComponentKey,
        parent_types: &HashSet<&'static str>,
        transforms: &mut IndexMap<ComponentKey, TransformOuter<String>>,
        expansions: &mut IndexMap<ComponentKey, Vec<ComponentKey>>,
    ) -> Result<(), String> {
        if !self.inner.nestable(parent_types) {
            return Err(format!(
                "the component {} cannot be nested in {:?}",
                self.inner.component_name(),
                parent_types
            ));
        }

        let expansion = self
            .inner
            .expand()
            .map_err(|err| format!("failed to expand transform '{}': {}", key, err))?;

        let mut ptypes = parent_types.clone();
        ptypes.insert(self.inner.component_name());

        if let Some((expanded, expand_type)) = expansion {
            let mut children = Vec::new();
            let mut inputs = self.inputs.clone();

            for (name, content) in expanded {
                let full_name = key.join(name);

                let child = TransformOuter {
                    inputs,
                    inner: content,
                };
                child.expand(full_name.clone(), &ptypes, transforms, expansions)?;
                children.push(full_name.clone());

                inputs = match expand_type {
                    ExpandType::Parallel { .. } => self.inputs.clone(),
                    ExpandType::Serial { .. } => vec![full_name.to_string()],
                }
            }

            if matches!(expand_type, ExpandType::Parallel { aggregates: true }) {
                transforms.insert(
                    key.clone(),
                    TransformOuter {
                        inputs: children.iter().map(ToString::to_string).collect(),
                        inner: Box::new(Noop),
                    },
                );
                children.push(key.clone());
            } else if matches!(expand_type, ExpandType::Serial { alias: true }) {
                transforms.insert(
                    key.clone(),
                    TransformOuter {
                        inputs,
                        inner: Box::new(Noop),
                    },
                );
                children.push(key.clone());
            }

            expansions.insert(key.clone(), children);
        } else {
            transforms.insert(key, self);
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct TransformContext {
    // This is optional because currently there are a lot of places we use `TransformContext`
    // that may not have the relevant data available (e.g. tests). In the furture it'd be
    // nice to make it required somehow.
    pub key: Option<ComponentKey>,
    pub globals: GlobalOptions,
}

impl TransformContext {
    pub fn new_with_globals(globals: GlobalOptions) -> Self {
        Self {
            globals,
            ..Default::default()
        }
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

    fn with_inputs<U>(self, inputs: Vec<U>) -> SinkOuter<U> {
        SinkOuter {
            inputs,
            inner: self.inner,
            buffer: self.buffer,
            health_check: self.health_check,
            proxy: self.proxy,
        }
    }
}

#[derive(Debug, Clone, Ord, PartialOrd, PartialEq, Eq)]
pub enum ConfigPath {
    File(PathBuf, FormatHint),
    Dir(PathBuf),
}

impl<'a> From<&'a ConfigPath> for &'a PathBuf {
    fn from(path: &'a ConfigPath) -> Self {
        match path {
            ConfigPath::File(path, _) => path,
            ConfigPath::Dir(path) => path,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub global: GlobalOptions,

    pub sources: IndexMap<ComponentKey, SourceOuter>,

    pub transforms: IndexMap<ComponentKey, TransformOuter<OutputId>>,

    pub sinks: IndexMap<ComponentKey, SinkOuter<OutputId>>,

    pub extensions: IndexMap<ComponentKey, Box<dyn ExtensionConfig>>,

    pub healthchecks: HealthcheckOptions,

    #[serde(skip_serializing, skip_deserializing)]
    expansions: IndexMap<ComponentKey, Vec<ComponentKey>>,
}

impl Config {
    pub fn builder() -> builder::Builder {
        Default::default()
    }

    pub fn get_inputs(&self, id: &ComponentKey) -> Vec<ComponentKey> {
        self.expansions
            .get(id)
            .cloned()
            .unwrap_or_else(|| vec![id.clone()])
    }
}

pub struct SourceContext {
    pub key: ComponentKey,
    pub output: Pipeline,
    pub shutdown: ShutdownSignal,
    pub globals: GlobalOptions,
    pub proxy: ProxyConfig,
    pub acknowledgements: bool,
    pub interval: Duration,
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
            interval: default_interval(),
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
                interval: default_interval(),
            },
            shutdown,
        )
    }
}

#[async_trait::async_trait]
#[typetag::serde(tag = "type")]
pub trait SourceConfig: NamedComponent + Debug + Send + Sync {
    async fn build(&self, cx: SourceContext) -> crate::Result<crate::Source>;

    fn outputs(&self) -> Vec<Output>;

    /// Resources that the source is using
    fn resources(&self) -> Vec<Resource> {
        Vec::new()
    }
}

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait TransformConfig: NamedComponent + Debug + Send + Sync {
    async fn build(&self, cx: &TransformContext) -> crate::Result<crate::Transform>;

    fn input_type(&self) -> DataType;

    fn outputs(&self) -> Vec<Output>;

    /// Returns true if the transform is able to be run across multiple tasks simultaneously
    /// with no concerns around statfulness, ordering, etc
    fn enable_concurrency(&self) -> bool {
        false
    }

    /// Allows to detect if a transform can be embedded in another transform.
    /// It's used by the pipelines transform for now
    fn nestable(&self, _parents: &HashSet<&'static str>) -> bool {
        true
    }

    /// Allows a transform configuration to expand itself into multiple "child"
    /// transformations to replace it. this allows a transform to act as a
    /// macro for various patterns
    fn expand(
        &mut self,
    ) -> crate::Result<Option<(IndexMap<String, Box<dyn TransformConfig>>, ExpandType)>> {
        Ok(None)
    }
}

#[derive(Debug, Clone)]
pub struct SinkContext {
    pub globals: GlobalOptions,
    pub acker: Acker,
    pub proxy: ProxyConfig,
    pub health_check: bool,
}

impl SinkContext {
    pub const fn proxy(&self) -> &ProxyConfig {
        &self.proxy
    }

    pub fn acker(&self) -> Acker {
        self.acker.clone()
    }

    #[cfg(any(test, feature = "test-util"))]
    pub fn new_test() -> Self {
        Self {
            globals: Default::default(),
            acker: Acker::passthrough(),
            proxy: Default::default(),
            health_check: true,
        }
    }
}

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait SinkConfig: NamedComponent + Debug + Send + Sync {
    async fn build(&self, cx: SinkContext) -> crate::Result<(crate::Sink, crate::Healthcheck)>;

    fn input_type(&self) -> DataType;

    fn resources(&self) -> Vec<Resource> {
        Vec::new()
    }
}

#[derive(Debug, Clone)]
pub struct ExtensionContext {
    pub name: String,
    pub global: GlobalOptions,
    pub shutdown: ShutdownSignal,
}

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait ExtensionConfig: core::fmt::Debug + Send + Sync {
    async fn build(&self, cx: ExtensionContext) -> crate::Result<Extension>;

    fn extension_type(&self) -> &'static str;

    fn resources(&self) -> Vec<Resource> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    #[ignore]
    fn deserialize_config() {
        let text = "\
global:
  data_dir: ./temp

health_checks:
  enabled: false

extensions:
  pprof:
    type: pprof
    listen: 127.0.0.1:9000

sources:
  zookeeper:
    type: zookeeper
    endpoint: 127.0.0.1:49158
  redis:
    type: redis
    interval: 15s
    url: redis://localhost:6379
  internal_metrics:
    type: internal_metrics
    interval: 15s
  kmsg:
    type: kmsg
  node:
    type: node_metrics
    interval: 15s
  selfstat:
    type: selfstat
  # generator:
  #   type: generator
  ntp:
    type: ntp
    interval: 15s
    timeout: 10s
    pools:
      - time1.aliyun.com
      - time2.aliyun.com
      - time3.aliyun.com
      - time4.aliyun.com
      - time5.aliyun.com
      - time6.aliyun.com
      - time7.aliyun.com

transforms:
  add_extra_tags:
    type: add_tags
    inputs:
      - generator
      - ntp
    tags:
      hostname: ${HOSTNAME}

sinks:
  blackhole:
    type: blackhole
    inputs:
      - kmsg
      - node
  prom:
    type: prometheus_exporter
    inputs:
      - add_extra_tags
      - selfstat
      - internal_metrics
      - redis
      - zookeeper
    listen: 127.0.0.1:9101

        ";

        let _b: Builder = format::deserialize(text, Some(format::Format::YAML)).unwrap();
    }

    #[test]
    fn deserialize_with_duration() {
        #[derive(Serialize, Deserialize)]
        struct WithDuration {
            #[serde(with = "humanize::duration::serde")]
            pub d: Duration,
        }

        let d: WithDuration = serde_yaml::from_str("d: 5m3s").unwrap();
        assert_eq!(d.d, Duration::from_secs(5 * 60 + 3));
    }
}
