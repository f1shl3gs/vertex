use std::path::PathBuf;

use async_trait::async_trait;
// IndexMap preserves insertion order, allowing us to output errors in the same order they are present in the file
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

pub use helper::*;
pub use diff::ConfigDiff;
pub use format::{Format, FormatHint};

use crate::{
    buffers::acker::Acker,
    pipeline::Pipeline,
    sinks,
    sources,
    timezone,
    transforms,
};
use crate::shutdown::ShutdownSignal;

pub mod format;
mod pipeline;
mod loading;
mod diff;
mod helper;
mod provider;
mod builder;
mod resource;
mod validation;

pub use resource::{
    Protocol,
    Resource,
};

pub use builder::Builder;

pub use helper::{
    deserialize_duration,
    deserialize_regex,
    serialize_duration,
    serialize_regex,
};
pub use loading::{
    load_from_paths_with_provider,
};
use futures::future::BoxFuture;
use crate::extensions::Extension;

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

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub enum ExpandType {
    Parallel,
    Serial,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct RuntimeConfig {
    #[serde(default = "default_worker")]
    pub worker: usize,
}

pub fn default_worker() -> usize {
    num_cpus::get()
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct GlobalOptions {
    #[serde(default = "default_data_dir")]
    pub data_dir: Option<PathBuf>,

    #[serde(default = "default_timezone")]
    pub timezone: timezone::TimeZone,
}

fn default_timezone() -> timezone::TimeZone {
    Default::default()
}

fn default_data_dir() -> Option<PathBuf> {
    Some(PathBuf::from("/var/lib/vertex"))
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
}

impl SinkOuter {
    pub fn new(inputs: Vec<String>, inner: Box<dyn SinkConfig>) -> Self {
        Self {
            inner,
            inputs,
            buffer: Default::default(),
        }
    }

    pub fn resources(&self, _id: &str) -> Vec<Resource> {
        self.inner.resources()
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

    pub sources: IndexMap<String, Box<dyn SourceConfig>>,

    pub transforms: IndexMap<String, TransformOuter>,

    pub sinks: IndexMap<String, SinkOuter>,

    #[serde(rename = "health_checks")]
    pub health_checks: HealthcheckOptions,
}

impl Config {}

pub struct SourceContext {
    pub name: String,
    pub out: Pipeline,
    pub shutdown: ShutdownSignal,
    pub global: GlobalOptions,
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
    fn expand(&mut self) -> crate::Result<Option<(IndexMap<String, Box<dyn TransformConfig>>, ExpandType)>> {
        Ok(None)
    }
}

#[derive(Debug, Clone)]
pub struct SinkContext {
    pub globals: GlobalOptions,
    pub acker: Acker,
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
    pub global: GlobalOptions,
}

#[async_trait]
#[typetag::serde(tag = "type")]
pub trait ExtensionConfig: core::fmt::Debug + Send + Sync {
    async fn build(&self, ctx: ExtensionConfig) -> crate::Result<Extension>;



    fn resource(&self) -> Vec<Resource> { Vec::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_config() {
        let text = "\
sources:
  node:
    type: node_metrics

transforms:
  add_tags:
    type: add_tags
    tags:
      foo: bar

sink:
  prom:
    type: prometheus
    listen: :3080
  stdout:
    type: stdout

service:
  extensions:
  pipelines:
    - sources:
        - node
      transforms:
        - relabel
      sink:
        - prom
        - stdout
        ";

        let cb: Config = format::deserialize(text, Some(format::Format::YAML)).unwrap();
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
        println!("{:?}", d.d)
    }
}