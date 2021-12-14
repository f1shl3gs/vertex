mod builder;
mod component;
mod diff;
mod format;
mod global;
mod helper;
mod loading;
mod provider;
mod proxy;
mod resource;
mod uri;
mod validation;

// re-export
#[cfg(test)]
pub use component::test_generate_config;
pub use component::{ComponentDescription, ExampleError, GenerateConfig};
pub use diff::ConfigDiff;
pub use format::{Format, FormatHint};
pub use helper::*;
pub use loading::load;
pub use provider::ProviderDescription;
pub use proxy::ProxyConfig;
pub use uri::*;

use async_trait::async_trait;
use std::path::PathBuf;
// IndexMap preserves insertion order, allowing us to output errors in the same order they are present in the file
use ::serde::{Deserialize, Serialize};
use indexmap::IndexMap;

use crate::shutdown::ShutdownSignal;
use crate::{pipeline::Pipeline, sinks, sources, transforms};

pub use resource::{Protocol, Resource};

pub use builder::Builder;

pub use crate::config::global::GlobalOptions;
use crate::extensions::Extension;
use buffers::Acker;
use futures::future::BoxFuture;
pub use helper::{
    deserialize_duration, deserialize_regex, serialize_duration, serialize_regex,
    skip_serializing_if_default,
};
pub use loading::load_from_paths_with_provider;

pub type HealthCheck = BoxFuture<'static, crate::Result<()>>;

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

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum DataType {
    Any,
    Log,
    Metric,
    Trace,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum ExpandType {
    Parallel,
    Serial,
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
    pub inner: Box<dyn SourceConfig>,
}

impl SourceOuter {
    pub fn new(source: impl SourceConfig + 'static) -> Self {
        Self {
            inner: Box::new(source),
            proxy: Default::default(),
        }
    }

    pub fn resources(&self) -> Vec<Resource> {
        self.inner.resources()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TransformOuter {
    pub inputs: Vec<String>,

    #[serde(flatten)]
    pub inner: Box<dyn TransformConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SinkOuter {
    pub inputs: Vec<String>,

    #[serde(flatten)]
    pub inner: Box<dyn SinkConfig>,

    #[serde(default)]
    pub buffer: crate::buffers::BufferConfig,

    #[serde(default = "default_true")]
    pub health_check: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "skip_serializing_if_default")]
    proxy: ProxyConfig,
}

impl SinkOuter {
    pub fn new(inputs: Vec<String>, inner: Box<dyn SinkConfig>) -> Self {
        Self {
            inner,
            inputs,
            buffer: Default::default(),
            proxy: Default::default(),
            health_check: true,
        }
    }

    pub fn resources(&self, _id: &str) -> Vec<Resource> {
        self.inner.resources()
    }

    pub fn health_check(&self) -> bool {
        self.health_check
    }

    pub const fn proxy(&self) -> &ProxyConfig {
        &self.proxy
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

    pub sources: IndexMap<String, SourceOuter>,

    pub transforms: IndexMap<String, TransformOuter>,

    pub sinks: IndexMap<String, SinkOuter>,

    pub extensions: IndexMap<String, Box<dyn ExtensionConfig>>,

    #[serde(rename = "health_checks")]
    pub health_checks: HealthcheckOptions,

    #[serde(skip_serializing, skip_deserializing)]
    expansions: IndexMap<String, Vec<String>>,
}

pub struct SourceContext {
    pub name: String,
    pub out: Pipeline,
    pub shutdown: ShutdownSignal,
    pub global: GlobalOptions,
    pub proxy: ProxyConfig,
}

impl SourceContext {
    #[cfg(test)]
    pub fn new_test(output: Pipeline) -> Self {
        Self {
            name: "default".to_string(),
            out: output,
            shutdown: ShutdownSignal::noop(),
            global: Default::default(),
            proxy: Default::default(),
        }
    }
}

#[async_trait::async_trait]
#[typetag::serde(tag = "type")]
pub trait SourceConfig: core::fmt::Debug + Send + Sync {
    async fn build(&self, ctx: SourceContext) -> crate::Result<sources::Source>;

    fn output_type(&self) -> DataType;

    fn source_type(&self) -> &'static str;

    /// Resources that the source is using
    fn resources(&self) -> Vec<Resource> {
        Vec::new()
    }
}

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait TransformConfig: core::fmt::Debug + Send + Sync + dyn_clone::DynClone {
    async fn build(&self, globals: &GlobalOptions) -> crate::Result<transforms::Transform>;

    fn input_type(&self) -> DataType;

    fn output_type(&self) -> DataType;

    fn transform_type(&self) -> &'static str;

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
    pub(super) globals: GlobalOptions,
    pub(super) acker: Acker,
    pub(super) proxy: ProxyConfig,
    pub(super) health_check: bool,
}

impl SinkContext {
    pub const fn proxy(&self) -> &ProxyConfig {
        &self.proxy
    }

    pub fn acker(&self) -> Acker {
        self.acker.clone()
    }

    #[cfg(test)]
    pub fn new_test() -> Self {
        Self {
            globals: Default::default(),
            acker: Acker::Null,
            proxy: Default::default(),
            health_check: true,
        }
    }
}

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait SinkConfig: core::fmt::Debug + Send + Sync {
    async fn build(&self, ctx: SinkContext) -> crate::Result<(sinks::Sink, HealthCheck)>;

    fn input_type(&self) -> DataType;

    fn sink_type(&self) -> &'static str;

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
    async fn build(&self, ctx: ExtensionContext) -> crate::Result<Extension>;

    fn extension_type(&self) -> &'static str;

    fn resources(&self) -> Vec<Resource> {
        Vec::new()
    }
}

pub type SourceDescription = ComponentDescription<Box<dyn SourceConfig>>;
inventory::collect!(SourceDescription);

pub type TransformDescription = ComponentDescription<Box<dyn TransformConfig>>;
inventory::collect!(TransformDescription);

pub type SinkDescription = ComponentDescription<Box<dyn SinkConfig>>;
inventory::collect!(SinkDescription);

pub type ExtensionDescription = ComponentDescription<Box<dyn ExtensionConfig>>;
inventory::collect!(ExtensionDescription);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
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
#  journald:
#    type: journald
#    units: []
#    excludes: []

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

        let _cb: Config = format::deserialize(text, Some(format::Format::YAML)).unwrap();
    }

    #[derive(Serialize, Deserialize)]
    struct WithDuration {
        #[serde(serialize_with = "serialize_duration")]
        #[serde(deserialize_with = "deserialize_duration")]
        pub d: chrono::Duration,
    }

    #[test]
    fn test_deserialize_duration() {
        let d: WithDuration = serde_yaml::from_str("d: 5m3s").unwrap();
        assert_eq!(d.d, chrono::Duration::seconds(5 * 60 + 3));
    }
}
