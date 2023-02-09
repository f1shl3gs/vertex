use std::collections::HashMap;

use configurable::configurable_component;
use event::Events;
use framework::config::{default_true, Output, TransformContext};
use framework::config::{DataType, TransformConfig};
use framework::{FunctionTransform, OutputBuffer, Transform};

#[configurable_component(transform, name = "add_fields")]
#[derive(Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct AddFieldsConfig {
    #[configurable(required)]
    pub fields: HashMap<String, String>,

    #[serde(default = "default_true")]
    pub overwrite: bool,
}

#[async_trait::async_trait]
#[typetag::serde(name = "add_fields")]
impl TransformConfig for AddFieldsConfig {
    async fn build(&self, _cx: &TransformContext) -> crate::Result<Transform> {
        if self.fields.is_empty() {
            return Err("fields is required".into());
        }

        Ok(Transform::function(AddFields::from(self)))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }
}

#[derive(Clone, Debug)]
struct AddFields {
    fields: HashMap<String, String>,
    overwrite: bool,
}

impl AddFields {
    fn from(conf: &AddFieldsConfig) -> Self {
        Self {
            fields: conf.fields.clone(),
            overwrite: conf.overwrite,
        }
    }
}

impl FunctionTransform for AddFields {
    fn transform(&mut self, output: &mut OutputBuffer, mut events: Events) {
        events.for_each_log(|log| {
            for (path, v) in self.fields.iter() {
                if log.fields.contains(path.as_str()) && !self.overwrite {
                    continue;
                }

                log.fields.insert(path.as_str(), v.to_string());
            }
        });

        output.push(events)
    }
}
