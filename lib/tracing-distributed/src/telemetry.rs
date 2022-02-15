use super::trace::{Event, Span, SpanAttributeVisitor, SpanEventVisitor};

/// Represents the ability to publish events and spans to some arbitrary backend.
pub trait Telemetry {
    /// Report a `Span` to this Telemetry instance's backend.
    fn report_span(&self, span: Span<SpanAttributeVisitor>);

    /// Report an `Event` to this Telemetry instance's backend.
    fn report_event(&self, event: Event<SpanEventVisitor>);
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use std::sync::Arc;
    use std::sync::Mutex;

    /// Visitor that records no information when visiting tracing fields.
    #[derive(Default, Debug)]
    pub struct BlackholeVisitor;

    impl tracing::field::Visit for BlackholeVisitor {
        fn record_debug(&mut self, _: &tracing::field::Field, _: &dyn std::fmt::Debug) {}
    }

    /// Mock telemetry capability
    pub struct TestTelemetry {
        spans: Arc<Mutex<Vec<Span<SpanAttributeVisitor>>>>,
        events: Arc<Mutex<Vec<Event<SpanEventVisitor>>>>,
    }

    impl TestTelemetry {
        pub fn new(
            spans: Arc<Mutex<Vec<Span<SpanAttributeVisitor>>>>,
            events: Arc<Mutex<Vec<Event<SpanEventVisitor>>>>,
        ) -> Self {
            TestTelemetry { spans, events }
        }
    }

    impl Telemetry for TestTelemetry {
        fn report_span(&self, span: Span<SpanAttributeVisitor>) {
            // succeed or die. failure is unrecoverable (mutex poisoned)
            let mut spans = self.spans.lock().unwrap();
            spans.push(span);
        }

        fn report_event(&self, event: Event<SpanEventVisitor>) {
            // succeed or die. failure is unrecoverable (mutex poisoned)
            let mut events = self.events.lock().unwrap();
            events.push(event);
        }
    }
}
