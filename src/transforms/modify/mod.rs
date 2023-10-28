mod action;

use action::Action;
use async_trait::async_trait;
use condition::Expression;
use configurable::{configurable_component, Configurable};
use event::Events;
use framework::config::{DataType, Output, TransformConfig, TransformContext};
use framework::{FunctionTransform, OutputBuffer, Transform};
use serde::{Deserialize, Serialize};

/// ErrorMode determines how this transformer reacts to errors.
#[derive(Clone, Configurable, Debug, Default, Deserialize, Serialize, PartialEq)]
enum ErrorMode {
    /// Drop the event, and write a log.
    Drop,

    /// Skip this error, and continue to transform the event.
    #[default]
    Continue,
}

#[configurable_component(transform, name = "modify")]
struct Config {
    /// The condition to be matched against every input event. Only events that pass
    /// the condition are transformed; messages that don’t pass are not.
    condition: Option<Expression>,

    #[serde(default)]
    error_mode: ErrorMode,

    #[configurable(skip)]
    #[serde(default)]
    actions: Vec<Action>,
}

#[async_trait]
#[typetag::serde(name = "modify")]
impl TransformConfig for Config {
    async fn build(&self, _cx: &TransformContext) -> framework::Result<Transform> {
        let modifier = Modifier {
            error_mode: self.error_mode.clone(),
            condition: self.condition.clone(),
            actions: self.actions.clone(),
        };

        Ok(Transform::function(modifier))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }
}

#[derive(Clone)]
struct Modifier {
    error_mode: ErrorMode,
    condition: Option<Expression>,
    actions: Vec<Action>,
}

impl FunctionTransform for Modifier {
    fn transform(&mut self, output: &mut OutputBuffer, events: Events) {
        if let Events::Logs(logs) = events {
            'outer: for mut log in logs {
                if let Some(cond) = &self.condition {
                    match cond.eval(&log) {
                        Ok(result) => {
                            if !result {
                                output.push_one(log.into());
                                return;
                            }
                        }
                        Err(err) => {
                            warn!(
                                message = "condition match failed",
                                ?err,
                                internal_log_rate_limit = true
                            );

                            continue;
                        }
                    }
                }

                for action in &self.actions {
                    if let Err(err) = action.apply(&mut log) {
                        if self.error_mode == ErrorMode::Drop {
                            warn!(
                                message = "event transform failed",
                                ?err,
                                internal_log_rate_limit = true
                            );

                            break 'outer;
                        }
                    }
                }

                output.push_one(log.into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use event::log::path::parse_target_path;
    use event::log::Value;
    use event::{fields, Event, LogRecord};

    use super::*;
    use crate::transforms::transform_one;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }

    #[test]
    fn transform() {
        let tests = [
            // Set
            (
                "set field",
                None,
                vec![Action::Set {
                    path: parse_target_path(".foo").unwrap(),
                    value: "bar".into(),
                }],
                fields!(
                    "key" => "value",
                    "foo" => "bar"
                ),
            ),
            (
                "set field with condition",
                Some(Expression::parse(".key contains val").unwrap()),
                vec![Action::Set {
                    path: parse_target_path(".foo").unwrap(),
                    value: "bar".into(),
                }],
                fields!(
                    "key" => "value",
                    "foo" => "bar"
                ),
            ),
            (
                "set field with condition but not match",
                Some(Expression::parse(".key contains foo").unwrap()),
                vec![Action::Set {
                    path: parse_target_path(".foo").unwrap(),
                    value: "bar".into(),
                }],
                fields!(
                    "key" => "value",
                ),
            ),
        ];

        for (name, condition, actions, want) in tests {
            let mut modifier = Modifier {
                condition,
                actions,
                error_mode: ErrorMode::Continue,
            };

            let event = Event::from(LogRecord::from(fields!(
                "key" => "value"
            )));

            let got = transform_one(&mut modifier, event).expect(name);
            assert_eq!(
                got.into_log().value(),
                &Value::from(want),
                "\nTest \"{name}\" failed"
            )
        }
    }
}
