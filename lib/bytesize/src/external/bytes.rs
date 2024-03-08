use bytes::{Bytes, BytesMut};

use crate::ByteSizeOf;

impl ByteSizeOf for Bytes {
    fn allocated_bytes(&self) -> usize {
        self.len()
    }
}

impl ByteSizeOf for BytesMut {
    fn allocated_bytes(&self) -> usize {
        self.len()
    }
}
