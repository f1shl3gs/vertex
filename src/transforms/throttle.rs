use std::num::NonZeroU32;
use std::pin::Pin;
use std::time::Duration;

use crate::common::events::TemplateRenderingFailed;
use async_stream::stream;
use async_trait::async_trait;
use event::Event;
use framework::config::{
    DataType, GenerateConfig, Output, TransformConfig, TransformContext, TransformDescription,
};
use framework::template::Template;
use framework::{TaskTransform, Transform};
use futures::{Stream, StreamExt};
use futures_util::stream;
use governor::{clock, Quota, RateLimiter};
use internal::emit;
use serde::{Deserialize, Serialize};
use snafu::{OptionExt, Snafu};

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ThrottleConfig {
    rate: u32,
    key_field: Option<Template>,
}

impl GenerateConfig for ThrottleConfig {
    fn generate_config() -> String {
        r#"
# Rate of each log stream, n/s
#
rate: 100

# The name of the log field whose value will be hashed to determine if the
# event should be rate limited.
#
# If left unspecified, or if the event doesn't have "key_field", the
# event be will not rate limited separately.
#
key_field: hostname

"#
        .into()
    }
}

inventory::submit! {
    TransformDescription::new::<ThrottleConfig>("throttle")
}

#[async_trait]
#[typetag::serde(name = "throttle")]
impl TransformConfig for ThrottleConfig {
    async fn build(&self, _cx: &TransformContext) -> framework::Result<Transform> {
        let flush_keys_interval = Duration::from_secs(1);
        let threshold = NonZeroU32::new(self.rate).unwrap();

        let quota = Quota::with_period(Duration::from_secs(1)).unwrap();
        quota.allow_burst(threshold);

        let throttle = Throttle {
            quota,
            clock: clock::MonotonicClock,
            flush_keys_interval,
            key_field: self.key_field.clone(),
        };

        Ok(Transform::event_task(throttle))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }

    fn transform_type(&self) -> &'static str {
        "throttle"
    }
}

#[derive(Debug, Snafu)]
enum BuildError {
    #[snafu(display("`rate` must e non-zero"))]
    NonZero,
}

#[derive(Clone)]
struct Throttle<C: clock::Clock<Instant = I>, I: clock::Reference> {
    quota: Quota,
    flush_keys_interval: Duration,
    key_field: Option<Template>,
    clock: C,
}

impl<C, I> Throttle<C, I>
where
    C: clock::Clock<Instant = I>,
    I: clock::Reference,
{
    pub fn new(rate: u32, clock: C, key_field: Option<Template>) -> crate::Result<Self> {
        let flush_keys_interval = Duration::from_secs(1);
        let threshold = NonZeroU32::new(rate).context(NonZero)?;

        let quota = Quota::with_period(Duration::from_secs(1)).unwrap();
        quota.allow_burst(threshold);

        let throttle = Throttle {
            quota,
            clock,
            flush_keys_interval,
            key_field,
        };

        Ok(throttle)
    }
}

impl<C, I> TaskTransform for Throttle<C, I>
where
    C: clock::Clock<Instant = I> + Send + 'static,
    I: clock::Reference + Send + 'static,
{
    fn transform(
        self: Box<Self>,
        mut input_rx: Pin<Box<dyn Stream<Item = Event> + Send>>,
    ) -> Pin<Box<dyn Stream<Item = Event> + Send>> {
        let mut flush_keys = tokio::time::interval(self.flush_keys_interval);
        let mut flush_stream = tokio::time::interval(Duration::from_secs(1));
        let limiter = RateLimiter::dashmap_with_clock(self.quota, &self.clock);

        Box::pin(
            stream! {
                loop {
                    let mut output = Vec::new();
                    let done = tokio::select! {
                        biased;

                        maybe_event = input_rx.next() => {
                            match maybe_event {
                                None => true,
                                Some(event) => {
                                    let key = self.key_field.as_ref()
                                        .and_then(|tmpl| {
                                            tmpl.render_string(&event)
                                                .map_err(|err| {
                                                    emit!(&TemplateRenderingFailed {
                                                        err,
                                                        field: Some("key_field"),
                                                        drop_event: false,
                                                    });
                                                }).ok()
                                        });

                                    match limiter.check_key(&key) {
                                        Ok(()) => {

                                        },
                                        _ => {}
                                    }

                                    false
                                }
                            }
                        }

                        _ = flush_keys.tick() => {
                            limiter.retain_recent();
                            false
                        }

                        _ = flush_stream.tick() => {
                            false
                        }
                    };

                    yield stream::iter(output.into_iter());

                    if done {
                        break
                    }
                }
            }
            .flatten(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use std::task::Poll;

    #[test]
    fn generate_config() {
        crate::testing::test_generate_config::<ThrottleConfig>();
    }

    #[tokio::test]
    async fn throttle_events() {
        let clock = clock::FakeRelativeClock::default();
        let config = serde_yaml::from_str::<ThrottleConfig>(
            r#"
rate: 2
"#,
        )
        .unwrap();

        let transform =
            Transform::event_task(Throttle::new(config.rate, clock.clone(), config.key_field).unwrap());
        let throttle = transform.into_task();

        let (mut tx, rx) = futures::channel::mpsc::channel(10);
        let mut out_stream = throttle.transform(Box::pin(rx));

        // tokio interval is always immediately ready, so we poll once to make sure
        // we trip it/set the interval in the future.
        assert_eq!(Poll::Pending, futures::poll!(out_stream.next()));

        tx.send(Event::from("")).await.unwrap();
        tx.send(Event::from("")).await.unwrap();

        let mut count = 0u8;
        while count < 2 {
            if let Some(_event) = out_stream.next().await {
                count += 1;
            } else {
                panic!("Unexpectedly received None in output stream");
            }
        }

        assert_eq!(2, count);
        clock.advance(Duration::from_secs(2));

        tx.send(Event::from("")).await.unwrap();

        // We should be back to pending, having the second event dropped.
        assert_eq!(Poll::Pending, futures::poll!(out_stream.next()));

        clock.advance(Duration::from_secs(3));

        tx.send(Event::from("")).await.unwrap();

        // The rate limiter should now be refreshed and allow
        // an additional event through.
        out_stream
            .next()
            .await
            .expect("Unexpectedly received None in output stream");

        // We should be back to pending, having nothing waiting for us
        assert_eq!(Poll::Pending, futures::poll!(out_stream.next()));

        tx.disconnect();

        // And still nothing there
        assert_eq!(Poll::Ready(None), futures::poll!(out_stream.next()));
    }
}
