use tail::multiline::Logic;

#[derive(Clone, Debug)]
pub struct Regex {
    pub regex: regex::bytes::Regex,
}

impl Logic for Regex {
    #[inline]
    fn is_start(&mut self, line: &[u8]) -> bool {
        self.regex.is_match(line)
    }
}

#[cfg(test)]
impl Regex {
    pub fn new(pattern: &str) -> Result<Self, regex::Error> {
        let regex = regex::bytes::Regex::new(pattern)?;
        Ok(Self { regex })
    }
}

#[cfg(test)]
mod tests {
    use super::super::assert_logic;
    use super::Regex;

    #[test]
    fn merge() {
        let input = ["foo", "  bar", "  blah", "foo", "foo", "foo", "  bar"];
        let expected = [
            concat!("foo\n", "  bar\n", "  blah",),
            "foo",
            "foo",
            "foo\n  bar",
        ];

        assert_logic(
            Regex::new("^/w+").unwrap(),
            input.as_slice(),
            expected.as_slice(),
        )
    }
}
