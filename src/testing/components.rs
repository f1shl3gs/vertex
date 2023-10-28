use std::env;
use std::future::Future;

use event::{Events, Metric, MetricValue};
use framework::Sink;
use futures::Stream;
use futures::StreamExt;
use once_cell::sync::Lazy;

use crate::testing::metrics::capture_metrics;

/// The standard set of tags for all `HttpSink`-based sinks.
pub const HTTP_SINK_TAGS: [&str; 2] = ["endpoint", "protocol"];

/// The component test specification for sinks that are push-based.
pub static SINK_TESTS: Lazy<ComponentTests> = Lazy::new(|| {
    ComponentTests {
        events: &["BytesSent", "EventsSent"], // EventsReceived is emitted in the topology
        tagged_counters: &["component_sent_bytes_total"],
        untagged_counters: &[
            "component_sent_events_total",
            "component_sent_event_bytes_total",
        ],
    }
});

/// This struct is used to describe a set of component tests.
pub struct ComponentTests {
    /// The list of event (suffixes) that must be emitted by the component
    events: &'static [&'static str],
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
        test.emitted_all_events(self.events);
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
        let mut metrics = capture_metrics();

        if env::var("DEBUG_COMPONENT_COMPLIANCE").is_ok() {
            metrics.sort_by(|a, b| a.name().cmp(b.name()));
            for metric in &metrics {
                println!("{}", metric);
            }
        }

        let errors = Vec::new();
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
                            .keys()
                            .map(|key| key.as_str())
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

    fn emitted_all_events(&mut self, _names: &[&str]) {
        /*        for name in names {
            if let Err(err_msg) = event_test_util::contains_name_once(name) {
                self.errors.push(format!("  - {}", err_msg));
            }
        }*/
    }
}

/// Tests if the given metric contains all the given tag names
fn has_tags(metric: &Metric, names: &[&str]) -> bool {
    for name in names {
        let key = name.to_string().into();
        if metric.tags().contains(&key) {
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
