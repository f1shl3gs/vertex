use event::Event;
use crate::config::{DataType, GlobalOptions, TransformConfig};
use crate::transforms::{FunctionTransform, Transform};

#[derive(Debug, Clone)]
struct SampleConfig {
    rate: u64,
    key_field: Option<String>,
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
        let value = self.key_field
            .as_ref()
            .and_then(|fiedld| {
                let log = event.as_log();
                log.fields.get(fiedld)
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
            emit!(SampleEventDiscard);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_config() {}
}