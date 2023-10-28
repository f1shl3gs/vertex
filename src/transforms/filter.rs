use async_trait::async_trait;
use condition::Expression;
use configurable::configurable_component;
use event::Events;
use framework::config::{DataType, Output, TransformConfig, TransformContext};
use framework::{FunctionTransform, OutputBuffer, Transform};
use metrics::Counter;

/// Filters events based on a set of conditions.
#[configurable_component(transform, name = "filter")]
#[serde(deny_unknown_fields)]
struct Config {
    /// The condition to be matched against every input event. Only messages that pass
    /// the condition are forwarded; messages that don’t pass are dropped.
    /// The LHS is always start with '.' and it's a path,
    /// e.g.
    ///   .meta.foo[0] contains bar
    ///   .message contains bar && (.upper > 10 or .lower lt 5.001)
    #[configurable(required, example = ".meta.foo[0] contains bar")]
    condition: Expression,
}

#[async_trait]
#[typetag::serde(name = "filter")]
impl TransformConfig for Config {
    async fn build(&self, _cx: &TransformContext) -> framework::Result<Transform> {
        let filter = Filter::new(self.condition.clone())?;
        Ok(Transform::function(filter))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }
}

#[derive(Clone)]
struct Filter {
    expr: Expression,

    // metrics
    discarded: Counter,
}

impl Filter {
    fn new(expr: Expression) -> Result<Self, crate::Error> {
        let discarded = metrics::register_counter(
            "events_discarded_total",
            "The total number of events discarded by this component",
        )
        .recorder([]);

        Ok(Self { expr, discarded })
    }
}

impl FunctionTransform for Filter {
    fn transform(&mut self, output: &mut OutputBuffer, events: Events) {
        if let Events::Logs(logs) = events {
            let mut discarded = 0;

            logs.into_iter().for_each(|log| match self.expr.eval(&log) {
                Ok(b) => {
                    if b {
                        output.push(log)
                    } else {
                        discarded += 1;
                    }
                }
                Err(err) => {
                    error!(
                        message = "eval condition failed",
                        ?err,
                        internal_log_rate_limit = true
                    );

                    discarded += 1;
                }
            });

            self.discarded.inc(discarded);
        }
    }
}

#[cfg(test)]
mod tests {
    use event::{fields, Event, LogRecord};

    use super::*;
    use crate::transforms::transform_one;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>();
    }

    #[test]
    fn transform() {
        let log = LogRecord::from(fields!(
            "message" => "info blah blah blah",
            "meta" => fields!(
                "array" => vec![1, 2, 3],
            )
        ));

        let tests = [
            (".message contains info", Some(Event::from(log.clone()))),
            (".message contains warn", None),
            (".foo contains info", None),
            (".meta.array[0] < 1", None),
            (".meta.array[1] < 3", Some(Event::from(log.clone()))),
        ];

        for (input, want) in tests {
            let expr = Expression::parse(input).unwrap();
            let mut transform = Filter::new(expr).unwrap();
            let got = transform_one(&mut transform, log.clone());
            assert_eq!(
                got, want,
                "input: {}\nwant: {:?}\ngot:  {:?}",
                input, want, got
            );
        }
    }
}
