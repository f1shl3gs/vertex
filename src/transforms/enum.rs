use async_trait::async_trait;
use event::log::Value;
use event::Events;
use framework::config::{
    DataType, GenerateConfig, Output, TransformConfig, TransformContext, TransformDescription,
};
use framework::{FunctionTransform, OutputBuffer, Transform};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct MappingItem {
    key: Value,
    value: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Config {
    source: String,
    target: String,

    mapping: Vec<MappingItem>,
}

impl GenerateConfig for Config {
    fn generate_config() -> String {
        r##"
# source is the filed to evaluate
#
source: foo

# target the field to store mapped value
#
target: bar

# mapping table
mapping:
  - key: 0
    value: success
  - key: 1
    value: error

"##
        .to_string()
    }
}

inventory::submit! {
    TransformDescription::new::<Config>("enum")
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

    fn transform_type(&self) -> &'static str {
        "enum"
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
            if let Some(got) = log.get_field(&self.source) {
                for MappingItem { key, value } in &self.mapping {
                    if key == got {
                        log.insert_field(self.target.clone(), value.clone());
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
    fn test_generate_config() {
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
