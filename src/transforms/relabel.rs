use configurable::{Configurable, configurable_component};
use event::Events;
use event::array::EventMutRef;
use event::tags::{Key, Tags, Value};
use framework::config::{
    DataType, InputType, OutputType, TransformConfig, TransformContext, serde_regex,
};
use framework::{FunctionTransform, OutputBuffer, Transform};
use md5::{Digest, Md5};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Clone, Configurable, Debug, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "lowercase")]
enum Operation {
    Set {
        key: Key,
        value: Value,
    },
    Add {
        key: Key,
        value: Value,
    },
    Delete {
        key: Key,
    },
    Rename {
        key: Key,
        new: Key,
    },
    /// Maps the concatenated source_labels to their lower case.
    Lowercase {
        target: Key,
    },
    /// Maps the concatenated source_labels to their upper case.
    Uppercase {
        target: Key,
    },
    HashMod {
        source: Key,
        target: Option<Key>,
        modules: u64,
    },
    LabelDrop {
        #[serde(with = "serde_regex")]
        regex: Regex,
    },
    LabelKeep {
        #[serde(with = "serde_regex")]
        regex: Regex,
    },
}

impl Operation {
    fn apply(&self, tags: &mut Tags) {
        match self {
            Operation::Set { key, value } => tags.insert(key.clone(), value.clone()),
            Operation::Add { key, value } => {
                if !tags.contains(key) {
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

            Operation::Lowercase { target } => {
                if let Some(Value::String(s)) = tags.get_mut(target) {
                    *s = s.to_lowercase();
                }
            }
            Operation::Uppercase { target } => {
                if let Some(Value::String(s)) = tags.get_mut(target) {
                    *s = s.to_uppercase();
                }
            }
            Operation::HashMod {
                source,
                target,
                modules,
            } => {
                if let Some(value) = tags.get(source) {
                    let s = value.to_string_lossy();
                    let mut hasher = Md5::new();
                    hasher.update(s.as_bytes());
                    let result = hasher.finalize()[8..].try_into().expect("must success");
                    let m = (<u64>::from_be_bytes(result) % modules) as i64;
                    match target {
                        Some(target) => tags.insert(target.clone(), m),
                        None => tags.insert(source.clone(), m),
                    }
                }
            }
            Operation::LabelDrop { regex } => {
                tags.retain(|key, _value| !regex.is_match(key.as_str()))
            }
            Operation::LabelKeep { regex } => {
                tags.retain(|key, _value| regex.is_match(key.as_str()))
            }
        }
    }
}

#[configurable_component(transform, name = "relabel")]
struct Config {
    operations: Vec<Operation>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "relabel")]
impl TransformConfig for Config {
    async fn build(&self, _cx: &TransformContext) -> crate::Result<Transform> {
        if self.operations.is_empty() {
            return Err("At least one operation required".into());
        }

        Ok(Transform::function(Relabel {
            operations: self.operations.clone(),
        }))
    }

    fn input(&self) -> InputType {
        InputType::new(DataType::Metric | DataType::Trace)
    }

    fn outputs(&self) -> Vec<OutputType> {
        vec![OutputType::metric(), OutputType::trace()]
    }

    fn enable_concurrency(&self) -> bool {
        true
    }
}

#[derive(Clone)]
struct Relabel {
    operations: Vec<Operation>,
}

impl FunctionTransform for Relabel {
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
    use event::{Event, Metric, tags};

    use super::*;
    use crate::transforms::transform_one;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }

    fn assert_transform(input: Tags, op: Operation, want: Tags) {
        let event: Event = Metric::sum_with_tags("foo", "", 1, input).into();

        let mut relabel = Relabel {
            operations: vec![op],
        };

        let event = transform_one(&mut relabel, event).expect("transform should success");

        assert_eq!(event.into_metric().tags(), &want);
    }

    #[test]
    fn add() {
        let op = Operation::Add {
            key: "k2".into(),
            value: "v2".into(),
        };

        let input = tags!(
            "k1" => "v1"
        );

        assert_transform(
            input,
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

        let input = tags!(
            "k1" => "v1"
        );

        assert_transform(
            input,
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

        let input = tags!(
            "k1" => "v1"
        );

        assert_transform(
            input,
            op,
            tags!(
                "k1" => "v2"
            ),
        )
    }

    #[test]
    fn delete() {
        let op = Operation::Delete { key: "k1".into() };

        let input = tags!(
            "k1" => "v1"
        );

        assert_transform(input, op, tags!());
    }

    #[test]
    fn rename() {
        let op = Operation::Rename {
            key: "k1".into(),
            new: "k2".into(),
        };

        let input = tags!(
            "k1" => "v1"
        );

        assert_transform(
            input,
            op,
            tags!(
                "k2" => "v1"
            ),
        )
    }

    #[test]
    fn lowercase() {
        let op = Operation::Lowercase {
            target: "k1".into(),
        };

        let input = tags!(
            "k1" => "VVV"
        );

        assert_transform(
            input,
            op,
            tags!(
                "k1" => "vvv"
            ),
        )
    }

    #[test]
    fn uppercase() {
        let op = Operation::Uppercase {
            target: "k1".into(),
        };

        let input = tags!(
            "k1" => "v1v"
        );

        assert_transform(
            input,
            op,
            tags!(
                "k1" => "V1V"
            ),
        )
    }

    #[test]
    fn hashmod() {
        let op = Operation::HashMod {
            source: "c".into(),
            target: None,
            modules: 1000,
        };

        let input = tags!(
            "c" => "baz"
        );

        assert_transform(
            input,
            op,
            tags!(
                "c" => 976
            ),
        )
    }

    #[test]
    fn labeldrop() {
        let op = Operation::LabelDrop {
            regex: Regex::new(r#"(b.*)"#).unwrap(),
        };
        let input = tags!(
            "a" =>  "foo",
            "b1" => "bar",
            "b2" => "baz",
        );

        assert_transform(
            input,
            op,
            tags!(
                "a" => "foo"
            ),
        )
    }
}
