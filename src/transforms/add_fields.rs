use crate::{
    config::{default_true}
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use event::Event;
use crate::config::{DataType, GlobalOptions, TransformConfig};
use crate::transforms::{FunctionTransform, Transform};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AddFieldsConfig {
    pub fields: IndexMap<String, String>,

    #[serde(default = "default_true")]
    pub overwrite: bool,
}

#[async_trait::async_trait]
#[typetag::serde(name = "add_fields")]
impl TransformConfig for AddFieldsConfig {
    async fn build(&self, _globals: &GlobalOptions) -> crate::Result<Transform> {
        Ok(Transform::function(AddFields::from(self)))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn output_type(&self) -> DataType {
        DataType::Log
    }

    fn transform_type(&self) -> &'static str {
        "add_fields"
    }
}

#[derive(Clone, Debug)]
struct AddFields {
    fields: IndexMap<String, String>,
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
    fn transform(&mut self, output: &mut Vec<Event>, mut event: Event) {
        if self.fields.is_empty() {
            return;
        }

        let log = event.as_mut_log();
        for (k, v) in self.fields.iter() {
            log.fields.insert(k.clone(), v.as_str().into());
        }

        output.push(event)
    }
}
