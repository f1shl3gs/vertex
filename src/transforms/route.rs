use async_trait::async_trait;
use condition::Expression;
use configurable::configurable_component;
use event::Events;
use framework::config::{DataType, Output, TransformConfig, TransformContext};
use framework::{SyncTransform, Transform, TransformOutputsBuf};
use indexmap::IndexMap;

const UNMATCHED_ROUTE: &str = "_unmatched";

#[configurable_component(transform, name = "route")]
#[serde(deny_unknown_fields)]
struct Config {
    /// A table of route identifiers to logical conditions representing the filter of the route.
    /// Each route can then be referenced as an input by other components with the name
    /// <transform_name>.<route_id>. If an event doesn’t match any route, it will be sent to
    /// the <transform_name>._unmatched output. Note, _unmatched is a reserved output name and
    /// cannot be used as a route name. _default is also reserved for future use.
    #[configurable(required)]
    route: IndexMap<String, String>,
}

#[async_trait]
#[typetag::serde(name = "route")]
impl TransformConfig for Config {
    async fn build(&self, _cx: &TransformContext) -> framework::Result<Transform> {
        let route = Route::new(&self.route)?;
        Ok(Transform::synchronous(route))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn outputs(&self) -> Vec<Output> {
        let mut result: Vec<Output> = self
            .route
            .keys()
            .map(|name| Output::default(DataType::Log).with_port(name))
            .collect();
        result.push(Output::default(DataType::Log).with_port(UNMATCHED_ROUTE));

        result
    }

    fn enable_concurrency(&self) -> bool {
        true
    }
}

#[derive(Clone)]
struct Route {
    routes: IndexMap<String, Expression>,
}

impl Route {
    fn new(routes: &IndexMap<String, String>) -> Result<Self, condition::Error> {
        let routes = routes
            .iter()
            .map(|(name, expr)| {
                let expr = condition::parse(expr)?;
                Ok((name.to_string(), expr))
            })
            .collect::<Result<IndexMap<String, Expression>, condition::Error>>()?;

        Ok(Self { routes })
    }
}

impl SyncTransform for Route {
    fn transform(&mut self, events: Events, output: &mut TransformOutputsBuf) {
        if let Events::Logs(logs) = events {
            logs.into_iter().for_each(|log| {
                let mut failed = 0;

                for (name, expr) in &self.routes {
                    match expr.eval(&log) {
                        Ok(b) if b => output.push_named(name, log.clone().into()),
                        _ => failed += 1,
                    }
                }

                if failed == self.routes.len() {
                    output.push_named(UNMATCHED_ROUTE, log.into())
                }
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event::{fields, Event, EventContainer, LogRecord};

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>();
    }

    #[test]
    fn route() {
        let outputs = ["first", "second", "third", UNMATCHED_ROUTE];
        let event: Event = LogRecord::from(fields!(
            "message" => "hello world",
            "second" => "second",
            "third" => "third",
        ))
        .into();

        let tests = [
            (
                "match all",
                r##"
route:
    first: .message contains world
    second: .second contains sec
    third: .third contains rd
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
    first: .message contains world
    second: .second contains foo
    third: .third contains bar
"##,
                [Some(event.clone()), None, None, None],
            ),
            (
                "no match",
                r##"
route:
    first: .message contains foo
    second: .second contains foo
    third: .third contains bar
"##,
                [None, None, None, Some(event.clone())],
            ),
        ];

        for (_test, config, wants) in tests {
            let config = serde_yaml::from_str::<Config>(config).unwrap();
            let mut transform = Route::new(&config.route).unwrap();
            let mut buf = TransformOutputsBuf::new_with_capacity(
                outputs
                    .iter()
                    .map(|name| Output::default(DataType::Log).with_port(name.to_string()))
                    .collect(),
                1,
            );

            transform.transform(event.clone().into(), &mut buf);

            for (output, want) in outputs.iter().zip(wants) {
                let mut events: Vec<_> = buf.drain_named(output).collect();
                match want {
                    None => assert!(events.is_empty()),
                    Some(want) => {
                        assert_eq!(events.len(), 1);
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
