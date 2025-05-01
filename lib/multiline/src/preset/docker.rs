use bytes::Bytes;

use crate::aggregate::{Mode, Rule};

pub struct Docker;

impl Rule for Docker {
    fn is_start(&mut self, _line: &Bytes) -> bool {
        true
    }

    fn is_condition(&mut self, _line: &Bytes) -> bool {
        false
    }

    fn mode(&self) -> Mode {
        Mode::ContinueThrough
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[allow(unused_variables)]
    fn merge() {
        let input = [
            "{\"log\": \"aa\\n\", \"stream\": \"stdout\", \"time\": \"2021-02-01T16:45:03.01231z\"}",
            "{\"log\": \"aa\\n\", \"stream\": \"stderr\", \"time\": \"2021-02-01T16:45:03.01231z\"}",
            "{\"log\": \"bb\", \"stream\": \"stdout\", \"time\": \"2021-02-01T16:45:03.01232z\"}",
            "{\"log\": \"cc\n\", \"stream\": \"stdout\", \"time\": \"2021-02-01T16:45:03.01233z\"}",
            "{\"log\": \"dd\", \"stream\": \"stderr\", \"time\": \"2021-02-01T16:45:03.01233z\"}",
            "single line to force pending flush of the previous line",
            "{\"log\": \"ee\\n\", \"stream\": \"stderr\", \"time\": \"2021-02-01T16:45:03.01234z\"}",
        ];

        let want = [
            "aa\n",
            "aa\n",
            "bbcc\n",
            "dd",
            "single line to force pending flush of the previous line",
            "ee\n",
        ];
    }
}
