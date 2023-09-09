use std::pin::Pin;
use std::task::{Context, Poll};

use event::EventContainer;
use futures::Stream;
use measurable::ByteSizeOf;
use metrics::{Attributes, Counter};
use pin_project_lite::pin_project;

pin_project! {
    #[derive(Clone, Debug)]
    pub struct MetricRecorder<S> {
        #[pin]
        inner: S,

        // metrics
        received_events: Counter,
        received_bytes: Counter,
    }
}

impl<S> Stream for MetricRecorder<S>
where
    S: Stream + Unpin,
    S::Item: EventContainer,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        match this.inner.poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(item)) => {
                this.received_events.inc(item.len() as u64);
                this.received_bytes.inc(item.size_of() as u64);

                Poll::Ready(Some(item))
            }
        }
    }
}

pub trait MetricStreamExt: Stream {
    fn metric_record(self, attrs: Attributes) -> MetricRecorder<Self>
    where
        Self: Sized,
        Self::Item: EventContainer,
    {
        let received_events = metrics::register_counter(
            "component_received_events_total",
            "The number of events accepted by this component either from tagged origins like file and uri, or cumulatively from other origins.",
        ).recorder(attrs.clone());
        let received_bytes = metrics::register_counter(
            "component_received_event_bytes_total",
            "The number of event bytes accepted by this component either from tagged origins like file and uri, or cumulatively from other origins."
        ).recorder(attrs);

        MetricRecorder {
            inner: self,
            received_events,
            received_bytes,
        }
    }
}

impl<S: ?Sized> MetricStreamExt for S where S: Stream {}
