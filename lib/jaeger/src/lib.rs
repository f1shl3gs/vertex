pub mod agent;
mod thrift;
mod translate;
mod transport;

use std::fmt::{Debug, Formatter};

pub use crate::thrift::jaeger::{Batch, Log, Process, Span, SpanRef, SpanRefType, Tag, TagType};

impl Debug for Tag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.v_type {
            TagType::String => write!(f, "Tag {{ key: {}, value: {:?} }}", self.key, self.v_str),
            TagType::Double => write!(f, "Tag {{ key: {}, value: {:?} }}", self.key, self.v_double),
            TagType::Bool => write!(f, "Tag {{ key: {}, value: {:?} }}", self.key, self.v_bool),
            TagType::Long => write!(f, "Tag {{ key: {}, value: {:?} }}", self.key, self.v_long),
            TagType::Binary => write!(f, "Tag {{ key: {}, value: {:?} }}", self.key, self.v_binary),
        }
    }
}
