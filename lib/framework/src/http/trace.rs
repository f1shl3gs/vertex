use std::str::FromStr;

use event::trace::{SpanContext, SpanId, TraceFlags, TraceId, TraceState};
use http::{HeaderValue, Request};
use tracing_internal::Context;

const SUPPORTED_VERSION: u8 = 0;
const MAX_VERSION: u8 = 254;
const TRACEPARENT_HEADER: &str = "traceparent";
const TRACESTATE_HEADER: &str = "tracestate";

pub fn inject<T>(cx: Context, req: &mut Request<T>) {
    if !cx.has_active_span() {
        return;
    }

    let span_ref = cx.span();
    let span_context = span_ref.span_context();
    if span_context.is_valid() {
        let header_value = format!(
            "{:02x}-{:032x}-{:016x}-{:02x}",
            SUPPORTED_VERSION,
            span_context.trace_id,
            span_context.span_id,
            span_context.trace_flags & TraceFlags::SAMPLED
        );

        let headers = req.headers_mut();
        if let Ok(val) = HeaderValue::from_str(&header_value) {
            headers.insert(TRACEPARENT_HEADER, val);
        }
        if let Ok(val) = HeaderValue::from_str(&span_context.trace_state.header()) {
            headers.insert(TRACESTATE_HEADER, val);
        }
    }
}

#[allow(dead_code)]
pub fn span_from_request<T>(req: &Request<T>) -> Result<SpanContext, ()> {
    let headers = req.headers();

    let value = headers
        .get(TRACEPARENT_HEADER)
        .ok_or(())?
        .to_str()
        .map_err(|_err| ())?;
    let parts = value.split_terminator('-').collect::<Vec<_>>();
    // Ensure parts are not out of range
    if parts.len() < 4 {
        return Err(());
    }

    // Ensure version is within range, for version 0 there must be 4 parts
    let version = u8::from_str_radix(parts[0], 16).map_err(|_| ())?;
    if version > MAX_VERSION || version == 0 && parts.len() != 4 {
        return Err(());
    }

    // Ensure trace id is lowercase
    if parts[1].chars().any(|c| c.is_ascii_uppercase()) {
        return Err(());
    }

    // Parse trace id section
    let trace_id = TraceId::from_hex(parts[1]).map_err(|_| ())?;

    // Ensure span id is lowercase
    if parts[2].chars().any(|c| c.is_ascii_uppercase()) {
        return Err(());
    }

    // Parse span id section
    let span_id = SpanId::from_hex(parts[2]).map_err(|_| ())?;

    // Parse trace flags section
    let opts = u8::from_str_radix(parts[3], 16).map_err(|_| ())?;

    // Ensure opts are valid for version 0
    if version == 0 && opts > 2 {
        return Err(());
    }

    // Build trace flags clearing all flags other than the trace-contet
    // supported sampling bit.
    let trace_flags = TraceFlags::new(opts) & TraceFlags::SAMPLED;

    let value = headers
        .get(TRACESTATE_HEADER)
        .map(|v| v.to_str().unwrap_or(""))
        .unwrap();

    let trace_state: TraceState =
        TraceState::from_str(value).unwrap_or_else(|_| TraceState::default());

    // Create context
    let span_context = SpanContext::new(trace_id, span_id, trace_flags, true, trace_state);

    // Ensure span is value
    if !span_context.is_valid() {
        return Err(());
    }

    Ok(span_context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::Body;
    use tracing_internal::SpanExt;

    #[ignore]
    #[test]
    fn inject_and_extract() {
        let mut req = Request::builder().uri("foo").body(Body::empty()).unwrap();

        assert!(req.headers().is_empty());

        crate::trace::init(false, false, "info", 10);
        let span = info_span!("foo");
        assert!(req.headers().is_empty());
        inject(span.context(), &mut req);
        assert!(!req.headers().is_empty())
    }
}
