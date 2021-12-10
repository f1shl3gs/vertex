use event::Event;
use internal::InternalEvent;
use serde::{Deserialize, Serialize};

use crate::config::{
    DataType, GenerateConfig, GlobalOptions, TransformConfig, TransformDescription,
};
use crate::transforms::{FunctionTransform, Transform};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SampleConfig {
    rate: u64,
    key_field: Option<String>,
}

impl GenerateConfig for SampleConfig {
    fn generate_config() -> serde_yaml::Value {
        serde_yaml::to_value(Self {
            rate: 0,
            key_field: None,
        })
        .unwrap()
    }
}

#[async_trait::async_trait]
#[typetag::serde(name = "sample")]
impl TransformConfig for SampleConfig {
    async fn build(&self, _global: &GlobalOptions) -> crate::Result<Transform> {
        todo!()
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn output_type(&self) -> DataType {
        DataType::Log
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
    pub fn new(rate: u64) -> Self {
        Self {
            rate,
            count: 0,
            key_field: None,
        }
    }
}

impl FunctionTransform for Sample {
    fn transform(&mut self, output: &mut Vec<Event>, event: Event) {
        let value = self
            .key_field
            .as_ref()
            .and_then(|field| {
                let log = event.as_log();
                log.fields.get(field)
            })
            .map(|v| v.to_string_lossy());

        let num = if let Some(value) = value {
            seahash::hash(value.as_bytes())
        } else {
            self.count
        };

        self.count = (self.count + 1) % self.rate;
        if num % self.rate == 0 {
            output.push(event);
        } else {
            emit!(&SampleEventDiscarded);
        }
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
    use crate::config::test_generate_config;

    #[test]
    fn generate_config() {
        test_generate_config::<SampleConfig>()
    }
}
