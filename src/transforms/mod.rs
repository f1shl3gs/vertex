#[cfg(feature = "transforms-cardinality")]
mod cardinality;
#[cfg(feature = "transforms-dedup")]
mod dedup;
#[cfg(feature = "transforms-filter")]
mod filter;
#[cfg(feature = "transforms-geoip")]
mod geoip;
#[cfg(feature = "transforms-metricalize")]
mod metricalize;
#[cfg(feature = "transforms-modify")]
mod modify;
#[cfg(feature = "transforms-rewrite")]
mod rewrite;
#[cfg(feature = "transforms-route")]
mod route;
#[cfg(feature = "transforms-sample")]
mod sample;
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
