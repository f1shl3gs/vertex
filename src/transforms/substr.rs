use async_trait::async_trait;
use bytes::Buf;
use configurable::configurable_component;
use event::log::OwnedTargetPath;
use event::{log::Value, Events};
use framework::config::{DataType, Output, TransformConfig, TransformContext};
use framework::{FunctionTransform, OutputBuffer, Transform};

#[configurable_component(transform, name = "substr")]
#[serde(deny_unknown_fields)]
struct Config {
    /// Which field to transform.
    #[configurable(required, example = ".some_field")]
    field: OwnedTargetPath,

    /// Offset from start, count from zero.
    #[serde(default)]
    offset: Option<usize>,

    /// Length from offset, keeping the first `length` bytes and dropping the
    /// rest. If `length` is greater than the bytes's current length, this has no
    /// effect.
    #[serde(default)]
    length: Option<usize>,
}

#[async_trait]
#[typetag::serde(name = "substr")]
impl TransformConfig for Config {
    async fn build(&self, _cx: &TransformContext) -> framework::Result<Transform> {
        Ok(Transform::function(Substr {
            field: self.field.clone(),
            offset: self.offset,
            length: self.length,
        }))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }
}

#[derive(Clone)]
struct Substr {
    field: OwnedTargetPath,
    offset: Option<usize>,
    length: Option<usize>,
}

impl FunctionTransform for Substr {
    fn transform(&mut self, output: &mut OutputBuffer, mut events: Events) {
        events.for_each_log(|log| {
            if let Some(Value::Bytes(value)) = log.get_mut(&self.field) {
                if let Some(offset) = self.offset {
                    let offset = value.remaining().min(offset);
                    value.advance(offset);
                }

                if let Some(length) = self.length {
                    value.truncate(length);
                }
            }
        });

        output.push(events);
    }
}

#[cfg(test)]
mod tests {
    use event::log::path::parse_target_path;
    use event::{fields, Event};
    use testify::assert_event_data_eq;

    use super::*;
    use crate::transforms::transform_one;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>();
    }

    #[test]
    fn substr() {
        let tests = [
            // name, input, config, want
            (
                "without_offset_and_length",
                fields!("value" => "some_value"),
                Substr {
                    field: parse_target_path("value").unwrap(),
                    offset: None,
                    length: None,
                },
                fields!("value" => "some_value"),
            ),
            (
                "with_offset",
                fields!("value" => "some_value"),
                Substr {
                    field: parse_target_path("value").unwrap(),
                    offset: Some(5),
                    length: None,
                },
                fields!("value" => "value"),
            ),
            (
                "with_length",
                fields!("value" => "some_value"),
                Substr {
                    field: parse_target_path("value").unwrap(),
                    offset: None,
                    length: Some(4),
                },
                fields!("value" => "some"),
            ),
            (
                "with_offset_and_length",
                fields!("value" => "some_value"),
                Substr {
                    field: parse_target_path("value").unwrap(),
                    offset: Some(5),
                    length: Some(3),
                },
                fields!("value" => "val"),
            ),
            (
                "offset_large_than_input_length",
                fields!("value" => "some_value"),
                Substr {
                    field: parse_target_path("value").unwrap(),
                    offset: Some(20),
                    length: None,
                },
                fields!("value" => ""),
            ),
        ];

        for (name, input, mut config, want) in tests {
            let event = transform_one(&mut config, Event::from(input)).unwrap();
            assert_event_data_eq!(event, Event::from(want), name);
        }
    }
}
