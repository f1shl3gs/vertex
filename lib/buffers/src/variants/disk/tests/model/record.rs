use std::{error, fmt, mem};

use bytes::{Buf, BufMut};
use finalize::{AddBatchNotifier, BatchNotifier, EventFinalizer, EventFinalizers};
use measurable::ByteSizeOf;

use crate::variants::disk::common::align16;
use crate::variants::disk::record::RECORD_HEADER_LEN;
use crate::{encoding::FixedEncodable, EventCount};

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
pub struct Record {
    id: u32,
    size: u32,
    event_count: u32,
    finalizers: EventFinalizers,
}

impl Record {
    pub(crate) const fn new(id: u32, size: u32, event_count: u32) -> Self {
        Record {
            id,
            size,
            event_count,
            finalizers: EventFinalizers::DEFAULT,
        }
    }

    const fn header_len() -> usize {
        mem::size_of::<u32>() * 3
    }

    const fn encoded_len(&self) -> usize {
        Self::header_len() + self.size as usize
    }

    pub const fn archived_len(&self) -> usize {
        // We kind of cheat here, because it's not the length of the actual record here, but the all-in length when we
        // write it to disk, which includes a wrapper type, and an overalignment of 16. If we don't do it here, or
        // account for it in some way, though, then our logic to figure out if the given record would be allowed based
        // on the configured `max_record_size` won't reflect reality, since the configuration builder _does_ take the
        // passed in `max_record_size`, less RECORD_HEADER_LEN, when calculating the number for how many bytes record
        // encoding can use.
        let encoded_len = self.encoded_len();
        align16(RECORD_HEADER_LEN + encoded_len)
    }
}

impl AddBatchNotifier for Record {
    fn add_batch_notifier(&mut self, notifier: BatchNotifier) {
        self.finalizers.add(EventFinalizer::new(notifier));
    }
}

impl EventCount for Record {
    fn event_count(&self) -> usize {
        self.event_count as usize
    }
}

impl ByteSizeOf for Record {
    fn allocated_bytes(&self) -> usize {
        0
    }
}

impl FixedEncodable for Record {
    type EncodeError = EncodeError;
    type DecodeError = DecodeError;

    fn encode<B>(self, buffer: &mut B) -> Result<(), Self::EncodeError>
    where
        B: BufMut,
        Self: Sized,
    {
        if buffer.remaining_mut() < self.encoded_len() {
            return Err(EncodeError);
        }

        buffer.put_u32(self.id);
        buffer.put_u32(self.size);
        buffer.put_u32(self.event_count);
        buffer.put_bytes(0x42, self.size as usize);
        Ok(())
    }

    fn encoded_size(&self) -> Option<usize> {
        Some(self.encoded_len())
    }

    fn decode<B>(mut buffer: B) -> Result<Self, Self::DecodeError>
    where
        B: Buf + Clone,
        Self: Sized,
    {
        if buffer.remaining() < Self::header_len() {
            return Err(DecodeError);
        }

        let id = buffer.get_u32();
        let size = buffer.get_u32();
        let event_count = buffer.get_u32();

        if buffer.remaining() < size as usize {
            return Err(DecodeError);
        }

        let payload = buffer.copy_to_bytes(size as usize);
        let valid = &payload.iter().all(|b| *b == 0x42);
        if !valid {
            return Err(DecodeError);
        }

        Ok(Record::new(id, size, event_count))
    }
}
