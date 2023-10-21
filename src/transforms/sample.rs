use ahash::RandomState;
use configurable::configurable_component;
use event::log::path::TargetPath;
use event::log::OwnedTargetPath;
use event::{Event, EventContainer, Events};
use framework::config::{DataType, Output, TransformConfig, TransformContext};
use framework::{FunctionTransform, OutputBuffer, Transform};
use metrics::Counter;

#[configurable_component(transform, name = "sample")]
struct Config {
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
    key_field: Option<OwnedTargetPath>,
}

#[async_trait::async_trait]
#[typetag::serde(name = "sample")]
impl TransformConfig for Config {
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
    key_field: Option<OwnedTargetPath>,
    state: RandomState,

    // metrics
    discards_events: Counter,
}

impl Sample {
    pub fn new(rate: u64, key_field: Option<OwnedTargetPath>) -> Self {
        let state = RandomState::with_seeds(
            0x16f11fe89b0d677c,
            0xb480a793d8e6c86c,
            0x6fe2e5aaf078ebc9,
            0x14f994a4c5259381,
        );

        Self {
            rate,
            count: 0,
            key_field,
            state,
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
                    .and_then(|field| log.value().get(field.value_path()))
                    .map(|v| v.to_string_lossy());

                let num = if let Some(value) = value {
                    self.state.hash_one(value.as_bytes())
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
    use super::{Config, Sample};
    use event::{Event, LogRecord};
    use framework::{FunctionTransform, OutputBuffer};
    use log_schema::log_schema;
    use testify::random::random_lines;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<Config>()
    }

    fn assert_approx_eq(a: f64, b: f64, delta: f64) {
        if !(a - b < delta || b - a < delta) {
            panic!("{} not approx equal to {}", a, b);
        }
    }

    fn random_events(n: usize) -> Vec<Event> {
        random_lines(10)
            .take(n)
            .map(|l| Event::Log(LogRecord::from(l)))
            .collect()
    }

    #[test]
    fn hash_samples_at_roughly_the_configured_rate() {
        let num = 10000;

        for rate in [2, 5, 10, 20, 50, 100] {
            let events = random_events(num);
            let mut sampler = Sample::new(rate, Some(log_schema().message_key().clone()));
            let passed = events
                .into_iter()
                .filter_map(|event| {
                    let mut buf = OutputBuffer::with_capacity(1);
                    sampler.transform(&mut buf, event.into());
                    buf.into_events().next()
                })
                .count();
            let ideal = 1.0 / rate as f64;
            let actual = passed as f64 / num as f64;
            assert_approx_eq(ideal, actual, ideal * 0.5);
        }
    }

    #[test]
    fn hash_consistently_samples_the_same_events() {
        let events = random_events(1000);
        let mut sampler = Sample::new(2, Some(log_schema().message_key().clone()));

        let first_run = events
            .clone()
            .into_iter()
            .filter_map(|event| {
                let mut buf = OutputBuffer::with_capacity(1);
                sampler.transform(&mut buf, event.into());
                buf.into_events().next()
            })
            .collect::<Vec<_>>();

        let second_run = events
            .clone()
            .into_iter()
            .filter_map(|event| {
                let mut buf = OutputBuffer::with_capacity(1);
                sampler.transform(&mut buf, event.into());
                buf.into_events().next()
            })
            .collect::<Vec<_>>();

        assert_eq!(first_run, second_run);
    }
}
