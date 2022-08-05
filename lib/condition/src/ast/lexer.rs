pub struct Lexer<'a> {
    text: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(s: &'a str) -> Self {
        Self {
            text: s.as_bytes(),
            pos: 0,
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        // trim whitespace
        while self.pos < self.text.len() {
            if !self.text[self.pos].is_ascii_whitespace() {
                break;
            }

            self.pos += 1;
        }

        if self.pos == self.text.len() {
            return None;
        }

        let start = self.pos;
        let c = self.text[self.pos];
        if c == b'(' || c == b')' {
            self.pos += 1;
        } else {
            // Consume util empty, (, ) and the end
            while self.pos < self.text.len() {
                let c = self.text[self.pos];
                if c == b'(' || c == b')' || c.is_ascii_whitespace() {
                    break;
                }

                self.pos += 1;
            }
        }

        let token =
            std::str::from_utf8(&self.text[start..self.pos]).expect("Convert to str failed");

        Some((start, token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iterate() {
        let tests = [
            ("", vec![]),
            ("foo(bar", vec![(0, "foo"), (3, "("), (4, "bar")]),
            ("foo(bar)", vec![(0, "foo"), (3, "("), (4, "bar"), (7, ")")]),
            (
                "foo(bar) ",
                vec![(0, "foo"), (3, "("), (4, "bar"), (7, ")")],
            ),
            ("foo bar)", vec![(0, "foo"), (4, "bar"), (7, ")")]),
            (
                "foo ( bar )",
                vec![(0, "foo"), (4, "("), (6, "bar"), (10, ")")],
            ),
            ("foo (", vec![(0, "foo"), (4, "(")]),
            ("foo", vec![(0, "foo")]),
            ("foo   \t   ", vec![(0, "foo")]),
            (" foo", vec![(1, "foo")]),
            ("foo bar", vec![(0, "foo"), (4, "bar")]),
            ("foo        bar", vec![(0, "foo"), (11, "bar")]),
            ("foo \tbar", vec![(0, "foo"), (5, "bar")]),
            (
                "foo bar foo \t bar",
                vec![(0, "foo"), (4, "bar"), (8, "foo"), (14, "bar")],
            ),
            (
                ".message contains info and (.upper gt 10 or .lower lt -1)",
                vec![
                    (0, ".message"),
                    (9, "contains"),
                    (18, "info"),
                    (23, "and"),
                    (27, "("),
                    (28, ".upper"),
                    (35, "gt"),
                    (38, "10"),
                    (41, "or"),
                    (44, ".lower"),
                    (51, "lt"),
                    (54, "-1"),
                    (56, ")"),
                ],
            ),
        ];

        for (input, want) in tests {
            let lexer = Lexer::new(input);
            let got = lexer.collect::<Vec<_>>();
            assert_eq!(
                want, got,
                "input: \"{}\", want: {:?}, got: {:?}",
                input, want, got
            );
        }
    }
}
