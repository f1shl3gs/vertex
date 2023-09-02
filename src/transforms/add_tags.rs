use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

use async_trait::async_trait;
use configurable::configurable_component;
use event::Events;
use framework::config::{default_true, DataType, Output, TransformConfig, TransformContext};
use framework::{FunctionTransform, OutputBuffer, Transform};

#[configurable_component(transform, name = "add_tags")]
#[derive(Clone)]
#[serde(deny_unknown_fields)]
pub struct AddTagsConfig {
    /// Tags add to the event.
    #[configurable(required)]
    pub tags: BTreeMap<String, String>,

    /// Controls how tag conflicts are handled if the event has tags that
    /// Vertex would add.
    #[serde(default = "default_true")]
    pub overwrite: bool,
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
    fn transform(&mut self, output: &mut OutputBuffer, mut events: Events) {
        events.for_each_event(|event| {
            let attrs = event.tags();

            for (key, value) in &self.tags {
                match (attrs.entry(key), self.overwrite) {
                    (Entry::Vacant(entry), _) => {
                        entry.insert(value.into());
                    }
                    (Entry::Occupied(mut entry), true) => {
                        entry.insert(value.into());
                    }
                    (Entry::Occupied(_entry), false) => {}
                }
            }
        });

        output.push(events)
    }
}

#[async_trait]
#[typetag::serde(name = "add_tags")]
impl TransformConfig for AddTagsConfig {
    async fn build(&self, _cx: &TransformContext) -> crate::Result<Transform> {
        if self.tags.is_empty() {
            return Err("At least one key/value pair required".into());
        }

        Ok(Transform::function(AddTags::new(
            self.tags.clone(),
            self.overwrite,
        )))
    }

    fn input_type(&self) -> DataType {
        DataType::All
    }

    fn outputs(&self) -> Vec<Output> {
        vec![
            Output::default(DataType::Metric),
            Output::default(DataType::Log),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transforms::transform_one;
    use event::{btreemap, tags, Metric};

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<AddTagsConfig>();
    }

    #[test]
    fn add_tags_without_overwrite() {
        let metric = Metric::sum_with_tags(
            "foo",
            "",
            1,
            tags!(
                "k1" => "v1"
            ),
        );

        let m = btreemap!(
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
        let m = btreemap!(
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
