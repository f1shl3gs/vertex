use async_trait::async_trait;
use bytes::Buf;
use event::{log::Value, Events};
use framework::config::{
    DataType, GenerateConfig, Output, TransformConfig, TransformContext, TransformDescription,
};
use framework::{FunctionTransform, OutputBuffer, Transform};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SubstrConfig {
    field: String,
    #[serde(default)]
    offset: Option<usize>,
    #[serde(default)]
    length: Option<usize>,
}

impl GenerateConfig for SubstrConfig {
    fn generate_config() -> String {
        r#"
# Substr works like "Bash's Substring", e.g. "${variable:4:6}"

# Which field to transform.
#
field: some.value

# Offset from start, count from zero.
#
# offset: 0

# Lenght from offset, keeping the first `length` bytes and dropping the
# rest. If `length` is greater than the bytes's current length, this has no
# effect.
#
length: 10
"#
        .to_string()
    }
}

inventory::submit! {
    TransformDescription::new::<SubstrConfig>("substr")
}

#[async_trait]
#[typetag::serde(name = "substr")]
impl TransformConfig for SubstrConfig {
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

    fn transform_type(&self) -> &'static str {
        "substr"
    }
}

#[derive(Clone)]
struct Substr {
    field: String,
    offset: Option<usize>,
    length: Option<usize>,
}

impl FunctionTransform for Substr {
    fn transform(&mut self, output: &mut OutputBuffer, mut events: Events) {
        events.for_each_log(|log| {
            if let Some(Value::Bytes(value)) = log.get_field_mut(self.field.as_str()) {
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
    use super::*;
    use crate::transforms::transform_one;
    use event::{assert_event_data_eq, fields, Event};

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<SubstrConfig>();
    }

    #[test]
    fn substr() {
        let tests = [
            // name, input, config, want
            (
                "without_offset_and_length",
                fields!("value" => "some_value"),
                Substr {
                    field: "value".to_string(),
                    offset: None,
                    length: None,
                },
                fields!("value" => "some_value"),
            ),
            (
                "with_offset",
                fields!("value" => "some_value"),
                Substr {
                    field: "value".to_string(),
                    offset: Some(5),
                    length: None,
                },
                fields!("value" => "value"),
            ),
            (
                "with_length",
                fields!("value" => "some_value"),
                Substr {
                    field: "value".to_string(),
                    offset: None,
                    length: Some(4),
                },
                fields!("value" => "some"),
            ),
            (
                "with_offset_and_length",
                fields!("value" => "some_value"),
                Substr {
                    field: "value".to_string(),
                    offset: Some(5),
                    length: Some(3),
                },
                fields!("value" => "val"),
            ),
            (
                "offset_large_than_input_length",
                fields!("value" => "some_value"),
                Substr {
                    field: "value".to_string(),
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
