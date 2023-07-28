#[cfg(feature = "transforms-add_fields")]
mod add_fields;
#[cfg(feature = "transforms-add_tags")]
mod add_tags;
#[cfg(feature = "transforms-cardinality")]
mod cardinality;
#[cfg(feature = "transforms-coercer")]
mod coercer;
#[cfg(feature = "transforms-dedup")]
mod dedup;
#[cfg(feature = "transforms-enum")]
mod r#enum;
#[cfg(feature = "transforms-filter")]
mod filter;
#[cfg(feature = "transforms-geoip")]
mod geoip;
#[cfg(feature = "transforms-json_parser")]
mod json_parser;
#[cfg(feature = "transforms-metricalize")]
mod metricalize;
#[cfg(feature = "transforms-route")]
mod route;
#[cfg(feature = "transforms-sample")]
mod sample;
#[cfg(feature = "transforms-substr")]
mod substr;
#[cfg(feature = "transforms-throttle")]
mod throttle;

/// Transform a single `Event` through the `FunctionTransform`
///
/// # Panics
///
/// If `ft` attempts to emit more than one `Event` on transform this function
/// will panic.
#[cfg(test)]
pub fn transform_one(
    ft: &mut dyn framework::FunctionTransform,
    event: impl Into<event::Event>,
) -> Option<event::Event> {
    use framework::OutputBuffer;

    let event = event.into();
    let mut buf = OutputBuffer::with_capacity(1);

    ft.transform(&mut buf, event.into());
    assert!(buf.len() < 2);
    buf.into_events().next()
}
