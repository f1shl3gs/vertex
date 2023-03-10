use tracing::Span;

use crate::context::WithContext;
use crate::Context;

pub trait SpanExt {
    /// Extracts an [`Context`] from `self`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tracing::Span;
    /// use tracing_internal::SpanExt;
    ///
    /// // Generate a tracing span as usual
    /// let app_root = tracing::span!(tracing::Level::INFO, "app_start");
    ///
    /// // To include tracing context in client requests from _this_ app,
    /// // extract the current OpenTelemetry context.
    /// make_request(app_root.context());
    ///
    /// // Or if the current span has been created elsewhere:
    /// make_request(Span::current().context())
    /// ```
    fn context(&self) -> Context;
}

impl SpanExt for Span {
    fn context(&self) -> Context {
        let mut cx = None;

        let _ = self.with_subscriber(|(id, subscriber)| {
            if let Some(get_context) = subscriber.downcast_ref::<WithContext>() {
                get_context.with_context(subscriber, id, |builder, tracer| {
                    cx = Some(tracer.sampled_context(builder));
                })
            }
        });

        cx.unwrap_or_default()
    }
}
