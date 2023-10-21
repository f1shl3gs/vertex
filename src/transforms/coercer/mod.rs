mod conversion;

use std::collections::HashMap;

use async_trait::async_trait;
use configurable::configurable_component;
use conversion::{parse_conversion_map, Conversion};
use event::log::{OwnedTargetPath, Value};
use event::Events;
use framework::config::{DataType, Output, TransformConfig, TransformContext};
use framework::timezone::TimeZone;
use framework::{FunctionTransform, OutputBuffer, Transform};

#[configurable_component(transform, name = "coercer")]
#[serde(deny_unknown_fields)]
struct Config {
    /// Coerce log filed to another type.
    ///
    /// NB: nonconvertible filed will be dropped.
    #[configurable(required)]
    types: HashMap<OwnedTargetPath, String>,

    timezone: Option<TimeZone>,
}

#[async_trait]
#[typetag::serde(name = "coercer")]
impl TransformConfig for Config {
    async fn build(&self, cx: &TransformContext) -> framework::Result<Transform> {
        let timezone = self.timezone.unwrap_or(cx.globals.timezone);
        let types = parse_conversion_map(&self.types, timezone)?;

        Ok(Transform::function(Coercer::new(types)))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }
}

#[derive(Clone, Debug)]
struct Coercer {
    types: HashMap<OwnedTargetPath, Conversion>,
}

impl Coercer {
    pub const fn new(types: HashMap<OwnedTargetPath, Conversion>) -> Self {
        Self { types }
    }
}

impl FunctionTransform for Coercer {
    fn transform(&mut self, output: &mut OutputBuffer, mut events: Events) {
        let mut errors = 0;

        events.for_each_log(|log| {
            for (field, conv) in &self.types {
                if let Some(value) = log.remove(field) {
                    match conv.convert::<Value>(value.coerce_to_bytes()) {
                        Ok(converted) => {
                            log.insert(field, converted);
                        }
                        Err(err) => {
                            error!(
                                message = "Could not convert types",
                                ?field,
                                ?err,
                                internal_log_rate_limit = true
                            );

                            errors += 1;
                        }
                    }
                }
            }
        });

        output.push(events);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event::{fields, LogRecord};

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>();
    }

    async fn run() -> LogRecord {
        let log = LogRecord::from(fields!(
            "number" => "1234",
            "bool" => "yes",
            "other" => "no",
            "float" => "broken"
        ));
        let metadata = log.metadata().clone();

        let mut coercer = serde_yaml::from_str::<Config>(
            r##"
types:
  number: int
  float: float
  bool: bool
"##,
        )
        .unwrap()
        .build(&TransformContext::default())
        .await
        .unwrap();

        let coercer = coercer.as_function();
        let mut buf = OutputBuffer::with_capacity(1);
        coercer.transform(&mut buf, log.into());
        let result = buf.first().unwrap().into_log();

        assert_eq!(&metadata, result.metadata());
        result
    }

    #[tokio::test]
    async fn converts() {
        let log = run().await;

        // valid fields
        assert_eq!(log.get("number").unwrap().clone(), 1234.into());
        assert_eq!(log.get("bool").unwrap().clone(), true.into());

        // drops non convertible fields
        assert!(log.get("float").is_none());

        // leaves unnamed fields
        assert_eq!(log.get("other").unwrap().clone(), "no".into());
    }
}
