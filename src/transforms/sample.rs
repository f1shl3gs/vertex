use event::{Event, EventContainer, Events};
use framework::config::{
    DataType, GenerateConfig, Output, TransformConfig, TransformContext, TransformDescription,
};
use framework::{FunctionTransform, OutputBuffer, Transform};
use internal::InternalEvent;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SampleConfig {
    rate: u64,
    key_field: Option<String>,
}

impl GenerateConfig for SampleConfig {
    fn generate_config() -> String {
        r#"
# The rate at which events will be forwarded, expressed as 1/N. For
# example, "10" means 1 out of every 10 events will be forwarded and
# rest will be dropped
#
rate: 10

# The name of the log field whose value will be hased to determine
# if the event should be passed.
#
# Consistently samples the same events. Actual rate of sampling may
# differ from the configured one if values in the field are not
# uniformly distributed. If left unspecified, or if the event doesn't
# have "key_field", events will be count rated.
#
# key_field: foo.bar
        "#
        .into()
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "sample")]
impl TransformConfig for SampleConfig {
    async fn build(&self, _ctx: &TransformContext) -> crate::Result<Transform> {
        todo!()
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }

    fn transform_type(&self) -> &'static str {
        "sample"
    }
}

inventory::submit! {
    TransformDescription::new::<SampleConfig>("sample")
}

#[derive(Clone, Debug)]
struct Sample {
    rate: u64,
    count: u64,
    key_field: Option<String>,
}

impl Sample {
    pub const fn new(rate: u64) -> Self {
        Self {
            rate,
            count: 0,
            key_field: None,
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
                    .and_then(|field| log.fields.get(field))
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
                    emit!(&SampleEventDiscarded);
                }
            }
        }

        output.push(logs.into());
    }
}

#[derive(Debug)]
struct SampleEventDiscarded;

impl InternalEvent for SampleEventDiscarded {
    fn emit_metrics(&self) {
        counter!("events_discarded_total", 1);
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
