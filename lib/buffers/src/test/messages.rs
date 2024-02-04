#![allow(clippy::redundant_closure_call)]

use std::{error, fmt, io, mem};

use bytes::{Buf, BufMut};
use event::{AddBatchNotifier, BatchNotifier, EventFinalizer, EventFinalizers, Finalizable};
use measurable::ByteSizeOf;
use quickcheck::{Arbitrary, Gen};

use crate::{encoding::FixedEncodable, EventCount};

macro_rules! message_wrapper {
    ($id:ident: $ty:ty, $event_count:expr) => {
        #[derive(Clone, Debug, Eq)]
        pub(crate) struct $id(pub $ty, EventFinalizers);

        impl $id {
            pub const fn new(value: $ty) -> Self {
                Self(value, EventFinalizers::DEFAULT)
            }
        }

        impl AddBatchNotifier for $id {
            fn add_batch_notifier(&mut self, batch: BatchNotifier) {
                self.1.add(EventFinalizer::new(batch));
            }
        }

        impl ByteSizeOf for $id {
            fn allocated_bytes(&self) -> usize {
                0
            }
        }

        impl EventCount for $id {
            fn event_count(&self) -> usize {
                usize::try_from($event_count(self)).unwrap_or(usize::MAX)
            }
        }

        impl Finalizable for $id {
            fn take_finalizers(&mut self) -> EventFinalizers {
                std::mem::take(&mut self.1)
            }
        }

        impl PartialEq for $id {
            fn eq(&self, other: &Self) -> bool {
                self.0 == other.0
            }
        }
    };
}

#[derive(Debug)]
pub struct EncodeError;

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl error::Error for EncodeError {}

#[derive(Debug)]
pub struct DecodeError;

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl error::Error for DecodeError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    id: u64,
}

impl Message {
    pub(crate) fn new(id: u64) -> Self {
        Message { id }
    }
}

impl ByteSizeOf for Message {
    fn allocated_bytes(&self) -> usize {
        0
    }
}

impl EventCount for Message {
    fn event_count(&self) -> usize {
        1
    }
}

impl Arbitrary for Message {
    fn arbitrary(g: &mut Gen) -> Self {
        Message {
            id: u64::arbitrary(g),
        }
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        Box::new(self.id.shrink().map(|id| Message { id }))
    }
}

impl FixedEncodable for Message {
    type EncodeError = EncodeError;
    type DecodeError = DecodeError;

    fn encode<B>(self, buffer: &mut B) -> Result<(), Self::EncodeError>
    where
        B: BufMut,
        Self: Sized,
    {
        buffer.put_u64(self.id);
        Ok(())
    }

    fn encoded_size(&self) -> Option<usize> {
        Some(mem::size_of::<u64>())
    }

    fn decode<B>(mut buffer: B) -> Result<Self, Self::DecodeError>
    where
        B: Buf,
        Self: Sized,
    {
        let id = buffer.get_u64();
        Ok(Message::new(id))
    }
}

message_wrapper!(SizedRecord: u32, |_| 1);

impl SizedRecord {
    pub fn encoded_len(&self) -> usize {
        let payload_len: usize = self
            .0
            .try_into()
            .expect("`SizedRecord` should never have a payload length greater than `usize`.");

        payload_len + mem::size_of_val(&self.0)
    }
}

impl FixedEncodable for SizedRecord {
    type EncodeError = io::Error;
    type DecodeError = io::Error;

    fn encode<B>(self, buffer: &mut B) -> Result<(), Self::EncodeError>
    where
        B: BufMut,
    {
        if buffer.remaining_mut() < self.0 as usize + 4 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "not enough capacity to encode record",
            ));
        }

        buffer.put_u32(self.0);
        buffer.put_bytes(0x42, self.0 as usize);
        Ok(())
    }

    fn encoded_size(&self) -> Option<usize> {
        Some(self.encoded_len())
    }

    fn decode<B>(mut buffer: B) -> Result<Self, Self::DecodeError>
    where
        B: Buf,
    {
        let buf_len = buffer.get_u32();
        buffer.advance(buf_len as usize);
        Ok(SizedRecord::new(buf_len))
    }
}

#[test]
fn sized_record() {
    let record = SizedRecord::new(12);
    let encode_len = record.encoded_len();

    let mut buf = Vec::new();
    record.encode(&mut buf).unwrap();

    assert_eq!(encode_len, buf.len())
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct UndecodableRecord;

impl AddBatchNotifier for UndecodableRecord {
    fn add_batch_notifier(&mut self, notifier: BatchNotifier) {
        drop(notifier);
    }
}

impl ByteSizeOf for UndecodableRecord {
    fn allocated_bytes(&self) -> usize {
        0
    }
}

impl EventCount for UndecodableRecord {
    fn event_count(&self) -> usize {
        1
    }
}

impl FixedEncodable for UndecodableRecord {
    type EncodeError = io::Error;
    type DecodeError = io::Error;

    fn encode<B>(self, buffer: &mut B) -> Result<(), Self::EncodeError>
    where
        B: BufMut,
    {
        if buffer.remaining_mut() < 4 {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "not enough capacity to encode record",
            ));
        }

        buffer.put_u32(42);
        Ok(())
    }

    fn decode<B>(_buffer: B) -> Result<Self, Self::DecodeError>
    where
        B: Buf,
    {
        Err(io::Error::new(io::ErrorKind::Other, "failed to decode"))
    }
}

message_wrapper!(MultiEventRecord: u32, |m: &Self| m.0);

impl MultiEventRecord {
    pub fn encoded_len(&self) -> usize {
        usize::try_from(self.0).unwrap_or(usize::MAX) + mem::size_of::<u32>()
    }
}

impl FixedEncodable for MultiEventRecord {
    type EncodeError = io::Error;
    type DecodeError = io::Error;

    fn encode<B>(self, buffer: &mut B) -> Result<(), Self::EncodeError>
    where
        B: BufMut,
    {
        if buffer.remaining_mut() < self.encoded_len() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "not enough capacity to encode record",
            ));
        }

        buffer.put_u32(self.0);
        buffer.put_bytes(0x42, usize::try_from(self.0).unwrap_or(usize::MAX));
        Ok(())
    }

    fn decode<B>(mut buffer: B) -> Result<Self, Self::DecodeError>
    where
        B: Buf,
    {
        let event_count = buffer.get_u32();
        buffer.advance(usize::try_from(event_count).unwrap_or(usize::MAX));
        Ok(MultiEventRecord::new(event_count))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PoisonPillMultiEventRecord(pub u32);

impl PoisonPillMultiEventRecord {
    pub fn encoded_size(&self) -> usize {
        usize::try_from(self.0).unwrap_or(usize::MAX) + mem::size_of::<u32>()
    }
}

impl ByteSizeOf for PoisonPillMultiEventRecord {
    fn allocated_bytes(&self) -> usize {
        0
    }
}

impl EventCount for PoisonPillMultiEventRecord {
    fn event_count(&self) -> usize {
        usize::try_from(self.0).unwrap_or(usize::MAX)
    }
}

impl FixedEncodable for PoisonPillMultiEventRecord {
    type EncodeError = io::Error;
    type DecodeError = io::Error;

    fn encode<B>(self, buffer: &mut B) -> Result<(), Self::EncodeError>
    where
        B: BufMut,
    {
        if buffer.remaining_mut() < self.encoded_size() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "not enough capacity to encode record",
            ));
        }

        buffer.put_u32(self.0);
        buffer.put_bytes(0x42, usize::try_from(self.0).unwrap_or(usize::MAX));
        Ok(())
    }

    fn decode<B>(mut buffer: B) -> Result<Self, Self::DecodeError>
    where
        B: Buf,
    {
        let event_count = buffer.get_u32();
        if event_count == 42 {
            return Err(io::Error::new(io::ErrorKind::Other, "failed to decode"));
        }

        buffer.advance(usize::try_from(event_count).unwrap_or(usize::MAX));
        Ok(PoisonPillMultiEventRecord(event_count))
    }
}
