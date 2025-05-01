use bytes::Bytes;

use crate::aggregate::{Mode, Rule};

#[derive(Debug)]
pub struct Regex {
    start_pattern: regex::bytes::Regex,

    condition_pattern: regex::bytes::Regex,

    mode: Mode,
}

impl Regex {
    pub fn new(
        start_pattern: regex::bytes::Regex,
        condition_pattern: regex::bytes::Regex,
        mode: Mode,
    ) -> Self {
        Self {
            start_pattern,
            condition_pattern,
            mode,
        }
    }
}

impl Rule for Regex {
    fn is_start(&mut self, line: &Bytes) -> bool {
        self.start_pattern.is_match(line)
    }

    fn is_condition(&mut self, line: &Bytes) -> bool {
        self.condition_pattern.is_match(line)
    }

    fn mode(&self) -> Mode {
        self.mode
    }
}
