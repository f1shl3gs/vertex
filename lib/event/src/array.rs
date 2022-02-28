use crate::Event;
use shared::ByteSizeOf;
use std::iter;

/// The core trait to abstract over any type that may work as an
/// array of events. This is effectively the same as the standard
/// `IntoIterator<Item = Event>` implementations, but that would
/// conflict with the base implementation for the type aliases
/// below.
pub trait EventContainer: ByteSizeOf {
    /// The type of `Iterator` used to turn this container into events.
    type IntoIter: Iterator<Item = Event>;

    /// The number of events in this container.
    fn len(&self) -> usize;

    /// Is this container empty?
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Turn this container into an iterator of events.
    fn into_events(self) -> Self::IntoIter;
}

impl EventContainer for Event {
    type IntoIter = iter::Once<Event>;

    fn len(&self) -> usize {
        1
    }

    fn is_empty(&self) -> bool {
        false
    }

    fn into_events(self) -> Self::IntoIter {
        iter::once(self)
    }
}
