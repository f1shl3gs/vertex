use std::num::NonZeroU32;
use std::pin::Pin;
use std::time::Duration;

use async_stream::stream;
use async_trait::async_trait;
use configurable::configurable_component;
use event::{EventContainer, Events};
use framework::config::{DataType, Output, TransformConfig, TransformContext};
use framework::template::Template;
use framework::{OutputBuffer, TaskTransform, Transform};
use futures::{Stream, StreamExt};
use governor::{clock, Quota, RateLimiter};
use thiserror::Error;

const fn default_window() -> Duration {
    Duration::from_secs(1)
}

#[configurable_component(transform, name = "throttle")]
#[derive(Debug)]
struct ThrottleConfig {
    /// The name of the log field whose value will be hashed to determine if the
    /// event should be rate limited.
    ///
    /// If left unspecified, or if the event doesn't have "key_field", the
    /// event be will not rate limited separately.
    key_field: Option<Template>,

    /// The number of events allowed for a given bucket per configured window.
    /// Each unique key will have its own threshold.
    threshold: u32,

    /// The time frame in which the configured "threshold" is applied.
    #[serde(default = "default_window", with = "humanize::duration::serde")]
    window: Duration,
}

#[async_trait]
#[typetag::serde(name = "throttle")]
impl TransformConfig for ThrottleConfig {
    async fn build(&self, _cx: &TransformContext) -> framework::Result<Transform> {
        let throttle = Throttle::new(
            self.threshold,
            self.window,
            clock::MonotonicClock,
            self.key_field.clone(),
        )?;

        Ok(Transform::event_task(throttle))
    }

    fn input_type(&self) -> DataType {
        DataType::Log
    }

    fn outputs(&self) -> Vec<Output> {
        vec![Output::default(DataType::Log)]
    }
}

#[derive(Debug, Error)]
enum BuildError {
    #[error("`rate` must e non-zero")]
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
    pub fn new(
        threshold: u32,
        window: Duration,
        clock: C,
        key_field: Option<Template>,
    ) -> crate::Result<Self> {
        let flush_keys_interval = window;
        let threshold = NonZeroU32::new(threshold).ok_or(BuildError::NonZero)?;

        let quota = Quota::with_period(Duration::from_secs_f64(
            window.as_secs_f64() / threshold.get() as f64,
        ))
        .ok_or(BuildError::NonZero)?
        .allow_burst(threshold);

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
        mut input_rx: Pin<Box<dyn Stream<Item = Events> + Send>>,
    ) -> Pin<Box<dyn Stream<Item = Events> + Send>> {
        let mut flush_keys = tokio::time::interval(self.flush_keys_interval);
        let mut flush_stream = tokio::time::interval(Duration::from_secs(1));
        let limiter = RateLimiter::dashmap_with_clock(self.quota, &self.clock);

        Box::pin(stream! {
            loop {
                let mut output = OutputBuffer::with_capacity(128);
                let done = tokio::select! {
                    biased;

                    maybe_events = input_rx.next() => {
                        match maybe_events {
                            None => true,
                            Some(events) => {
                                for event in events.into_events() {
                                    let key = self.key_field.as_ref()
                                    .and_then(|tmpl| {
                                        tmpl.render_string(&event)
                                            .map_err(|err| {
                                                error!(
                                                    message = "Failed to render template",
                                                    ?err,
                                                    field = "key_field",
                                                    drop_event = false
                                                );
                                                // TODO: metrics
                                                // emit!(&TemplateRenderingFailed {
                                                //     err,
                                                //     field: Some("key_field"),
                                                //     drop_event: false,
                                                // });
                                            }).ok()
                                    });

                                    match limiter.check_key(&key) {
                                        Ok(()) => output.push_one(event),
                                        _ => {
                                            debug!(message = "Rate limit exceeded", ?key);

                                            // TODO: metrics
                                            //
                                            // counter!("events_discarded_total", 1, "key" => self.key.to_owned())
                                            //
                                            // if let Some(key) = key {
                                            //     emit!(&ThrottleEventDiscarded{ key });
                                            // } else {
                                            //     emit!(&ThrottleEventDiscarded { key: "None".to_string() })
                                            // }
                                        }
                                    }
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

                for events in output.drain() {
                    yield events
                }

                if done {
                    break
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event::{fields, Event};
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
threshold: 2
window: 5s
"#,
        )
        .unwrap();

        let transform = Transform::event_task(
            Throttle::new(
                config.threshold,
                config.window,
                clock.clone(),
                config.key_field,
            )
            .unwrap(),
        );
        let throttle = transform.into_task();

        let (mut tx, rx) = futures::channel::mpsc::channel(10);
        let mut out_stream = throttle.transform_events(Box::pin(rx));

        // tokio interval is always immediately ready, so we poll once to make sure
        // we trip it/set the interval in the future.
        assert_eq!(Poll::Pending, futures::poll!(out_stream.next()));

        tx.send(Event::from("")).await.unwrap();
        tx.send(Event::from("")).await.unwrap();

        let mut count = 0_u8;
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

    #[tokio::test]
    async fn throttle_buckets() {
        let clock = clock::FakeRelativeClock::default();
        let config = serde_yaml::from_str::<ThrottleConfig>(
            r#"
threshold: 1
window: 5s
key_field: "{{ bucket }}"
"#,
        )
        .unwrap();

        assert!(config.key_field.is_some());
        let throttle = Throttle::new(config.threshold, config.window, clock, config.key_field)
            .map(Transform::event_task)
            .unwrap();

        let throttle = throttle.into_task();

        let (mut tx, rx) = futures::channel::mpsc::channel(10);
        let mut out_stream = throttle.transform_events(Box::pin(rx));

        // tokio interval is always immediately ready, so we poll once to
        // make sure we trip it/set the interval in the furture
        assert_eq!(Poll::Pending, futures::poll!(out_stream.next()));

        let log_a = Event::from(fields!("bucket" => "a"));
        let log_b = Event::from(fields!("bucket" => "b"));
        tx.send(log_a).await.unwrap();
        tx.send(log_b).await.unwrap();

        let mut count = 0u8;
        while count < 2 {
            if let Some(_event) = out_stream.next().await {
                count += 1;
            } else {
                panic!("Unexpectedly received None in output stream");
            }
        }

        assert_eq!(2, count);

        // We should be back to pending, having nothing waiting for us
        assert_eq!(Poll::Pending, futures::poll!(out_stream.next()));

        tx.disconnect();

        // And still nothing there
        assert_eq!(Poll::Ready(None), futures::poll!(out_stream.next()));
    }
}
