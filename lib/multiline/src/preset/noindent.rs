use bytes::Bytes;

use crate::{Mode, Rule};

pub struct NoIndent;

impl Rule for NoIndent {
    fn is_start(&self, line: &Bytes) -> bool {
        let b = line.as_ref();
        b[0] != b' ' && b[0] != b'\t'
    }

    fn is_condition(&self, line: &Bytes) -> bool {
        let b = line.as_ref();
        b[0] == b' ' || b[0] == b'\t'
    }

    #[inline]
    fn mode(&self) -> Mode {
        Mode::ContinueThrough
    }
}
