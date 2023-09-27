use std::fmt::{Display, Formatter};

use bytes::{Buf, BufMut};
use event::{proto, Event, Events};
use prost::Message;

use super::{encoding::AsMetadata, Encodable, EventCount};

impl EventCount for Event {
    fn event_count(&self) -> usize {
        1
    }
}

impl EventCount for Events {
    fn event_count(&self) -> usize {
        match self {
            Events::Logs(logs) => logs.len(),
            Events::Metrics(metrics) => metrics.len(),
            Events::Traces(traces) => traces.len(),
        }
    }
}

#[derive(Debug)]
pub enum EncodeError {
    BufferTooSmall,
}

impl Display for EncodeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the provided buffer was too small to fully encode this item"
        )
    }
}

impl std::error::Error for EncodeError {}

#[derive(Debug)]
pub enum DecodeError {
    InvalidProtobufPayload,
    UnsupportedEncodingMetadata,
}

impl Display for DecodeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::InvalidProtobufPayload => {
                write!(
                    f,
                    "the provided buffer could not be decoded as a valid Protocol Buffers payload"
                )
            }
            DecodeError::UnsupportedEncodingMetadata => {
                write!(f, "unsupported encoding metadata for this context")
            }
        }
    }
}

impl std::error::Error for DecodeError {}

/// Flags for describing the encoding scheme used by our primary event types that flow through buffers.
///
/// # Stability
///
/// This enumeration should never have any flags removed, only added.  This ensures that previously
/// used flags cannot have their meaning changed/repurposed after-the-fact.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EventEncodableMetadata(u32);

impl EventEncodableMetadata {
    const FLAG_VERSION_V1: Self = Self(0b1);
}

impl AsMetadata for EventEncodableMetadata {
    fn into_u32(self) -> u32 {
        self.0
    }

    fn from_u32(value: u32) -> Option<Self> {
        if value == 0b1 {
            Some(Self::FLAG_VERSION_V1)
        } else {
            None
        }
    }
}

impl Encodable for Events {
    type Metadata = EventEncodableMetadata;
    type EncodeError = EncodeError;
    type DecodeError = DecodeError;

    fn get_metadata() -> Self::Metadata {
        EventEncodableMetadata::FLAG_VERSION_V1
    }

    fn can_decode(metadata: Self::Metadata) -> bool {
        metadata == EventEncodableMetadata::FLAG_VERSION_V1
    }

    fn encode<B>(self, buf: &mut B) -> Result<(), Self::EncodeError>
    where
        B: BufMut,
    {
        proto::Events::from(self)
            .encode(buf)
            .map_err(|_| EncodeError::BufferTooSmall)
    }

    fn decode<B>(_metadata: Self::Metadata, buf: B) -> Result<Self, Self::DecodeError>
    where
        B: Buf + Clone,
    {
        proto::Events::decode(buf)
            .map(Into::into)
            .map_err(|_| DecodeError::InvalidProtobufPayload)
    }
}
