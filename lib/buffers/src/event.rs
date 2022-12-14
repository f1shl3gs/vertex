use std::fmt::{Display, Formatter};

use bytes::{Buf, BufMut};
use enumflags2::{bitflags, BitFlags, FromBitsError};
use event::{proto, Event, Events};
use prost::Message;

use crate::encoding::AsMetadata;
use crate::{Encodable, EventCount};

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
#[allow(clippy::derive_partial_eq_without_eq)]
#[bitflags]
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum EventEncodableMetadataFlags {
    /// Chained encoding scheme that first tries to decode as `EventArray` and then as `Event`, as a
    /// way to support gracefully migrating existing v1-based disk buffers to the new
    /// `EventArray`-based architecture.
    ///
    /// All encoding uses the `EventArray` variant, however.
    DiskBufferV1CompatibilityMode = 0b1,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EventEncodableMetadata(BitFlags<EventEncodableMetadataFlags>);

impl EventEncodableMetadata {
    fn contains(self, flag: EventEncodableMetadataFlags) -> bool {
        self.0.contains(flag)
    }
}

impl From<EventEncodableMetadataFlags> for EventEncodableMetadata {
    fn from(flag: EventEncodableMetadataFlags) -> Self {
        Self(BitFlags::from(flag))
    }
}

impl From<BitFlags<EventEncodableMetadataFlags>> for EventEncodableMetadata {
    fn from(flags: BitFlags<EventEncodableMetadataFlags>) -> Self {
        Self(flags)
    }
}

impl TryFrom<u32> for EventEncodableMetadata {
    type Error = FromBitsError<EventEncodableMetadataFlags>;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        BitFlags::try_from(value).map(Self)
    }
}

impl AsMetadata for EventEncodableMetadata {
    fn into_u32(self) -> u32 {
        self.0.bits()
    }

    fn from_u32(value: u32) -> Option<Self> {
        EventEncodableMetadata::try_from(value).ok()
    }
}

impl Encodable for Events {
    type Metadata = EventEncodableMetadata;
    type EncodeError = EncodeError;
    type DecodeError = DecodeError;

    fn get_metadata() -> Self::Metadata {
        EventEncodableMetadataFlags::DiskBufferV1CompatibilityMode.into()
    }

    fn can_decode(metadata: Self::Metadata) -> bool {
        metadata.contains(EventEncodableMetadataFlags::DiskBufferV1CompatibilityMode)
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
