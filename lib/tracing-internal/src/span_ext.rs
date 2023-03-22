use crate::context::WithContext;
use crate::Context;

/// Utility functions to allow tracing [`Span`]s to accept and return ours [`Context`]s.
pub trait SpanExt {
    /// Extracts a Context from self
    fn context(&self) -> Context;
}

impl SpanExt for tracing::Span {
    fn context(&self) -> Context {
        let mut cx = None;
        self.with_subscriber(|(id, collector)| {
            if let Some(get_context) = collector.downcast_ref::<WithContext>() {
                get_context.with_context(collector, id, |builder, tracer| {
                    cx = Some(tracer.sampled_context(builder));
                })
            }
        });

        cx.unwrap_or_default()
    }
}
