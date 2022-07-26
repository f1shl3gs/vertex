#![allow(dead_code)]

use std::num::ParseFloatError;

use crate::ast::Expression;

mod ast;
mod lexer;

#[derive(Debug, PartialEq)]
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
        err: ParseFloatError,
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

pub fn parse(input: &str) -> Result<Expression, Error> {
    let mut parser = ast::Parser::new(input);

    parser.parse()
}
