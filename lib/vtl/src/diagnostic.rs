#![allow(warnings)]
#![allow(clippy::print_stdout)]

use std::error::Error;
use std::fmt::Write;

use crate::compiler::Span;

pub struct Diagnostic {
    source: String,
}

pub trait SpannedError: Error {
    fn span(&self) -> Option<Span> {
        None
    }
}

impl Diagnostic {
    fn snippet<E>(&self, err: E) -> String
    where
        E: SpannedError,
    {
        let mut msg = err.to_string();
        let (start, end) = match err.span() {
            Some(span) => (span.start, span.end),
            None => return msg,
        };

        let mut consumed = 0;
        let mut lines = vec![];
        for (lo, line) in self.source.lines().enumerate() {
            if consumed + line.len() < start {
                consumed += line.len();
                continue;
            }

            if consumed > end && lines.len() > 2 {
                break;
            }

            lines.push((lo + 1, line));
            consumed += line.len();
        }

        for (lo, line) in lines {
            msg.write_fmt(format_args!("\n{lo} | {line}")).unwrap();
        }

        msg
    }
}

/*

error[E0308]: mismatched types
  --> src/format.rs:52:1
   |
51 |   ) -> Option<String> {
   |        -------------- expected `Option<String>` because of return type
52 | /     for ann in annotations {
53 | |         match (ann.range.0, ann.range.1) {
54 | |             (None, None) => continue,
55 | |             (Some(start), Some(end)) if start > end_index => continue,
...  |
71 | |         }
72 | |     }
   | |_____^ expected enum `std::option::Option`, found ()

*/

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::{Debug, Display, Formatter};

    #[derive(Debug)]
    struct TestError {
        msg: String,
        span: Span,
    }

    impl Error for TestError {}

    impl Display for TestError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            f.write_str(&self.msg)
        }
    }

    impl SpannedError for TestError {
        fn span(&self) -> Option<Span> {
            Some(self.span)
        }
    }

    #[test]
    fn snippets() {
        let source = "abcd\nefgh\nijkl\nmnop\nqrst\nuvwx\nyz";
        let err = TestError {
            msg: "blah".to_string(),
            span: Span { start: 6, end: 7 },
        };

        let diagnostic = Diagnostic {
            source: source.to_string(),
        };

        let output = diagnostic.snippet(err);
        println!("{}", output);
    }
}
