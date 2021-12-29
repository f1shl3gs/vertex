use crate::multiline::aggregate::{Mode, Rule};
use bytes::Bytes;

pub struct DockerParser;

impl Rule for DockerParser {
    fn is_start(&self, _line: &Bytes) -> bool {
        todo!()
    }

    fn is_condition(&self, _line: &Bytes) -> bool {
        todo!()
    }

    fn mode(&self) -> Mode {
        todo!()
    }
}
