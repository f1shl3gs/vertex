use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use crate::config::{TransformConfig, GlobalOptions, DataType};
use crate::transforms::{Transform, FunctionTransform};
use event::Event;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AddTagsConfig {
    pub tags: BTreeMap<String, String>,

    #[serde(default = "default_overwrite")]
    pub overwrite: bool,
}

pub fn default_overwrite() -> bool {
    true
}

#[derive(Clone, Debug)]
pub struct AddTags {
    tags: BTreeMap<String, String>,
    overwrite: bool,
}

impl AddTags {
    pub fn new(tags: BTreeMap<String, String>, overwrite: bool) -> Self {
        AddTags { tags, overwrite }
    }
}

impl FunctionTransform for AddTags {
    fn transform(&mut self, output: &mut Vec<Event>, mut event: Event) {
        if self.tags.is_empty() {
            return;
        }

        match event {
            Event::Metric(ref mut metric) => {
                merge_tags(&self.tags, &mut metric.tags, self.overwrite);

                output.push(event);
            }

            Event::Log(ref mut log) => {
                merge_tags(&self.tags, &mut log.tags, self.overwrite);
                output.push(event);
            }
        }
    }
}

fn merge_tags(from: &BTreeMap<String, String>, to: &mut BTreeMap<String, String>, overwrite: bool) {
    if overwrite {
        for (k, v) in from {
            to.insert(k.clone(), v.clone());
        }

        return;
    }

    for (k, v) in from {
        if to.contains_key(k) {
            continue;
        }

        to.insert(k.clone(), v.clone());
    }
}

#[async_trait]
#[typetag::serde(name = "add_tags")]
impl TransformConfig for AddTagsConfig {
    async fn build(&self, _: &GlobalOptions) -> crate::Result<Transform> {
        Ok(Transform::function(AddTags::new(
            self.tags.clone(),
            self.overwrite,
        )))
    }

    fn input_type(&self) -> DataType {
        DataType::Any
    }

    fn output_type(&self) -> DataType {
        DataType::Any
    }

    fn transform_type(&self) -> &'static str {
        "add_tags"
    }
}

#[cfg(test)]
mod tests {
    use event::{Metric, tags};
    use crate::transforms::transform_one;
    use super::*;

    #[test]
    fn add_tags() {
        let metric = Metric::sum_with_tags(
            "foo",
            "",
            1,
            tags!(
                "k1" => "v1"
            )
        );

        let m = tags!(
            "k1" => "v1_new",
            "k2" => "v2"
        );
        let mut transform = AddTags::new(m, false);

        let event = transform_one(&mut transform, metric);
        assert_eq!(
            event.unwrap(),
            Metric::sum_with_tags(
                "foo",
                "",
                1,
                tags!(
                    "k1" => "v1",
                    "k2" => "v2"
                )
            ).into()
        )
    }

    #[test]
    fn add_tags_overwrite() {
        let metric = Metric::sum_with_tags(
            "foo",
            "",
            1,
            tags!(
                "k1" => "v1"
            )
        );
        let m = tags!(
            "k1" => "v1_new",
            "k2" => "v2"
        );
        let mut transform = AddTags::new(m, true);
        let event = transform_one(&mut transform, metric);
        assert_eq!(
            event.unwrap(),
            Metric::sum_with_tags(
                "foo",
                "",
                1,
                tags!(
                    "k1" => "v1_new",
                    "k2" => "v2"
                )
            ).into()
        );
    }
}