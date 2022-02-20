mod compression;
pub mod metrics;
pub mod partition;

pub use compression::*;
use flate2::write::GzEncoder;
use std::io::Write;

use crate::batch::{Batch, BatchSize, PushResult};

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
    Plain(Vec<u8>),
    Gzip(GzEncoder<Vec<u8>>),
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
            let buffer = Vec::with_capacity(bytes);

            match compression {
                Compression::None => InnerBuffer::Plain(buffer),
                Compression::Gzip(level) => InnerBuffer::Gzip(GzEncoder::new(buffer, level)),
            }
        })
    }

    pub fn push(&mut self, input: &[u8]) {
        self.num_items += 1;

        match self.buffer() {
            InnerBuffer::Plain(inner) => {
                inner.extend_from_slice(input);
            }
            InnerBuffer::Gzip(inner) => {
                inner.write_all(input).unwrap();
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.inner
            .as_ref()
            .map(|inner| match inner {
                InnerBuffer::Plain(inner) => inner.is_empty(),
                InnerBuffer::Gzip(inner) => inner.get_ref().is_empty(),
            })
            .unwrap_or(true)
    }
}

impl Batch for Buffer {
    type Input = Vec<u8>;
    type Output = Vec<u8>;

    fn push(&mut self, item: Self::Input) -> PushResult<Self::Input> {
        todo!()
    }

    fn is_empty(&self) -> bool {
        todo!()
    }

    fn fresh(&self) -> Self {
        todo!()
    }

    fn finish(self) -> Self::Output {
        todo!()
    }

    fn num_items(&self) -> usize {
        todo!()
    }
}
