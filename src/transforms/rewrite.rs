use async_trait::async_trait;
use configurable::{configurable_component, Configurable};
use event::array::EventMutRef;
use event::tags::{Key, Tags, Value};
use event::Events;
use framework::config::{DataType, Output, TransformConfig, TransformContext};
use framework::{FunctionTransform, OutputBuffer, Transform};
use serde::{Deserialize, Serialize};

#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum Operation {
    Set { key: Key, value: Value },
    Add { key: Key, value: Value },
    Delete { key: Key },
    Rename { key: Key, new: Key },
}

impl Operation {
    fn apply(&self, tags: &mut Tags) {
        match self {
            Operation::Set { key, value } => tags.insert(key.clone(), value.clone()),
            Operation::Add { key, value } => {
                if !tags.contains_key(key) {
                    tags.insert(key.clone(), value.clone());
                }
            }
            Operation::Delete { key } => {
                tags.remove(key);
            }
            Operation::Rename { key, new } => {
                if let Some(value) = tags.remove(key) {
                    tags.insert(new.clone(), value);
                }
            }
        }
    }
}

#[configurable_component(transform, name = "rewrite")]
struct Config {
    operations: Vec<Operation>,
}

#[async_trait]
#[typetag::serde(name = "rewrite")]
impl TransformConfig for Config {
    async fn build(&self, _cx: &TransformContext) -> crate::Result<Transform> {
        if self.operations.is_empty() {
            return Err("At least one operation required".into());
        }

        Ok(Transform::function(Rewrite {
            operations: self.operations.clone(),
        }))
    }

    fn input_type(&self) -> DataType {
        DataType::All
    }

    fn outputs(&self) -> Vec<Output> {
        vec![
            Output::default(DataType::Metric),
            Output::default(DataType::Trace),
        ]
    }
}

#[derive(Clone)]
struct Rewrite {
    operations: Vec<Operation>,
}

impl FunctionTransform for Rewrite {
    fn transform(&mut self, output: &mut OutputBuffer, mut events: Events) {
        events.for_each_event(|event| {
            let tags = match event {
                EventMutRef::Metric(log) => log.tags_mut(),
                EventMutRef::Trace(trace) => &mut trace.tags,
                _ => unreachable!(),
            };

            for op in &self.operations {
                op.apply(tags);
            }
        });

        output.push(events)
    }
}

#[cfg(test)]
mod tests {
    use event::{tags, Event, Metric};

    use super::*;
    use crate::transforms::transform_one;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>();
    }

    fn assert_transform(op: Operation, tags: Tags) {
        let event: Event = Metric::sum_with_tags(
            "foo",
            "",
            1,
            tags!(
                "k1" => "v1"
            ),
        )
        .into();

        let mut rewrite = Rewrite {
            operations: vec![op],
        };

        let event = transform_one(&mut rewrite, event).expect("transform should success");

        assert_eq!(event.into_metric().tags(), &tags);
    }

    #[test]
    fn add() {
        let op = Operation::Add {
            key: "k2".into(),
            value: "v2".into(),
        };

        assert_transform(
            op,
            tags!(
                "k1" => "v1",
                "k2" => "v2",
            ),
        );
    }

    #[test]
    fn add_failed() {
        let op = Operation::Add {
            key: "k1".into(),
            value: "v1".into(),
        };

        assert_transform(
            op,
            tags!(
                "k1" => "v1",
            ),
        );
    }

    #[test]
    fn set() {
        let op = Operation::Set {
            key: "k1".into(),
            value: "v2".into(),
        };

        assert_transform(
            op,
            tags!(
                "k1" => "v2"
            ),
        )
    }

    #[test]
    fn delete() {
        let op = Operation::Delete { key: "k1".into() };

        assert_transform(op, tags!());
    }

    #[test]
    fn rename() {
        let op = Operation::Rename {
            key: "k1".into(),
            new: "k2".into(),
        };

        assert_transform(
            op,
            tags!(
                "k2" => "v1"
            ),
        )
    }
}
