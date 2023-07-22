mod compression;
pub mod metrics;
pub mod partition;
pub mod vec;

use std::io::Write;

use bytes::{BufMut, BytesMut};
pub use compression::*;
use flate2::write::{GzEncoder, ZlibEncoder};

use crate::batch::{err_event_too_large, Batch, BatchSize, PushResult};

#[derive(Debug)]
pub struct Buffer {
    inner: Option<InnerBuffer>,
    num_items: usize,
    num_bytes: usize,
    settings: BatchSize<Self>,
    compression: Compression,
}

#[derive(Debug)]
pub enum InnerBuffer {
    Plain(bytes::buf::Writer<BytesMut>),
    Gzip(GzEncoder<bytes::buf::Writer<BytesMut>>),
    Zlib(ZlibEncoder<bytes::buf::Writer<BytesMut>>),
}

impl Buffer {
    pub const fn new(settings: BatchSize<Self>, compression: Compression) -> Self {
        Self {
            inner: None,
            num_items: 0,
            num_bytes: 0,
            settings,
            compression,
        }
    }

    fn buffer(&mut self) -> &mut InnerBuffer {
        let bytes = self.settings.bytes;
        let compression = self.compression;

        self.inner.get_or_insert_with(|| {
            let writer = BytesMut::with_capacity(bytes).writer();

            match compression {
                Compression::None => InnerBuffer::Plain(writer),
                Compression::Gzip(level) => InnerBuffer::Gzip(GzEncoder::new(writer, level)),
                Compression::Zlib(level) => InnerBuffer::Zlib(ZlibEncoder::new(writer, level)),
            }
        })
    }

    pub fn push(&mut self, input: &[u8]) {
        self.num_items += 1;

        match self.buffer() {
            InnerBuffer::Plain(inner) => {
                inner.write_all(input).unwrap();
            }
            InnerBuffer::Gzip(inner) => {
                inner.write_all(input).unwrap();
            }
            InnerBuffer::Zlib(inner) => {
                inner.write_all(input).unwrap();
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.inner
            .as_ref()
            .map(|inner| match inner {
                InnerBuffer::Plain(inner) => inner.get_ref().is_empty(),
                InnerBuffer::Gzip(inner) => inner.get_ref().get_ref().is_empty(),
                InnerBuffer::Zlib(inner) => inner.get_ref().get_ref().is_empty(),
            })
            .unwrap_or(true)
    }
}

impl Batch for Buffer {
    type Input = BytesMut;
    type Output = BytesMut;

    fn push(&mut self, item: Self::Input) -> PushResult<Self::Input> {
        // The compressed encoders don't flush bytes immediately, so we
        // can't trace compressed sizes. Keep a running count of the
        // number of bytes written instead.
        let new_bytes = self.num_bytes + item.len();

        if self.is_empty() && item.len() > self.settings.bytes {
            err_event_too_large(item.len(), self.settings.bytes)
        } else if self.num_items >= self.settings.events || new_bytes > self.settings.bytes {
            PushResult::Overflow(item)
        } else {
            self.push(&item);
            self.num_bytes = new_bytes;

            PushResult::Ok(
                self.num_bytes >= self.settings.events || new_bytes >= self.settings.bytes,
            )
        }
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn fresh(&self) -> Self {
        Self::new(self.settings, self.compression)
    }

    fn finish(self) -> Self::Output {
        match self.inner {
            Some(InnerBuffer::Plain(inner)) => inner.into_inner(),
            Some(InnerBuffer::Gzip(inner)) => inner
                .finish()
                .expect("This can't fail because the inner writer is a Vec")
                .into_inner(),
            Some(InnerBuffer::Zlib(inner)) => inner
                .finish()
                .expect("This can't fail because the inner writer is a Vec")
                .into_inner(),
            None => BytesMut::new(),
        }
    }

    fn num_items(&self) -> usize {
        self.num_items
    }
}
