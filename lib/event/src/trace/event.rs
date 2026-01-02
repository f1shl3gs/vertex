use std::borrow::Cow;
use std::ops::Deref;

use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};
use typesize::TypeSize;

use super::Attributes;

/// Events record things that happened during a `Span`'s lifetime
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize)]
pub struct Event {
    /// The name of this event
    pub name: Cow<'static, str>,

    /// The time at which this event occurred.
    pub timestamp: i64,

    /// Attributes that describe this event
    pub attributes: Attributes,
}

impl TypeSize for Event {
    fn allocated_bytes(&self) -> usize {
        self.name.len() + self.attributes.allocated_bytes()
    }
}

impl Event {
    pub fn new(
        name: impl Into<Cow<'static, str>>,
        timestamp: i64,
        attributes: impl Into<Attributes>,
    ) -> Self {
        Self {
            name: name.into(),
            timestamp,
            attributes: attributes.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, PartialOrd)]
pub struct SpanEvents {
    /// The events stored as a vector. Could be empty if there are no events
    pub events: Vec<Event>,

    /// The number of Events dropped from the span
    pub dropped: u32,
}

impl Deref for SpanEvents {
    type Target = [Event];

    fn deref(&self) -> &Self::Target {
        &self.events
    }
}

impl From<Vec<Event>> for SpanEvents {
    fn from(events: Vec<Event>) -> Self {
        Self { events, dropped: 0 }
    }
}

impl IntoIterator for SpanEvents {
    type Item = Event;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.events.into_iter()
    }
}

impl Serialize for SpanEvents {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut arr = serializer.serialize_seq(Some(self.events.len()))?;
        for event in &self.events {
            arr.serialize_element(event)?;
        }

        arr.end()
    }
}

impl TypeSize for SpanEvents {
    #[inline]
    fn allocated_bytes(&self) -> usize {
        self.events.allocated_bytes()
    }
}

impl SpanEvents {
    pub fn push(&mut self, event: Event) {
        self.events.push(event);
    }
}
