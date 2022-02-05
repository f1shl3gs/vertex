#[cfg(feature = "transforms-add_fields")]
mod add_fields;
#[cfg(feature = "transforms-add_tags")]
mod add_tags;
mod aggregate;
mod ansii_striper;

#[cfg(feature = "transforms-cardinality")]
mod cardinality;

mod filter;
mod geoip;
mod grok_parser;
mod jsmn_parser;
mod json_parser;
mod logfmt_parser;
mod rename_fields;
mod rename_tags;
mod route;
#[cfg(feature = "transforms-sample")]
mod sample;

use event::Event;

/// Transform a single `Event` through the `FunctionTransform`
///
/// # Panics
///
/// If `ft` attempts to emit more than one `Event` on transform this function
/// will panic.
#[cfg(test)]
pub fn transform_one(
    ft: &mut dyn framework::FunctionTransform,
    event: impl Into<Event>,
) -> Option<Event> {
    let mut buf = Vec::with_capacity(1);
    ft.transform(&mut buf, event.into());
    assert!(buf.len() < 2);
    buf.into_iter().next()
}
