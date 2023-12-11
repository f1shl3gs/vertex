use std::fmt::{Display, Formatter, Write};

use crate::compiler::Span;

#[derive(Debug, PartialEq)]
pub enum LexError {
    // End of File
    Eof,
    // Unexpected character
    UnexpectedChar { ch: char, pos: usize },
    // Unable to parse the integer or float
    NumericLiteral { err: String, span: Span },
}

impl Display for LexError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LexError::Eof => f.write_str("unexpected eof"),
            LexError::UnexpectedChar { .. } => f.write_str("unexpected char"),
            LexError::NumericLiteral { err, .. } => {
                write!(f, "parse numeric token \"{}\" failed", err)
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Token<S> {
    Identifier(S),
    PathField(S),
    FunctionCall(S),

    // Operators
    Assign,     // =
    Not,        // !
    Add,        // +
    Subtract,   // -
    Multiply,   // *
    Divide,     // /
    Exponent,   // ^
    And,        // `and` or &&
    Or,         // `or` or ||
    Equal,      // ==
    NotEqual,   // !=
    GreatEqual, // >=
    GreatThan,  // >
    LessEqual,  // <=
    LessThan,   // <

    // Simple Literals
    String(S),
    Integer(i64),
    Float(f64),

    // Keywords, they must be in lowercase
    If,       // if
    Else,     // else
    For,      // for
    In,       // in
    Null,     // null
    False,    // false
    True,     // true
    Break,    // break
    Continue, // continue
    Return,   // return

    // Tokens
    Comma,        // ,
    Colon,        // :
    LeftParen,    // (
    RightParen,   // )
    LeftBracket,  // [
    RightBracket, // ]
    LeftBrace,    // {
    RightBrace,   // }
    Question,     // ?
}

impl<S> Display for Token<S>
where
    S: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Identifier(s) => s.fmt(f),
            Token::PathField(s) => s.fmt(f),
            Token::FunctionCall(s) => s.fmt(f),
            Token::Assign => f.write_char('='),
            Token::Not => f.write_char('!'),
            Token::Add => f.write_char('+'),
            Token::Subtract => f.write_char('-'),
            Token::Multiply => f.write_char('*'),
            Token::Divide => f.write_char('/'),
            Token::Exponent => f.write_char('^'),
            Token::And => f.write_str("and"),
            Token::Or => f.write_str("or"),
            Token::Equal => f.write_str("=="),
            Token::NotEqual => f.write_str("!="),
            Token::GreatEqual => f.write_str(">="),
            Token::GreatThan => f.write_char('>'),
            Token::LessEqual => f.write_str("<="),
            Token::LessThan => f.write_char('<'),
            Token::String(s) => s.fmt(f),
            Token::Integer(i) => write!(f, "{}", i),
            Token::Float(n) => write!(f, "{}", *n),
            Token::If => f.write_str("if"),
            Token::Else => f.write_str("{"),
            Token::For => f.write_str("for"),
            Token::In => f.write_str("in"),
            Token::Null => f.write_str("null"),
            Token::False => f.write_str("false"),
            Token::True => f.write_str("true"),
            Token::Break => f.write_str("break"),
            Token::Continue => f.write_str("continue"),
            Token::Return => f.write_str("return"),
            Token::Comma => f.write_char(','),
            Token::Colon => f.write_char(':'),
            Token::LeftParen => f.write_char('('),
            Token::RightParen => f.write_char(')'),
            Token::LeftBracket => f.write_char('['),
            Token::RightBracket => f.write_char(']'),
            Token::LeftBrace => f.write_char('{'),
            Token::RightBrace => f.write_char('}'),
            Token::Question => f.write_char('?'),
        }
    }
}

pub struct Lexer<'input> {
    text: &'input [u8],
    pos: usize,
}

impl<'input> Lexer<'input> {
    #[inline]
    pub fn new(input: &'input str) -> Self {
        Self {
            text: input.as_bytes(),
            pos: 0,
        }
    }

    pub fn next(&mut self) -> Option<Result<(Token<&'input str>, Span), LexError>> {
        self.skip_whitespace();

        if self.pos == self.text.len() {
            return None;
        }

        return match self.text[self.pos] {
            b'(' => {
                let start = self.pos;
                self.pos += 1;
                Some(Ok((
                    Token::LeftParen,
                    Span {
                        start,
                        end: self.pos,
                    },
                )))
            }
            b')' => {
                let start = self.pos;
                self.pos += 1;
                Some(Ok((
                    Token::RightParen,
                    Span {
                        start,
                        end: self.pos,
                    },
                )))
            }
            b'[' => {
                let start = self.pos;
                self.pos += 1;
                Some(Ok((
                    Token::LeftBracket,
                    Span {
                        start,
                        end: self.pos,
                    },
                )))
            }
            b']' => {
                let start = self.pos;
                self.pos += 1;
                Some(Ok((
                    Token::RightBracket,
                    Span {
                        start,
                        end: self.pos,
                    },
                )))
            }
            b'{' => {
                let start = self.pos;
                self.pos += 1;
                Some(Ok((
                    Token::LeftBrace,
                    Span {
                        start,
                        end: self.pos,
                    },
                )))
            }
            b'}' => {
                let start = self.pos;
                self.pos += 1;
                Some(Ok((
                    Token::RightBrace,
                    Span {
                        start,
                        end: self.pos,
                    },
                )))
            }
            b'?' => {
                let start = self.pos;
                self.pos += 1;
                Some(Ok((
                    Token::Question,
                    Span {
                        start,
                        end: self.pos,
                    },
                )))
            }
            b',' => {
                let start = self.pos;
                self.pos += 1;
                Some(Ok((
                    Token::Comma,
                    Span {
                        start,
                        end: self.pos,
                    },
                )))
            }
            b':' => {
                let start = self.pos;
                self.pos += 1;
                Some(Ok((
                    Token::Colon,
                    Span {
                        start,
                        end: self.pos,
                    },
                )))
            }

            // Operators
            b'+' => {
                let start = self.pos;
                self.pos += 1;
                Some(Ok((
                    Token::Add,
                    Span {
                        start,
                        end: self.pos,
                    },
                )))
            }
            b'-' => {
                let start = self.pos;
                self.pos += 1;
                Some(Ok((
                    Token::Subtract,
                    Span {
                        start,
                        end: self.pos,
                    },
                )))
            }
            // TODO: double slash for comment
            b'/' => {
                let start = self.pos;
                self.pos += 1;
                Some(Ok((
                    Token::Divide,
                    Span {
                        start,
                        end: self.pos,
                    },
                )))
            }
            b'*' => {
                let start = self.pos;
                self.pos += 1;
                Some(Ok((
                    Token::Multiply,
                    Span {
                        start,
                        end: self.pos,
                    },
                )))
            }
            b'^' => {
                let start = self.pos;
                self.pos += 1;
                Some(Ok((
                    Token::Exponent,
                    Span {
                        start,
                        end: self.pos,
                    },
                )))
            }
            b'>' => {
                let start = self.pos;
                self.pos += 1;
                if self.pos < self.text.len() && self.text[self.pos] == b'=' {
                    self.pos += 1;
                    return Some(Ok((
                        Token::GreatEqual,
                        Span {
                            start,
                            end: self.pos,
                        },
                    )));
                }

                return Some(Ok((
                    Token::GreatThan,
                    Span {
                        start,
                        end: self.pos,
                    },
                )));
            }

            b'<' => {
                let start = self.pos;
                self.pos += 1;
                if self.pos < self.text.len() && self.text[self.pos] == b'=' {
                    self.pos += 1;
                    return Some(Ok((
                        Token::LessEqual,
                        Span {
                            start,
                            end: self.pos,
                        },
                    )));
                }

                return Some(Ok((
                    Token::LessThan,
                    Span {
                        start,
                        end: self.pos,
                    },
                )));
            }

            // "&&"
            b'&' => {
                let start = self.pos;
                self.pos += 1;

                if self.pos < self.text.len() && self.text[self.pos] == b'&' {
                    self.pos += 1;

                    return Some(Ok((
                        Token::And,
                        Span {
                            start,
                            end: self.pos,
                        },
                    )));
                }

                return Some(Err(LexError::UnexpectedChar {
                    ch: '&',
                    pos: start,
                }));
            }

            // "||"
            b'|' => {
                let start = self.pos;
                self.pos += 1;

                if self.pos < self.text.len() && self.text[self.pos] == b'|' {
                    self.pos += 1;

                    return Some(Ok((
                        Token::Or,
                        Span {
                            start,
                            end: self.pos,
                        },
                    )));
                }

                return Some(Err(LexError::UnexpectedChar {
                    ch: '|',
                    pos: start,
                }));
            }

            // Comment
            b'#' => {
                while self.pos < self.text.len() {
                    if self.text[self.pos] == b'\n' {
                        return self.next();
                    }

                    self.pos += 1;
                }

                None
            }

            // Identifier, Function call or path
            b'a'..=b'z' | b'A'..=b'Z' => {
                let start = self.pos;

                let token = match self.identifier() {
                    // keywords
                    "if" => Token::If,
                    "else" => Token::Else,
                    "for" => Token::For,
                    "in" => Token::In,
                    "null" => Token::Null,
                    "return" => Token::Return,
                    "true" => Token::True,
                    "false" => Token::False,
                    "break" => Token::Break,
                    "continue" => Token::Continue,

                    s => {
                        // `foo(` is function call
                        // `foo (` is not a function call
                        if self.pos < self.text.len() && self.text[self.pos] == b'(' {
                            Token::FunctionCall(s)
                        } else if self.pos < self.text.len()
                            && (self.text[self.pos] == b'.' || self.text[self.pos] == b'[')
                        {
                            // take to whitespace
                            while self.pos < self.text.len() {
                                match self.text[self.pos] {
                                    b'0'..=b'9'
                                    | b'a'..=b'z'
                                    | b'A'..=b'Z'
                                    | b'_'
                                    | b'['
                                    | b']'
                                    | b'.' => {}
                                    _ => break,
                                }

                                self.pos += 1;
                            }

                            let s = unsafe {
                                std::str::from_utf8_unchecked(&self.text[start..self.pos])
                            };

                            Token::PathField(s)
                        } else {
                            Token::Identifier(s)
                        }
                    }
                };

                return Some(Ok((
                    token,
                    Span {
                        start,
                        end: self.pos,
                    },
                )));
            }

            // Numbers
            b'0'..=b'9' => Some(self.numeric_literal()),

            // Quoted string
            b'"' => Some(self.quota_string()),

            // Path identifier
            b'.' => return Some(self.path_identifier()),

            // metadata path
            b'%' => return Some(self.path_identifier()),

            b'=' => {
                let start = self.pos;
                self.pos += 1;
                if self.pos < self.text.len() && self.text[self.pos] == b'=' {
                    self.pos += 1;
                    return Some(Ok((
                        Token::Equal,
                        Span {
                            start,
                            end: self.pos,
                        },
                    )));
                }

                return Some(Ok((
                    Token::Assign,
                    Span {
                        start,
                        end: self.pos,
                    },
                )));
            }

            b'!' => {
                let start = self.pos;
                if self.pos < self.text.len() && self.text[self.pos] == b'=' {
                    self.pos += 1;
                    return Some(Ok((
                        Token::NotEqual,
                        Span {
                            start,
                            end: self.pos,
                        },
                    )));
                }

                return Some(Ok((
                    Token::Not,
                    Span {
                        start,
                        end: self.pos,
                    },
                )));
            }

            // Unknown char
            ch => Some(Err(LexError::UnexpectedChar {
                ch: ch as char,
                pos: self.pos,
            })),
        };
    }

    #[inline]
    pub fn peek(&mut self) -> Option<Result<(Token<&'input str>, Span), LexError>> {
        let pos = self.pos;
        let next = self.next();
        self.pos = pos;
        next
    }

    #[inline]
    pub fn pos(&self) -> usize {
        self.pos
    }

    #[inline]
    fn skip_whitespace(&mut self) {
        while self.pos < self.text.len() {
            let ch = self.text[self.pos];

            if ch == b'\n' {
                self.pos += 1;
                continue;
            }

            if !matches!(ch, b'\t' | b'\x0C' | b'\r' | b' ') {
                break;
            }

            self.pos += 1;
        }
    }

    fn identifier(&mut self) -> &'input str {
        let start = self.pos;

        while self.pos < self.text.len() {
            match self.text[self.pos] {
                b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'_' => {}
                _ => break,
            }

            self.pos += 1;
        }

        unsafe { std::str::from_utf8_unchecked(&self.text[start..self.pos]) }
    }

    fn quota_string(&mut self) -> Result<(Token<&'input str>, Span), LexError> {
        let start = self.pos;
        self.pos += 1;
        let mut quoting = false;

        while self.pos < self.text.len() {
            if quoting {
                self.pos += 1;
                quoting = false;
                continue;
            }

            let ch = self.text[self.pos];
            if ch == b'\\' {
                self.pos += 1;
                quoting = true;
                continue;
            }

            if self.text[self.pos] == b'"' {
                let s = unsafe { std::str::from_utf8_unchecked(&self.text[start + 1..self.pos]) };
                self.pos += 1;
                return Ok((
                    Token::String(s),
                    Span {
                        start,
                        end: self.pos,
                    },
                ));
            }

            self.pos += 1;
        }

        Err(LexError::Eof)
    }

    fn numeric_literal(&mut self) -> Result<(Token<&'input str>, Span), LexError> {
        let start = self.pos;
        let mut float = false;

        while self.pos < self.text.len() {
            let ch = self.text[self.pos];
            if !ch.is_ascii_digit() {
                if ch == b'.' {
                    float = true;
                } else {
                    break;
                }
            }

            self.pos += 1;
        }

        let s = unsafe { std::str::from_utf8_unchecked(&self.text[start..self.pos]) };
        if float {
            let f = s.parse::<f64>().map_err(|err| LexError::NumericLiteral {
                err: err.to_string(),
                span: Span {
                    start,
                    end: self.pos,
                },
            })?;
            Ok((
                Token::Float(f),
                Span {
                    start,
                    end: self.pos,
                },
            ))
        } else {
            let i = s.parse::<i64>().map_err(|err| LexError::NumericLiteral {
                err: err.to_string(),
                span: Span {
                    start,
                    end: self.pos,
                },
            })?;
            Ok((
                Token::Integer(i),
                Span {
                    start,
                    end: self.pos,
                },
            ))
        }
    }

    // `.` is event itself, `%` is metadata
    fn path_identifier(&mut self) -> Result<(Token<&'input str>, Span), LexError> {
        let start = self.pos;

        while self.pos < self.text.len() {
            match self.text[self.pos] {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'.' | b'[' | b']' | b'%' => {
                    self.pos += 1
                }
                _ => break,
            }
        }

        let s = unsafe { std::str::from_utf8_unchecked(&self.text[start..self.pos]) };

        Ok((
            Token::PathField(s),
            Span {
                start,
                end: self.pos,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use super::*;

    impl From<Range<usize>> for Span {
        fn from(value: Range<usize>) -> Self {
            Span {
                start: value.start,
                end: value.end,
            }
        }
    }

    #[allow(clippy::type_complexity)]
    fn assert_lex<const N: usize>(tests: [(&str, Vec<(Token<&str>, Range<usize>)>); N]) {
        for (input, want) in tests {
            let mut lexer = Lexer::new(input);
            let mut got = vec![];
            while let Some(result) = lexer.next() {
                let ts = result.expect(input);
                got.push(ts);
            }
            let want = want
                .into_iter()
                .map(|(token, range)| {
                    (
                        token,
                        Span {
                            start: range.start,
                            end: range.end,
                        },
                    )
                })
                .collect::<Vec<_>>();

            assert_eq!(got, want, "{input}")
        }
    }

    #[test]
    fn path() {
        assert_lex([
            (".foo.bar", vec![(Token::PathField(".foo.bar"), 0..8)]),
            ("%foo.bar", vec![(Token::PathField("%foo.bar"), 0..8)]),
            (
                ".foo.bar %foo.bar",
                vec![
                    (Token::PathField(".foo.bar"), 0..8),
                    (Token::PathField("%foo.bar"), 9..17),
                ],
            ),
            (
                ".foo.bar\n%foo.bar",
                vec![
                    (Token::PathField(".foo.bar"), 0..8),
                    (Token::PathField("%foo.bar"), 9..17),
                ],
            ),
            ("foo.bar", vec![(Token::PathField("foo.bar"), 0..7)]),
            ("arr[1]", vec![(Token::PathField("arr[1]"), 0..6)]),
        ])
    }

    #[test]
    fn identifier() {
        assert_lex([
            // we can found this in "for" statement or assign multiple variable
            (
                "k,v",
                vec![
                    (Token::Identifier("k"), 0..1),
                    (Token::Comma, 1..2),
                    (Token::Identifier("v"), 2..3),
                ],
            ),
            ("foo", vec![(Token::Identifier("foo"), 0..3)]),
            ("foo   ", vec![(Token::Identifier("foo"), 0..3)]),
            ("foo\n", vec![(Token::Identifier("foo"), 0..3)]),
            ("foo  \n", vec![(Token::Identifier("foo"), 0..3)]),
            ("\nfoo  \n", vec![(Token::Identifier("foo"), 1..4)]),
            (
                "foo bar",
                vec![
                    (Token::Identifier("foo"), 0..3),
                    (Token::Identifier("bar"), 4..7),
                ],
            ),
        ])
    }

    #[test]
    fn quoted_string() {
        assert_lex([(r#""foo bar""#, vec![(Token::String("foo bar"), 0..9)])])
    }

    #[test]
    fn comment() {
        assert_lex([
            ("# comment", vec![]),
            ("# comment\nfoo", vec![(Token::Identifier("foo"), 10..13)]),
            ("foo # abcdefg", vec![(Token::Identifier("foo"), 0..3)]),
        ])
    }

    #[test]
    fn keywords() {
        assert_lex([
            ("if", vec![(Token::If, 0..2)]),
            ("else", vec![(Token::Else, 0..4)]),
            ("null", vec![(Token::Null, 0..4)]),
            ("return", vec![(Token::Return, 0..6)]),
            ("true", vec![(Token::True, 0..4)]),
            ("false", vec![(Token::False, 0..5)]),
        ])
    }

    #[test]
    fn function() {
        assert_lex([(
            "now()",
            vec![
                (Token::FunctionCall("now"), 0..3),
                (Token::LeftParen, 3..4),
                (Token::RightParen, 4..5),
            ],
        )])
    }

    #[test]
    fn literal() {
        assert_lex([
            ("12345", vec![(Token::Integer(12345), 0..5)]),
            ("1.2345", vec![(Token::Float(1.2345), 0..6)]),
        ])
    }

    #[test]
    fn calc() {
        assert_lex([
            (
                "1+2-3*4/5",
                vec![
                    (Token::Integer(1), 0..1),
                    (Token::Add, 1..2),
                    (Token::Integer(2), 2..3),
                    (Token::Subtract, 3..4),
                    (Token::Integer(3), 4..5),
                    (Token::Multiply, 5..6),
                    (Token::Integer(4), 6..7),
                    (Token::Divide, 7..8),
                    (Token::Integer(5), 8..9),
                ],
            ),
            (
                "1 * 2 + 3",
                vec![
                    (Token::Integer(1), 0..1),
                    (Token::Multiply, 2..3),
                    (Token::Integer(2), 4..5),
                    (Token::Add, 6..7),
                    (Token::Integer(3), 8..9),
                ],
            ),
        ])
    }

    #[test]
    fn if_then() {
        assert_lex([(
            "if foo { }",
            vec![
                (Token::If, 0..2),
                (Token::Identifier("foo"), 3..6),
                (Token::LeftBrace, 7..8),
                (Token::RightBrace, 9..10),
            ],
        )])
    }

    #[test]
    fn if_then_else() {
        assert_lex([(
            "if foo { } else {}",
            vec![
                (Token::If, 0..2),
                (Token::Identifier("foo"), 3..6),
                (Token::LeftBrace, 7..8),
                (Token::RightBrace, 9..10),
                (Token::Else, 11..15),
                (Token::LeftBrace, 16..17),
                (Token::RightBrace, 17..18),
            ],
        )])
    }

    #[test]
    fn assign() {
        assert_lex([(
            r#".foo = "bar""#,
            vec![
                (Token::PathField(".foo"), 0..4),
                (Token::Assign, 5..6),
                (Token::String("bar"), 7..12), // 12 - 7 = 5, 5 = bar + " + "
            ],
        )])
    }

    #[test]
    fn json() {
        assert_lex([(
            "{\"foo\": 1}",
            vec![
                (Token::LeftBrace, 0..1),
                (Token::String("foo"), 1..6),
                (Token::Colon, 6..7),
                (Token::Integer(1), 8..9),
                (Token::RightBrace, 9..10),
            ],
        )])
    }
}
