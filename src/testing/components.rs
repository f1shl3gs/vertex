#![allow(clippy::redundant_pattern_matching)]

use std::env;
use std::future::Future;
use std::sync::LazyLock;
use std::time::Duration;

use event::{Events, Metric, MetricValue};
use framework::config::{SourceConfig, SourceContext};
use framework::{Pipeline, Sink};
use futures::Stream;
use futures::StreamExt;
use tokio::time::sleep;
use tokio::{pin, select};

use super::metrics::capture_metrics;

/// The standard set of tags for all `HttpSink`-based sinks.
pub const HTTP_SINK_TAGS: [&str; 2] = ["endpoint", "protocol"];

/// The most basic set of tags for sources, regardless of whether or not they pull data or have it pushed in.
pub const SOURCE_TAGS: [&str; 1] = ["output"];

/// The component test specification for all sources.
pub static SOURCE_TESTS: LazyLock<ComponentTests> = LazyLock::new(|| ComponentTests {
    tagged_counters: &[
        "component_sent_events_total",
        "component_sent_event_bytes_total",
    ],
    untagged_counters: &[],
});

/// The component test specification for sinks that are push-based.
pub static SINK_TESTS: LazyLock<ComponentTests> = LazyLock::new(|| ComponentTests {
    tagged_counters: &["component_sent_bytes_total"],
    untagged_counters: &[
        "component_sent_events_total",
        "component_sent_event_bytes_total",
    ],
});

/// This struct is used to describe a set of component tests.
pub struct ComponentTests {
    /// The list of counter metrics (with given tags) that must be incremented
    tagged_counters: &'static [&'static str],
    /// The list of counter metrics (with no particular tags) that must be incremented
    untagged_counters: &'static [&'static str],
}

impl ComponentTests {
    /// Run the test specification, and assert that all tests passed.
    #[track_caller]
    pub fn assert(&self, tags: &[&str]) {
        let mut test = ComponentTester::new();
        test.emitted_all_counters(self.tagged_counters, tags);
        test.emitted_all_counters(self.untagged_counters, &[]);

        if !test.errors.is_empty() {
            panic!(
                "Failed to assert compliance, errors:\n{}\n",
                test.errors.join("\n")
            );
        }
    }
}

/// Standard metrics test environment data
struct ComponentTester {
    metrics: Vec<Metric>,
    errors: Vec<String>,
}

impl ComponentTester {
    #[allow(clippy::print_stdout)]
    fn new() -> Self {
        let metrics = capture_metrics();
        let errors = Vec::new();

        if env::var("DEBUG_COMPONENT_COMPLIANCE").is_ok() {
            for metric in &metrics {
                println!("capture metric: {}", metric);
            }
        }

        Self { metrics, errors }
    }

    fn emitted_all_counters(&mut self, names: &[&str], tags: &[&str]) {
        let tag_suffix = (!tags.is_empty())
            .then(|| format!("{{{}}}", tags.join(",")))
            .unwrap_or_default();

        for name in names {
            if !self.metrics.iter().any(|m| {
                matches!(m.value(), MetricValue::Sum { .. })
                    && m.name() == *name
                    && has_tags(m, tags)
            }) {
                // If we didn't find a direct match, see if any other metrics exist which are counters of the same name,
                // which could represent metrics being emitted but without the correct tag(s).
                let partial_matches = self
                    .metrics
                    .iter()
                    .filter(|m| {
                        matches!(m.value(), MetricValue::Sum { .. })
                            && m.name() == *name
                            && !has_tags(m, tags)
                    })
                    .map(|m| {
                        let tags = m
                            .tags()
                            .iter()
                            .map(|(key, _value)| key.as_str())
                            .collect::<Vec<_>>()
                            .join(",");
                        format!("\n    -> Found similar metric `{}{}`", m.name(), tags)
                    })
                    .collect::<Vec<_>>();
                let partial = partial_matches.join("");

                self.errors.push(format!(
                    "  - Missing metric `{}{}`{}",
                    name, tag_suffix, partial
                ));
            }
        }
    }
}

/// Tests if the given metric contains all the given tag names
fn has_tags(metric: &Metric, names: &[&str]) -> bool {
    for name in names {
        // Avoid `Option::is_some` because it bloats LLVM IR.
        if let Some(_) = metric.tags().get(name) {
            return true;
        }
    }

    false
}

/// Initialize the necessary bits needed to run a component test specification.
#[cfg(test)]
pub fn init_test() {
    framework::trace::init(false, false, "error", 10);
    testify::event::clear_recorded_events();
}

/// Convenience wrapper for running sink tests
pub async fn assert_sink_compliance<T>(tags: &[&str], f: impl Future<Output = T>) -> T {
    init_test();

    let result = f.await;

    SINK_TESTS.assert(tags);

    result
}

pub async fn run_and_assert_sink_compliance<S, I>(sink: Sink, events: S, tags: &[&str])
where
    S: Stream<Item = I> + Send,
    I: Into<Events>,
{
    assert_sink_compliance(tags, async move {
        let events = events.map(Into::into);
        sink.run(events).await.expect("Running sink failed")
    })
    .await;
}

/// Runs source tests with timeout and asserts happy path compliance.
pub async fn run_and_assert_source_compliance<SC>(
    source: SC,
    timeout: Duration,
    tags: &[&str],
) -> Vec<Events>
where
    SC: SourceConfig,
{
    run_and_assert_source_advanced(source, |_| {}, Some(timeout), None, &SOURCE_TESTS, tags).await
}

/// Runs and asserts source test specifications with configurations.
pub async fn run_and_assert_source_advanced<SC>(
    source: SC,
    setup: impl FnOnce(&mut SourceContext),
    timeout: Option<Duration>,
    event_count: Option<usize>,
    tests: &LazyLock<ComponentTests>,
    tags: &[&str],
) -> Vec<Events>
where
    SC: SourceConfig,
{
    assert_source(tests, tags, async move {
        // Build the source and set ourselves up to both drive it to completion
        // as well as collect all the events it sends out.
        let (tx, mut rx) = Pipeline::new_test();
        let mut context = SourceContext::new_test(tx);

        setup(&mut context);

        let mut source = source
            .build(context)
            .await
            .expect("source should not fail to build");

        // If a timeout was given, use that, otherwise, use an infinitely long one.
        let source_timeout = sleep(timeout.unwrap_or_else(|| Duration::from_nanos(u64::MAX)));
        pin!(source_timeout);

        let mut events = Vec::new();

        // Try and drive both our timeout and the source itself, while collecting any events that the source sends out in
        // the meantime.  We store these locally and return them all at the end.
        loop {
            // If an event count was given, and we've hit it, break out of the loop.
            if let Some(count) = event_count {
                if events.len() == count {
                    break;
                }
            }

            select! {
                _ = &mut source_timeout => break,
                Some(event) = rx.next() => events.push(event),
                _ = &mut source => break,
            }
        }

        drop(source);

        // Drain any remaining events that we didn't get to before our timeout.
        //
        // If an event count was given, break out if we've reached the limit. Otherwise, just drain the remaining events
        // until no more are left, which avoids timing issues with missing events that came in right when the timeout
        // fired.
        while let Some(event) = rx.next().await {
            if let Some(count) = event_count {
                if events.len() == count {
                    break;
                }
            }

            events.push(event);
        }

        events
    })
    .await
}

/// Runs and returns a future and asserts that the provided test specification passes.
pub async fn assert_source<T>(
    tests: &LazyLock<ComponentTests>,
    tags: &[&str],
    f: impl Future<Output = T>,
) -> T {
    init_test();

    let result = f.await;

    tests.assert(tags);

    result
}
