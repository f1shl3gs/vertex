use std::collections::BTreeMap;
use std::path::PathBuf;

use bytes::Bytes;
use configurable::{configurable_component, Configurable};
use event::tags::Tags;
use event::{Events, LogRecord, Metric, MetricValue};
use framework::config::{DataType, Output, TransformConfig, TransformContext};
use framework::{SyncTransform, Transform, TransformOutputsBuf};
use serde::{Deserialize, Serialize};
use value::path::{PathPrefix, TargetPath};
use value::{ObjectMap, OwnedSegment, OwnedTargetPath, Value};
use vtl::{ContextError, Diagnostic, Program, Target};

const VALID_METRIC_PATHS_SET: &str = ".name, .timestamp, .kind, .tags";

/// Metrics aren't interested in paths that have a length longer than 3.
///
/// The longest path is 2, and we need to check that a third segment doesn't exist as we don't want
/// fields such as `.tags.host.thing`.
const MAX_METRIC_PATH_DEPTH: usize = 3;

/// ErrorMode determines how this transformer reacts to errors.
#[derive(Clone, Configurable, Debug, Default, Deserialize, Serialize, PartialEq)]
enum ErrorMode {
    /// Drop the event, and write a log.
    Drop,

    /// Skip this error, and continue to transform the event.
    #[default]
    Continue,
}

#[derive(Configurable, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Source {
    File(PathBuf),
    Script(String),
}

impl Source {
    fn load(&self) -> Result<String, std::io::Error> {
        match self {
            Source::File(path) => std::fs::read_to_string(path),
            Source::Script(content) => Ok(content.to_string()),
        }
    }
}

#[configurable_component(transform, name = "rewrite")]
struct Config {
    #[serde(flatten)]
    source: Source,
}

#[async_trait::async_trait]
#[typetag::serde(name = "rewrite")]
impl TransformConfig for Config {
    async fn build(&self, _cx: &TransformContext) -> framework::Result<Transform> {
        let script = self.source.load()?;

        match vtl::compile(&script) {
            Ok(program) => {
                let rewrite = Rewrite {
                    // error_mode: self.error_mode.clone(),
                    program,
                };

                Ok(Transform::synchronous(rewrite))
            }
            Err(err) => {
                let diagnostic = Diagnostic::new(script);
                Err(diagnostic.snippets(err).into())
            }
        }
    }

    fn input_type(&self) -> DataType {
        DataType::Log | DataType::Metric
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log | DataType::Metric)]
    }
}

const DROPPED: &str = "dropped";

#[derive(Clone)]
struct Rewrite {
    // error_mode: ErrorMode,
    program: Program,
}

impl SyncTransform for Rewrite {
    fn transform(&mut self, events: Events, output: &mut TransformOutputsBuf) {
        match events {
            Events::Logs(logs) => {
                for log in logs {
                    let mut target = LogTarget { log };
                    match self.program.run(&mut target) {
                        Ok(_value) => {
                            output.push(target.log.into());
                        }
                        Err(err) => {
                            warn!(message = "", ?err, internal_log_rate_limit = true,);

                            output.push_named(DROPPED, target.log.into())
                        }
                    }
                }
            }
            Events::Metrics(metrics) => {
                for metric in metrics {
                    let value = precompute_metric_value(&metric, self.program.target_queries());
                    let mut target = MetricTarget { metric, value };

                    match self.program.run(&mut target) {
                        Ok(_value) => {
                            output.push(target.metric.into());
                        }
                        Err(err) => {
                            warn!(message = "", ?err, internal_log_rate_limit = true);

                            output.push_named(DROPPED, target.metric.into());
                        }
                    }
                }
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
struct LogTarget {
    log: LogRecord,
}

impl Target for LogTarget {
    #[inline]
    fn insert(&mut self, path: &OwnedTargetPath, value: Value) -> Result<(), ContextError> {
        self.log.insert(path, value);
        Ok(())
    }

    #[inline]
    fn get(&mut self, path: &OwnedTargetPath) -> Result<Option<&Value>, ContextError> {
        Ok(self.log.get(path))
    }

    #[inline]
    fn get_mut(&mut self, path: &OwnedTargetPath) -> Result<Option<&mut Value>, ContextError> {
        Ok(self.log.get_mut(path))
    }

    fn remove(
        &mut self,
        path: &OwnedTargetPath,
        compact: bool,
    ) -> Result<Option<Value>, ContextError> {
        Ok(self.log.remove_prune(path, compact))
    }
}

#[derive(Debug)]
struct MetricTarget {
    metric: Metric,
    value: Value,
}

fn tags_to_value(tags: &Tags) -> Value {
    use event::tags::Value::*;

    let mut object = BTreeMap::new();
    for (key, value) in tags {
        let value = match value {
            Bool(b) => Value::Boolean(*b),
            I64(i) => Value::Integer(*i),
            F64(f) => Value::Float(*f),
            String(s) => Value::Bytes(Bytes::copy_from_slice(s.as_bytes())),
            Array(arr) => {
                let value = match arr {
                    event::tags::Array::Bool(b) => {
                        b.iter().map(|b| (*b).into()).collect::<Vec<_>>()
                    }
                    event::tags::Array::I64(i) => i.iter().map(|i| (*i).into()).collect::<Vec<_>>(),
                    event::tags::Array::F64(f) => f.iter().map(|f| (*f).into()).collect::<Vec<_>>(),
                    event::tags::Array::String(s) => s.iter().map(|s| s.into()).collect::<Vec<_>>(),
                };

                Value::Array(value)
            }
        };

        object.insert(key.to_string(), value);
    }

    Value::Object(object)
}

/// Pre-compute the `Value` structure of the metric.
///
/// This structure is partially populated based on the fields accessed
/// by the VTL program as informed by `Program`.
fn precompute_metric_value(metric: &Metric, paths: &[OwnedTargetPath]) -> Value {
    let mut map = ObjectMap::new();

    let mut set_name = false;
    let mut set_type = false;
    let mut set_timestamp = false;
    let mut set_tags = false;

    for target_path in paths {
        // Accessing a root path requires us to pre-populate all fields
        if target_path == &OwnedTargetPath::event_root() {
            if !set_name {
                map.insert("name".to_string(), metric.name().into());
            }

            if !set_type {
                let typ = match metric.value {
                    MetricValue::Sum(_) => "sum",
                    MetricValue::Gauge(_) => "gauge",
                    MetricValue::Summary { .. } => "summary",
                    MetricValue::Histogram { .. } => "histogram",
                };

                map.insert("type".to_string(), typ.into());
            }

            if !set_timestamp {
                if let Some(ts) = metric.timestamp {
                    map.insert("timestamp".into(), ts.into());
                }
            }

            if !set_tags {
                let value = tags_to_value(metric.tags());
                map.insert("tags".to_string(), value);
            }

            break;
        }

        // For non-root paths, we continuously populate the value with
        // the relevant data.
        if let Some(OwnedSegment::Field(field)) = target_path.path.segments.first() {
            match field.as_ref() {
                "name" if !set_name => {
                    set_name = true;
                    map.insert("name".to_string(), metric.name().into());
                }

                "type" if !set_type => {
                    set_type = true;
                    let typ = match metric.value {
                        MetricValue::Sum(_) => "sum",
                        MetricValue::Gauge(_) => "gauge",
                        MetricValue::Summary { .. } => "summary",
                        MetricValue::Histogram { .. } => "histogram",
                    };

                    map.insert("type".to_string(), typ.into());
                }

                "timestamp" if !set_timestamp && metric.timestamp.is_some() => {
                    set_timestamp = true;
                    map.insert("timestamp".to_string(), metric.timestamp().unwrap().into());
                }

                "tags" if !set_tags => {
                    set_tags = true;
                    map.insert("tags".to_string(), tags_to_value(metric.tags()));
                }

                _ => {}
            }
        }
    }

    map.into()
}

impl Target for MetricTarget {
    fn insert(&mut self, path: &OwnedTargetPath, value: Value) -> Result<(), ContextError> {
        match path.prefix() {
            PathPrefix::Metadata => {
                self.metric
                    .metadata_mut()
                    .value_mut()
                    .insert(path.value_path(), value);

                Ok(())
            }
            PathPrefix::Event => {
                let path = path.value_path();

                if path.is_root() {
                    return Err(ContextError::NotFound);
                }

                if let Some(paths) = path.to_alternative_components(3).get(0) {
                    match paths.as_slice() {
                        ["tags"] => {
                            if let Value::Object(map) = &value {
                                let mut tags = Tags::with_capacity(map.len());

                                for (key, value) in map {
                                    match value {
                                        Value::Integer(i) => tags.insert(key, *i),
                                        Value::Float(f) => tags.insert(key, *f),
                                        Value::Boolean(b) => tags.insert(key, *b),
                                        Value::Bytes(b) => {
                                            tags.insert(
                                                key,
                                                String::from_utf8_lossy(b).to_string(),
                                            );
                                        }
                                        _ => {
                                            return Err(ContextError::InvalidValue {
                                                expected: "integer, float, boolean or bytes",
                                            })
                                        }
                                    }
                                }

                                self.value.insert(path, value);
                                *self.metric.tags_mut() = tags;
                            } else {
                                return Err(ContextError::InvalidValue { expected: "map" });
                            }
                        }
                        ["tags", field] => {
                            match value {
                                Value::Integer(i) => self.metric.tags_mut().insert(*field, i),
                                Value::Float(f) => self.metric.tags_mut().insert(*field, f),
                                Value::Boolean(b) => self.metric.tags_mut().insert(*field, b),
                                Value::Bytes(b) => {
                                    let tv = String::from_utf8(b.into()).map_err(|_err| {
                                        ContextError::InvalidValue {
                                            expected: "valid utf8 string",
                                        }
                                    })?;

                                    self.metric.tags_mut().insert(*field, tv);
                                }
                                _ => {
                                    return Err(ContextError::InvalidValue {
                                        expected: "integer, float, boolean or string",
                                    })
                                }
                            }

                            return Ok(());
                        }
                        ["name"] => {}
                        ["timestamp"] => {}
                        ["kind"] => {}
                        _ => {
                            return Err(ContextError::InvalidPath {
                                expected: VALID_METRIC_PATHS_SET,
                            })
                        }
                    }
                }

                Ok(())
            }
        }
    }

    fn get(&mut self, path: &OwnedTargetPath) -> Result<Option<&Value>, ContextError> {
        match path.prefix() {
            PathPrefix::Event => {
                let path = path.value_path();
                let value = self.value.get(path);

                for paths in path.to_alternative_components(MAX_METRIC_PATH_DEPTH) {
                    match paths.as_slice() {
                        ["name"] | ["type"] | ["tags", _] => return Ok(value),
                        ["timestamp"] | ["tags"] => {
                            if let Some(value) = value {
                                return Ok(Some(value));
                            }
                        }
                        _ => return Err(ContextError::NotFound),
                    }
                }

                Ok(None)
            }
            PathPrefix::Metadata => Ok(self.metric.metadata().value().get(path.value_path())),
        }
    }

    fn get_mut(
        &mut self,
        target_path: &OwnedTargetPath,
    ) -> Result<Option<&mut Value>, ContextError> {
        match target_path.prefix() {
            PathPrefix::Event => {
                let path = target_path.value_path();
                if path.is_root() {
                    return Ok(Some(&mut self.value));
                }

                let value = self.value.get_mut(path);

                for paths in path.to_alternative_components(MAX_METRIC_PATH_DEPTH) {
                    match paths.as_slice() {
                        ["name"] | ["type"] | ["tags", _] => return Ok(value),
                        ["timestamp"] | ["tags"] => {
                            if let Some(value) = value {
                                return Ok(Some(value));
                            }
                        }
                        _ => {
                            return Err(ContextError::InvalidPath {
                                expected: VALID_METRIC_PATHS_SET,
                            })
                        }
                    }
                }

                // We only reach this point if we have requested a tag that
                // doesn't exist or an empty field.
                Ok(None)
            }
            PathPrefix::Metadata => {
                let value = self
                    .metric
                    .metadata_mut()
                    .value_mut()
                    .get_mut(target_path.value_path());

                Ok(value)
            }
        }
    }

    fn remove(
        &mut self,
        target_path: &OwnedTargetPath,
        compact: bool,
    ) -> Result<Option<Value>, ContextError> {
        match target_path.prefix() {
            PathPrefix::Event => {
                let path = target_path.value_path();

                if path.is_root() {
                    return Err(ContextError::InvalidPath {
                        expected: "non-root path",
                    });
                }

                if let Some(paths) = path.to_alternative_components(MAX_METRIC_PATH_DEPTH).get(0) {
                    match paths.as_slice() {
                        ["timestamp"] => {
                            self.metric.timestamp.take();
                        }
                        ["tags"] => {
                            *self.metric.tags_mut() = Tags::with_capacity(2);
                        }
                        ["tags", field] => {
                            self.metric.tags_mut().remove(field);
                        }
                        _ => {
                            return Err(ContextError::InvalidPath {
                                expected: VALID_METRIC_PATHS_SET,
                            })
                        }
                    }

                    return Ok(self.value.remove(path, false));
                }

                Ok(None)
            }
            PathPrefix::Metadata => {
                let removed = self
                    .metric
                    .metadata_mut()
                    .value_mut()
                    .remove(target_path.value_path(), compact);

                Ok(removed)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize() {
        let src = Source::Script("foo".to_string());
        let data = serde_json::to_string(&src).unwrap();
        assert_eq!(data, r#"{"script":"foo"}"#);
    }

    #[ignore]
    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }
}
