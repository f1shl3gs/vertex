mod diff;
mod extension;
mod format;
mod global;
mod healthcheck;
mod helper;
pub mod http;
mod id;
mod loading;
mod paths;
mod provider;
mod proxy;
mod resource;
mod sink;
mod source;
mod transform;
mod uri;
#[cfg(all(unix, not(target_os = "macos")))]
pub mod watcher;

use std::fmt::{Debug, Display, Formatter};
use std::ops::BitOr;

pub use diff::ConfigDiff;
pub use extension::{ExtensionConfig, ExtensionContext, ExtensionOuter};
pub use format::{Format, FormatHint};
pub use global::GlobalOptions;
pub use healthcheck::HealthcheckOptions;
pub use helper::{
    default_interval, default_true, serde_http_method, serde_regex, serde_uri,
    skip_serializing_if_default,
};
pub use id::{ComponentKey, OutputId};
use indexmap::IndexMap;
#[cfg(feature = "test-util")]
pub use loading::load_from_str;
pub use loading::{
    Builder, load, load_builder_from_paths, load_from_paths_with_provider_and_secrets,
};
pub use paths::{ConfigPath, process_paths};
pub use proxy::ProxyConfig;
pub use resource::{Protocol, Resource};
use serde::Serialize;
pub use sink::{SinkConfig, SinkContext, SinkOuter};
pub use source::{SourceConfig, SourceContext, SourceOuter};
pub use transform::{TransformConfig, TransformContext, TransformOuter, get_transform_output_ids};
pub use uri::UriSerde;

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
pub struct OutputType {
    pub port: Option<String>,
    pub typ: DataType,
}

impl OutputType {
    /// Create a default `OutputType` of the given data type
    ///
    /// A default output is one without a port identifier (i.e., not a named output)
    /// and the default output consumers will receive if they declare the component
    /// itself as an input
    #[must_use]
    pub const fn new(typ: DataType) -> Self {
        Self { port: None, typ }
    }

    /// Create an `OutputType` of the given data type that contains no output `Definition`s.
    /// Designed for use in metrics sources
    ///
    /// Sets the datatype to be [`DataType::Metric`]
    #[must_use]
    pub fn metric() -> Self {
        Self {
            port: None,
            typ: DataType::Metric,
        }
    }

    /// Create an `OutputType` of the given data type that contains a single output `Definition`s.
    /// Designed for use in log sources.
    ///
    /// Sets the datatype to be [`DataType::Log`]
    #[must_use]
    pub fn log() -> Self {
        Self {
            port: None,
            typ: DataType::Log,
        }
    }

    /// Create an `OutputType` of the given data type that contains no output `Definition`s.
    /// Designed for use in trace sources.
    ///
    /// Sets the datatype to be [`DataType::Trace`]
    #[must_use]
    pub fn trace() -> Self {
        Self {
            port: None,
            typ: DataType::Trace,
        }
    }

    /// Set the port name for this `Output`
    pub fn with_port(mut self, name: impl Into<String>) -> Self {
        self.port = Some(name.into());
        self
    }
}

impl<T: Into<String>> From<(T, DataType)> for OutputType {
    fn from((name, typ): (T, DataType)) -> Self {
        Self {
            port: Some(name.into()),
            typ,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InputType {
    typ: DataType,
}

impl InputType {
    #[inline]
    pub const fn new(typ: DataType) -> Self {
        Self { typ }
    }

    #[inline]
    pub const fn log() -> Self {
        Self { typ: DataType::Log }
    }

    #[inline]
    pub const fn metric() -> Self {
        Self {
            typ: DataType::Metric,
        }
    }

    #[inline]
    pub const fn trace() -> Self {
        Self {
            typ: DataType::Trace,
        }
    }

    #[inline]
    pub const fn all() -> Self {
        Self { typ: DataType::All }
    }

    #[inline]
    pub fn data_type(&self) -> DataType {
        self.typ
    }
}

#[derive(Debug, Default, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub global: GlobalOptions,

    pub healthcheck: HealthcheckOptions,

    pub extensions: IndexMap<ComponentKey, ExtensionOuter>,

    pub sources: IndexMap<ComponentKey, SourceOuter>,

    pub transforms: IndexMap<ComponentKey, TransformOuter<OutputId>>,

    pub sinks: IndexMap<ComponentKey, SinkOuter<OutputId>>,
}

impl Config {
    pub fn builder() -> Builder {
        Default::default()
    }

    pub fn propagate_acknowledgements(&mut self) -> Result<(), Vec<String>> {
        let inputs = self
            .sinks
            .iter()
            .filter(|(_, sink)| sink.inner.acknowledgements())
            .flat_map(|(name, sink)| {
                sink.inputs
                    .iter()
                    .map(|input| (name.clone(), input.clone()))
            })
            .collect();

        self.propagate_acks_rec(inputs);

        Ok(())
    }

    fn propagate_acks_rec(&mut self, sink_inputs: Vec<(ComponentKey, OutputId)>) {
        for (sink, input) in sink_inputs {
            let component = &input.component;

            if let Some(source) = self.sources.get_mut(component) {
                if source.inner.can_acknowledge() {
                    source.sink_acknowledgements = true;
                } else {
                    warn!(
                        message = "Source has acknowledgements enabled by a sink, but acknowledgements are not supported by this source. Silent data loss could occur.",
                        source = component.id(),
                        sink = sink.id()
                    );
                }
            } else if let Some(transform) = self.transforms.get(component) {
                let inputs = transform
                    .inputs
                    .iter()
                    .map(|input| (sink.clone(), input.clone()))
                    .collect();

                self.propagate_acks_rec(inputs);
            }
        }
    }

    pub fn warnings(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        let source_ids = self.sources.iter().flat_map(|(key, source)| {
            source
                .inner
                .outputs()
                .iter()
                .map(|output| {
                    if let Some(port) = &output.port {
                        ("source", OutputId::from((key, port.clone())))
                    } else {
                        ("source", OutputId::from(key))
                    }
                })
                .collect::<Vec<_>>()
        });
        let transform_ids = self.transforms.iter().flat_map(|(key, transform)| {
            get_transform_output_ids(transform.inner.as_ref(), key.clone())
                .map(|output| ("transform", output))
                .collect::<Vec<_>>()
        });

        for (typ, id) in transform_ids.chain(source_ids) {
            if !self
                .transforms
                .iter()
                .any(|(_, transform)| transform.inputs.contains(&id))
                && !self.sinks.iter().any(|(_, sink)| sink.inputs.contains(&id))
            {
                warnings.push(format!("{} {id:?} has no consumers", typ))
            }
        }

        warnings
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde::Deserialize;

    use super::*;

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
    type: node
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
    type: relabel
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

        let _b: Builder = Format::YAML.deserialize(text).unwrap();
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
