use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use event::Event;
use framework::config::{
    DataType, GenerateConfig, Output, TransformConfig, TransformContext, TransformDescription,
};
use framework::{FunctionTransform, Transform};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AddTagsConfig {
    pub tags: BTreeMap<String, String>,

    #[serde(default = "default_overwrite")]
    pub overwrite: bool,
}

const fn default_overwrite() -> bool {
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
    async fn build(&self, _ctx: &TransformContext) -> crate::Result<Transform> {
        Ok(Transform::function(AddTags::new(
            self.tags.clone(),
            self.overwrite,
        )))
    }

    fn input_type(&self) -> DataType {
        DataType::Any
    }

    fn outputs(&self) -> Vec<Output> {
        vec![
            Output::default(DataType::Metric),
            Output::default(DataType::Log),
        ]
    }

    fn transform_type(&self) -> &'static str {
        "add_tags"
    }
}

inventory::submit! {
    TransformDescription::new::<AddTagsConfig>("add_tags")
}

impl GenerateConfig for AddTagsConfig {
    fn generate_config() -> String {
        r#"
# Tags add to the event
tags:
  foo: bar
  host: ${HOSTNAME}

# Controls how tag conflicts are handled if the event has tags that
# Vertex would add.
#
# overwrite: false

"#
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transforms::transform_one;
    use event::{tags, Metric};

    #[test]
    fn add_tags() {
        let metric = Metric::sum_with_tags(
            "foo",
            "",
            1,
            tags!(
                "k1" => "v1"
            ),
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
                ),
            )
            .into()
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
            ),
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
                ),
            )
            .into()
        );
    }
}
