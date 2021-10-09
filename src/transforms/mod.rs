#[cfg(feature = "transforms-add_fields")]
mod add_fields;
#[cfg(feature = "transforms-add_tags")]
mod add_tags;
mod ansii_striper;
mod aggregate;

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

use std::pin::Pin;
use futures::Stream;
use event::Event;

/// Transforms that are simple, and don't require attention to coordination.
/// You can run them as simple functions over events in any order
pub trait FunctionTransform: Send + dyn_clone::DynClone + Sync {
    fn transform(&mut self, output: &mut Vec<Event>, event: Event);
}

dyn_clone::clone_trait_object!(FunctionTransform);

/// Transforms that tend to be more complicated runtime style componets.
///
/// These require coordination and map a stream of some `T` to some `U`
///
/// # Invariants
///
/// * It is an illegal invariant to implement `FunctionTransform` for a
/// `TaskTransform` or vice versa.
pub trait TaskTransform: Send {
    fn transform(
        self: Box<Self>,
        task: Pin<Box<dyn Stream<Item=Event> + Send>>,
    ) -> Pin<Box<dyn Stream<Item=Event> + Send>>;
}

/// Transforms come in two variants. Functions or tasks.
/// While function transforms can be run out of order or concurrently,
/// task transforms act as a coordination or barrier point.
pub enum Transform {
    Function(Box<dyn FunctionTransform>),
    Task(Box<dyn TaskTransform>),
}

impl Transform {
    /// Create a new function transform
    ///
    /// These functions are "stateless" and can be run in parallel,
    /// without regard for coordination.
    pub fn function(v: impl FunctionTransform + 'static) -> Self {
        Transform::Function(Box::new(v))
    }

    /// Create a new task transform.
    ///
    /// These tasks are coordinated, and map a stream of some `U` to some other
    /// `T`.
    ///
    /// **Note:** You should prefer to implement [`FunctionTransform`] over this
    /// where possible.
    pub fn task(v: impl TaskTransform + 'static) -> Self {
        Transform::Task(Box::new(v))
    }
}
