mod builder;
mod diff;
mod extension;
mod format;
mod global;
mod graph;
mod helper;
mod id;
mod loading;
mod provider;
mod proxy;
mod resource;
mod sink;
mod source;
mod transform;
mod uri;
mod validation;
#[cfg(all(unix, not(target_os = "macos")))]
pub mod watcher;

// re-export
pub use diff::ConfigDiff;
pub use extension::{ExtensionConfig, ExtensionContext};
pub use format::{Format, FormatHint};
pub use helper::*;
pub use id::{ComponentKey, OutputId};
pub use loading::{load, load_builder_from_paths, load_from_str, merge_path_lists, process_paths};
pub use proxy::ProxyConfig;
pub use sink::{SinkConfig, SinkContext};
pub use source::{SourceConfig, SourceContext};
pub use transform::{TransformConfig, TransformContext};
pub use uri::*;
pub use validation::warnings;

use std::fmt::{Debug, Display, Formatter};
use std::ops::BitOr;
use std::path::PathBuf;

use ::serde::{Deserialize, Serialize};
pub use builder::Builder;
pub use global::GlobalOptions;
// IndexMap preserves insertion order, allowing us to output errors in the
// same order they are present in the file.
use indexmap::IndexMap;
pub use loading::load_from_paths_with_provider;
pub use resource::{Protocol, Resource};

pub use crate::config::sink::SinkOuter;
use crate::config::source::SourceOuter;
use crate::config::transform::TransformOuter;

/// Healthcheck options
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct HealthcheckOptions {
    /// Whether or not healthcheck are enabled for all sinks
    ///
    /// Can be overriden on a per-sink basis.
    pub enabled: bool,

    /// Whether or not to require a sink to report as being healthy during startup.
    ///
    /// When enabled and a sink reports not being healthy, Vertex will exit during
    /// start-up.
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DataType(u32);

impl Display for DataType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut types = vec![];

        if self.contains(DataType::Log) {
            types.push("Log")
        }
        if self.contains(DataType::Metric) {
            types.push("Metric")
        }
        if self.contains(DataType::Trace) {
            types.push("Trace")
        }

        f.write_str(&types.join(","))
    }
}

impl BitOr for DataType {
    type Output = DataType;

    fn bitor(self, rhs: Self) -> Self::Output {
        DataType(self.0.bitor(rhs.0))
    }
}

#[allow(non_upper_case_globals)]
impl DataType {
    pub const Log: DataType = DataType(0x01);
    pub const Metric: DataType = DataType(0x02);
    pub const Trace: DataType = DataType(0x04);
    pub const All: DataType = DataType(!0);

    #[inline]
    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    #[inline]
    pub fn intersects(&self, other: Self) -> bool {
        (self.0 & other.0) != 0 || other.0 == 0
    }
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

    pub healthcheck: HealthcheckOptions,

    #[serde(skip_serializing, skip_deserializing)]
    expansions: IndexMap<ComponentKey, Vec<ComponentKey>>,
}

impl Config {
    pub fn builder() -> Builder {
        Default::default()
    }

    pub fn get_inputs(&self, id: &ComponentKey) -> Vec<ComponentKey> {
        self.expansions
            .get(id)
            .cloned()
            .unwrap_or_else(|| vec![id.clone()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn data_type_contains() {
        let all = DataType::All;
        assert!(all.contains(DataType::Log));
        assert!(all.contains(DataType::Metric));
        assert!(all.contains(DataType::Trace));

        let log = DataType::Log;
        assert!(log.contains(DataType::Log));
        assert!(!log.contains(DataType::Metric));
        assert!(!log.contains(DataType::Trace));

        assert!(!log.contains(DataType::All))
    }

    #[test]
    fn data_type_intersects() {
        let log_and_metric = DataType::Log | DataType::Metric;
        let metric_and_trace = DataType::Metric | DataType::Trace;

        assert!(log_and_metric.intersects(metric_and_trace));
        assert!(log_and_metric.intersects(DataType::Log));
        assert!(log_and_metric.intersects(DataType::Metric));
        assert!(!log_and_metric.intersects(DataType::Trace))
    }

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
    type: rewrite
    inputs:
      - generator
      - ntp
    operations:
      - type: set
        key: hostname
        value: ${HOSTNAME}

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
