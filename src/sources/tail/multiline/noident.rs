use tail::multiline::Logic;

#[derive(Clone)]
pub struct NoIndent;

impl Logic for NoIndent {
    fn is_start(&mut self, line: &[u8]) -> bool {
        if let [first, ..] = line {
            !first.is_ascii_whitespace()
        } else {
            // empty line
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::tail::multiline::assert_logic;

    #[test]
    fn merge() {
        let input = ["foo", "  bar", "  blah", "foo", "foo", "foo", "  bar"];
        let expected = [
            concat!("foo\n", "  bar\n", "  blah",),
            "foo",
            "foo",
            "foo\n  bar",
        ];

        assert_logic(NoIndent, input.as_slice(), expected.as_slice())
    }
}
