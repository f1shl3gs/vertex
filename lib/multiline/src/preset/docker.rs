use bytes::Bytes;

use crate::{Mode, Rule};

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
