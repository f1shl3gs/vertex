use async_trait::async_trait;
use configurable::configurable_component;
use event::Events;
use framework::config::{DataType, Output, TransformConfig, TransformContext};
use framework::{SyncTransform, Transform, TransformOutputsBuf};
use indexmap::IndexMap;
use value::Value;
use vtl::{Diagnostic, Program};

use crate::common::vtl::LogTarget;

const UNMATCHED_ROUTE: &str = "_unmatched";

#[configurable_component(transform, name = "route")]
#[serde(deny_unknown_fields)]
struct Config {
    /// A table of route identifiers to logical conditions representing the filter of the route.
    /// Each route can then be referenced as an input by other components with the name
    /// <transform_name>.<route_id>. If an event does not match any route, it will be sent to
    /// the <transform_name>._unmatched output. Note, _unmatched is a reserved output name and
    /// cannot be used as a route name. _default is also reserved for future use.
    #[configurable(required)]
    route: IndexMap<String, String>,
}

#[async_trait]
#[typetag::serde(name = "route")]
impl TransformConfig for Config {
    async fn build(&self, _cx: &TransformContext) -> framework::Result<Transform> {
        let mut routes = Vec::with_capacity(self.route.len());

        for (route, script) in &self.route {
            match vtl::compile(script) {
                Ok(program) => {
                    if !program.type_def().is_boolean() {
                        return Err(format!("vtl condition for {route} must return a bool").into());
                    }

                    routes.push((route.to_string(), program));
                }
                Err(err) => {
                    let diagnostic = Diagnostic::new(script.to_string());
                    return Err(diagnostic.snippets(err).into());
                }
            }
        }

        Ok(Transform::synchronous(Route { routes }))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn outputs(&self) -> Vec<Output> {
        let mut result: Vec<Output> = self
            .route
            .keys()
            .map(|name| Output::logs().with_port(name))
            .collect();
        result.push(Output::logs().with_port(UNMATCHED_ROUTE));

        result
    }

    fn enable_concurrency(&self) -> bool {
        true
    }
}

#[derive(Clone)]
struct Route {
    routes: Vec<(String, Program)>,
}

impl SyncTransform for Route {
    fn transform(&mut self, events: Events, output: &mut TransformOutputsBuf) {
        let Events::Logs(logs) = events else {
            return;
        };

        for log in logs {
            let mut failed = 0;
            let mut target = LogTarget { log };

            for (route, program) in self.routes.iter_mut() {
                match program.run(&mut target) {
                    Ok(value) => match value {
                        Value::Boolean(b) => {
                            if b {
                                output.push_named(route, target.log.clone().into())
                            } else {
                                // it's not match, but fine
                            }
                        }
                        value => {
                            failed += 1;
                            let typ = match value {
                                Value::Bytes(_) => "bytes",
                                Value::Float(_) => "float",
                                Value::Integer(_) => "integer",
                                Value::Boolean(_) => unreachable!(),
                                Value::Timestamp(_) => "timestamp",
                                Value::Object(_) => "object",
                                Value::Array(_) => "array",
                                Value::Null => "null",
                            };

                            warn!(
                                message = "unexpected value type resolved",
                                r#type = typ,
                                internal_log_rate_limit = true
                            )
                        }
                    },
                    Err(err) => {
                        warn!(
                            message = "run vtl script failed",
                            ?err,
                            route,
                            internal_log_rate_limit = true
                        );

                        failed += 1;
                    }
                }
            }

            if failed == self.routes.len() {
                output.push_named(UNMATCHED_ROUTE, target.log.into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use event::{Event, LogRecord};
    use value::value;

    use super::*;

    #[test]
    fn generate_config() {
        crate::testing::generate_config::<Config>();
    }

    #[test]
    fn route() {
        let outputs = ["first", "second", "third", UNMATCHED_ROUTE];
        let event: Event = LogRecord::from(value!({
            "message": "hello world",
            "second": "second",
            "third": "third",
        }))
        .into();

        let tests = [
            (
                "match all",
                r##"
route:
    first: contains(.message, "world")
    second: contains(.second, "sec")
    third: contains(.third, "rd")
"##,
                [
                    Some(event.clone()),
                    Some(event.clone()),
                    Some(event.clone()),
                    None,
                ],
            ),
            (
                "pass one",
                r##"
route:
    first: contains(.message, "world")
    second: contains(.second, "foo")
    third: contains(.third, "bar")
"##,
                [Some(event.clone()), None, None, None],
            ),
            (
                "no match",
                r##"
route:
    first: contains(.message, "foo")
    second: contains(.second, "foo")
    third: contains(.third, "bar")
"##,
                [None, None, None, None],
            ),
        ];

        for (test, config, wants) in tests {
            let config = serde_yaml::from_str::<Config>(config).unwrap();
            let mut routes = vec![];
            for (name, script) in config.route {
                let program = vtl::compile(&script).unwrap();
                routes.push((name, program));
            }
            let mut transform = Route { routes };
            let mut buf = TransformOutputsBuf::new_with_capacity(
                outputs
                    .iter()
                    .map(|name| Output::logs().with_port(*name))
                    .collect(),
                1,
            );

            transform.transform(event.clone().into(), &mut buf);

            for (output, want) in outputs.iter().zip(wants) {
                let mut events: Vec<_> = buf.drain_named(output).collect();
                match want {
                    None => assert!(events.is_empty()),
                    Some(want) => {
                        assert_eq!(events.len(), 1, "{test}");
                        let events = events.pop().unwrap();
                        assert_eq!(events.len(), 1);
                        if let Events::Logs(mut logs) = events {
                            assert_eq!(logs.len(), 1);
                            let got = logs.pop().unwrap();
                            assert_eq!(want, got.into());
                        } else {
                            unreachable!();
                        }
                    }
                }
            }
        }
    }
}
