use configurable::configurable_component;
use event::{Event, EventContainer, Events};
use framework::config::{DataType, Output, TransformConfig, TransformContext};
use framework::{FunctionTransform, OutputBuffer, Transform};
use metrics::Counter;

#[configurable_component(transform, name = "sample")]
#[derive(Clone, Debug)]
struct SampleConfig {
    /// The rate at which events will be forwarded, expressed as 1/N. For
    /// example, "10" means 1 out of every 10 events will be forwarded and
    /// rest will be dropped
    #[configurable(required)]
    rate: u64,

    /// The name of the log field whose value will be hased to determine
    /// if the event should be passed.
    ///
    /// Consistently samples the same events. Actual rate of sampling may
    /// differ from the configured one if values in the field are not
    /// uniformly distributed. If left unspecified, or if the event doesn't
    /// have "key_field", events will be count rated.
    key_field: Option<String>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "sample")]
impl TransformConfig for SampleConfig {
    async fn build(&self, _cx: &TransformContext) -> crate::Result<Transform> {
        Ok(Transform::function(Sample::new(
            self.rate,
            self.key_field.clone(),
        )))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }
}

#[derive(Clone, Debug)]
struct Sample {
    rate: u64,
    count: u64,
    key_field: Option<String>,

    // metrics
    discards_events: Counter,
}

impl Sample {
    pub fn new(rate: u64, key_field: Option<String>) -> Self {
        Self {
            rate,
            count: 0,
            key_field,
            discards_events: metrics::register_counter(
                "events_discarded_total",
                "The total number of events discarded by this component.",
            )
            .recorder(&[]),
        }
    }
}

impl FunctionTransform for Sample {
    fn transform(&mut self, output: &mut OutputBuffer, events: Events) {
        let mut logs = vec![];
        for event in events.into_events() {
            if let Event::Log(log) = event {
                let value = self
                    .key_field
                    .as_ref()
                    .and_then(|field| log.fields.get(field.as_str()))
                    .map(|v| v.to_string_lossy());

                let num = if let Some(value) = value {
                    seahash::hash(value.as_bytes())
                } else {
                    self.count
                };

                self.count = (self.count + 1) % self.rate;
                if num % self.rate == 0 {
                    logs.push(log);
                } else {
                    self.discards_events.inc(1);
                }
            }
        }

        output.push(logs);
    }
}

#[cfg(test)]
mod tests {
    use super::SampleConfig;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<SampleConfig>()
    }
}
