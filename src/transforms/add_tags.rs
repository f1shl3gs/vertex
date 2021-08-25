use serde::{Deserialize, Serialize};
use indexmap::IndexMap;
use async_trait::async_trait;

use crate::config::{TransformConfig, GlobalOptions, DataType};
use crate::transforms::{Transform, FunctionTransform};
use crate::event::Event;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AddTagsConfig {
    pub tags: IndexMap<String, String>,

    #[serde(default = "default_overwrite")]
    pub overwrite: bool,
}

pub fn default_overwrite() -> bool {
    true
}

#[derive(Clone, Debug)]
pub struct AddTags {
    tags: IndexMap<String, String>,
    overwrite: bool,
}

impl AddTags {
    pub fn new(tags: IndexMap<String, String>, overwrite: bool) -> Self {
        AddTags { tags, overwrite }
    }
}

impl FunctionTransform for AddTags {
    fn transform(&mut self, output: &mut Vec<Event>, mut event: Event) {
        if self.tags.is_empty() {
            return;
        }

        let metric = event.as_mut_metric();
        for (name, value) in &self.tags {

        }
    }
}

#[async_trait]
#[typetag::serde(name = "add_tags")]
impl TransformConfig for AddTagsConfig {
    async fn build(&self, globals: &GlobalOptions) -> crate::Result<Transform> {
        Ok(Transform::function(AddTags::new(
            self.tags.clone(),
            self.overwrite,
        )))
    }

    fn input_type(&self) -> DataType {
        DataType::Metric
    }

    fn output_type(&self) -> DataType {
        DataType::Metric
    }

    fn transform_type(&self) -> &'static str {
        "add_tags"
    }
}

