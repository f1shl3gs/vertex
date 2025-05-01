use bytes::Bytes;

use crate::aggregate::{Mode, Rule};

pub struct NoIndent;

impl Rule for NoIndent {
    fn is_start(&mut self, line: &Bytes) -> bool {
        !line.as_ref()[0].is_ascii_whitespace()
    }

    fn is_condition(&mut self, line: &Bytes) -> bool {
        line.as_ref()[0].is_ascii_whitespace()
    }

    #[inline]
    fn mode(&self) -> Mode {
        Mode::ContinueThrough
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::assert_rule;

    #[tokio::test]
    async fn no_indent() {
        let input = [
            "111111111",
            "222222222",
            " 22222222",
            "333333333",
            " 33333333",
            " 33333333",
            "444444444",
            "555555555",
        ];
        let output = [
            "111111111",
            "222222222\n 22222222",
            "333333333\n 33333333\n 33333333",
            "444444444",
            "555555555",
        ];

        assert_rule(&input, &output, NoIndent).await;
    }
}
