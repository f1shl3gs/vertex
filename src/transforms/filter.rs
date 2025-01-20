use async_trait::async_trait;
use configurable::configurable_component;
use event::Events;
use framework::config::{DataType, Output, TransformConfig, TransformContext};
use framework::{FunctionTransform, OutputBuffer, Transform};
use metrics::Counter;
use value::Value;
use vtl::{Diagnostic, Program};

use crate::common::vtl::LogTarget;

/// Filters events based on a set of conditions.
#[configurable_component(transform, name = "filter")]
#[serde(deny_unknown_fields)]
struct Config {
    /// The condition to be matched against every input event. Only messages that pass
    /// the condition are forwarded; messages that don’t pass are dropped.
    #[configurable(required, example = ".meta.foo[0] contains bar")]
    condition: String,
}

#[async_trait]
#[typetag::serde(name = "filter")]
impl TransformConfig for Config {
    async fn build(&self, _cx: &TransformContext) -> framework::Result<Transform> {
        let program = vtl::compile(&self.condition)
            .map_err(|err| Diagnostic::new(self.condition.clone()).snippets(err))?;

        if !program.type_def().is_boolean() {
            return Err("vtl filter must return a bool".into());
        }

        Ok(Transform::function(Filter::new(program)))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::logs()]
    }

    fn enable_concurrency(&self) -> bool {
        true
    }
}

#[derive(Clone)]
struct Filter {
    program: Program,

    // metrics
    discarded: Counter,
}

impl Filter {
    fn new(program: Program) -> Self {
        let discarded = metrics::register_counter(
            "events_discarded_total",
            "The total number of events discarded by this component",
        )
        .recorder([]);

        Self { program, discarded }
    }
}

impl FunctionTransform for Filter {
    fn transform(&mut self, output: &mut OutputBuffer, events: Events) {
        if let Events::Logs(logs) = events {
            let mut discarded = 0;

            logs.into_iter().for_each(|log| {
                let mut target = LogTarget { log };

                match self.program.run(&mut target) {
                    Ok(value) => match value {
                        Value::Boolean(b) => {
                            if b {
                                output.push(target.log);
                            } else {
                                discarded += 1;
                            }
                        }

                        value => {
                            warn!(
                                message = "unexpected value type resolved",
                                r#type = value.kind().to_string(),
                                internal_log_rate_limit = true
                            );
                        }
                    },
                    Err(err) => {
                        warn!(
                            message = "filter script run failed",
                            %err,
                            internal_log_rate_limit = true
                        )
                    }
                }
            });

            self.discarded.inc(discarded);
        }
    }
}

#[cfg(test)]
mod tests {
    use event::{Event, LogRecord};
    use value::value;

    use super::*;
    use crate::transforms::transform_one;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }

    #[test]
    fn transform() {
        let log = LogRecord::from(value!({
            message: "info blah blah blah",
            meta: {
                array: [1, 2, 3],
            }
        }));

        let tests = [
            (
                "contains(.message, \"info\")",
                Some(Event::from(log.clone())),
            ),
            ("contains(.message, \"warn\")", None),
            ("contains(.foo, \"info\")", None),
            (".meta.array[0] < 1", None),
            (".meta.array[1] < 3", Some(Event::from(log.clone()))),
        ];

        for (input, want) in tests {
            let program = vtl::compile(input).unwrap();
            let mut transform = Filter::new(program);
            let got = transform_one(&mut transform, log.clone());
            assert_eq!(
                got, want,
                "input: {}\nwant: {:?}\ngot:  {:?}",
                input, want, got
            );
        }
    }
}
