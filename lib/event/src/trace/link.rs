use std::ops::Deref;

use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};

use super::{Attributes, KeyValue, SpanContext, SpanId, TraceId};

/// Link is the relationship between two Spans
///
/// The relationship can be within the same trace or across different traces.
#[derive(Clone, Debug, PartialEq, PartialOrd, Serialize)]
#[non_exhaustive]
pub struct Link {
    /// The span context of the linked span
    pub span_context: SpanContext,

    /// Attributes that describe this link
    pub attributes: Attributes,
}

impl Link {
    pub fn new(span_context: SpanContext, attributes: Vec<KeyValue>) -> Self {
        Self {
            span_context,
            attributes: attributes.into(),
        }
    }

    pub fn trace_id(&self) -> TraceId {
        self.span_context.trace_id
    }

    pub fn span_id(&self) -> SpanId {
        self.span_context.span_id
    }
}

/// Stores span links along with dropped count
#[derive(Clone, Debug, Default, PartialEq, PartialOrd)]
#[non_exhaustive]
pub struct SpanLinks {
    /// The links stored as vector, could be empty if there are no links
    pub links: Vec<Link>,

    /// The number of links dropped from the span
    pub dropped: u32,
}

impl Deref for SpanLinks {
    type Target = [Link];

    fn deref(&self) -> &Self::Target {
        &self.links
    }
}

impl From<Vec<Link>> for SpanLinks {
    fn from(links: Vec<Link>) -> Self {
        Self { links, dropped: 0 }
    }
}

impl Serialize for SpanLinks {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut array = serializer.serialize_seq(Some(self.links.len()))?;
        for link in &self.links {
            array.serialize_element(link)?;
        }

        array.end()
    }
}

impl SpanLinks {
    pub fn push(&mut self, link: Link) {
        self.links.push(link);
    }
}
