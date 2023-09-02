use std::fmt::Formatter;

use async_trait::async_trait;
use bytes::Bytes;
use configurable::{configurable_component, Configurable};
use event::Events;
use framework::config::{DataType, Output, TransformConfig, TransformContext};
use framework::{FunctionTransform, OutputBuffer, Transform};
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Configurable, Clone, Debug)]
enum Value {
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Delegate;

        impl<'de> serde::de::Visitor<'de> for Delegate {
            type Value = Value;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str(r#"integer, float, string or bool expect"#)
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Value::Bool(v))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Value::Integer(v))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Value::String(v.to_string()))
            }
        }

        deserializer.deserialize_any(Delegate)
    }
}

impl Serialize for Value {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        todo!()
    }
}

impl From<&Value> for event::log::Value {
    fn from(value: &Value) -> Self {
        match value {
            Value::Integer(i) => event::log::Value::Int64(*i),
            Value::Float(f) => event::log::Value::Float(*f),
            Value::String(s) => event::log::Value::Bytes(Bytes::from(s.to_owned())),
            Value::Bool(b) => event::log::Value::Boolean(*b),
        }
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

#[derive(Configurable, Clone, Debug, Deserialize, Serialize)]
struct MappingItem {
    /// Key value could be integer, float, string or boolean
    #[configurable(required)]
    key: Value,

    /// Value's value could be integer, float, string or boolean
    #[configurable(required)]
    value: Value,
}

#[configurable_component(transform, name = "enum")]
#[derive(Clone)]
#[serde(deny_unknown_fields)]
struct Config {
    /// source is the filed to evaluate.
    #[configurable(required)]
    source: String,

    /// target the field to store mapped value
    #[configurable(required)]
    target: String,

    /// mapping table
    #[configurable(required)]
    mapping: Vec<MappingItem>,
}

#[async_trait]
#[typetag::serde(name = "enum")]
impl TransformConfig for Config {
    async fn build(&self, _cx: &TransformContext) -> framework::Result<Transform> {
        let transform = Enum {
            source: self.source.clone(),
            target: self.target.clone(),
            mapping: self.mapping.clone(),
        };

        Ok(Transform::function(transform))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }
}

#[derive(Clone)]
struct Enum {
    source: String,
    target: String,
    mapping: Vec<MappingItem>,
}

impl FunctionTransform for Enum {
    fn transform(&mut self, output: &mut OutputBuffer, mut events: Events) {
        events.for_each_log(|log| {
            if let Some(got) = log.get_field(self.source.as_str()) {
                for MappingItem { key, value } in &self.mapping {
                    let equal = match (key, got) {
                        (Value::Integer(ai), event::log::Value::Int64(bi)) => ai == bi,
                        (Value::Float(af), event::log::Value::Float(bf)) => af == bf,
                        (Value::String(astr), event::log::Value::Bytes(bs)) => astr == bs,
                        (Value::Bool(ab), event::log::Value::Boolean(bb)) => ab == bb,
                        _ => false,
                    };

                    if equal {
                        let n: event::log::Value = From::from(value);
                        log.insert_field(self.target.as_str(), n);
                        return;
                    }
                }
            }
        });

        output.push(events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transforms::transform_one;
    use event::{fields, Event};

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>();
    }

    #[test]
    fn transform() {
        let tests = [
            (
                "number to string",
                fields!(
                    "source" => 0,
                ),
                Config {
                    source: "source".into(),
                    target: "target".into(),
                    mapping: vec![
                        MappingItem {
                            key: 0.into(),
                            value: "success".into(),
                        },
                        MappingItem {
                            key: 1.into(),
                            value: "failed".into(),
                        },
                    ],
                },
                fields!(
                    "source" => 0,
                    "target" => "success"
                ),
            ),
            (
                "key not found", // name
                fields!(            // input
                    "other_source" => 0,
                ),
                Config {
                    source: "source".into(),
                    target: "target".into(),
                    mapping: vec![
                        MappingItem {
                            key: 0.into(),
                            value: "success".into(),
                        },
                        MappingItem {
                            key: 1.into(),
                            value: "failed".into(),
                        },
                    ],
                },
                fields!(
                    "other_source" => 0,
                ),
            ),
            (
                "string to number",
                fields!(
                    "source" => "success"
                ),
                Config {
                    source: "source".into(),
                    target: "target".into(),
                    mapping: vec![MappingItem {
                        key: "success".into(),
                        value: 0.into(),
                    }],
                },
                fields!(
                    "source" => "success",
                    "target" => 0,
                ),
            ),
            (
                "overwrite",
                fields!(
                    "source" => "success",
                ),
                Config {
                    source: "source".into(),
                    target: "source".into(),
                    mapping: vec![MappingItem {
                        key: "success".into(),
                        value: 0.into(),
                    }],
                },
                fields!(
                    "source" => 0
                ),
            ),
            (
                "source not found",
                fields!(
                    "foo" => "bar",
                ),
                Config {
                    source: "source".into(),
                    target: "target".into(),
                    mapping: vec![MappingItem {
                        key: "success".into(),
                        value: 0.into(),
                    }],
                },
                fields!(
                    "foo" => "bar"
                ),
            ),
        ];

        for (name, input, config, want) in tests {
            let mut transform = Enum {
                source: config.source,
                target: config.target,
                mapping: config.mapping.clone(),
            };

            let event = Event::from(input);

            let got = transform_one(&mut transform, event).unwrap();
            assert_eq!(got, Event::from(want), "testcase {}", name)
        }
    }
}
