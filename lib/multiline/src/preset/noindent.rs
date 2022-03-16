use bytes::Bytes;

use crate::{Mode, Rule};

pub struct NoIndent;

impl Rule for NoIndent {
    fn is_start(&self, line: &Bytes) -> bool {
        if line.is_empty() {
            return false;
        }

        let b = line.as_ref();
        b[0] != b' ' && b[0] != b'\t'
    }

    fn is_condition(&self, line: &Bytes) -> bool {
        if line.is_empty() {
            return true;
        }

        let b = line.as_ref();
        b[0] == b' ' || b[0] == b'\t'
    }

    fn mode(&self) -> Mode {
        Mode::ContinueThrough
    }
}
