#![allow(dead_code)]

mod ast;
mod lexer;

#[derive(Debug)]
pub enum Error {
    Empty,
    UnexpectedToken {
        pos: usize,
        found: String,
    },
    UnknownOperator {
        pos: usize,
        found: String,
    },
    PathExpected {
        pos: usize,
    },
    RhsExpected {
        pos: usize,
    },
    EarlyEOF,
    ExpectClosing {
        pos: usize,
        found: String,
    },
    ExpectPathOrLeftParentheses {
        pos: usize,
        found: String,
    },

    // Parse
    UnexpectedCombiningOp(String),
    InvalidNumber {
        pos: usize,
        token: String,
    },
    UnknownFieldOp {
        pos: usize,
        token: String,
    },
    InvalidRegex {
        pos: usize,
        token: String,
        err: regex::Error,
    },

    // Eval error
    MissingField,
}
