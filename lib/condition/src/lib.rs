mod ast;

use std::fmt::{Display, Formatter};
use std::num::ParseFloatError;

pub use ast::Expression;

#[derive(Debug, PartialEq)]
pub enum Error {
    // Parse
    PathExpected {
        pos: usize,
    },
    EarlyEOF,
    UnknownCombiningOp {
        pos: usize,
        token: String,
    },
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
    InvalidPath {
        path: String,
    },

    // Eval errors
    MissingField(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::PathExpected { pos } => write!(f, "path expected at {}", pos),
            Error::EarlyEOF => write!(f, "unexpected eof"),
            Error::UnknownCombiningOp { pos, token } => {
                write!(f, "unknown combining operator \"{}\" at {}", token, pos)
            }
            Error::InvalidNumber { pos, token, err } => {
                write!(f, "invalid number \"{}\" at {}, err: {}", token, pos, err)
            }
            Error::UnknownFieldOp { pos, token } => {
                write!(f, "unknown field operator \"{}\" at {}", token, pos)
            }
            Error::InvalidRegex { pos, token, err } => {
                write!(f, "invalid regex \"{}\" at {}, err: {}", token, pos, err)
            }
            Error::InvalidPath { path } => {
                write!(f, "invalid path \"{}\"", path)
            }
            Error::MissingField(field) => write!(f, "filed \"{}\" is not found", field),
        }
    }
}

impl std::error::Error for Error {}

pub fn parse(input: &str) -> Result<Expression, Error> {
    let mut parser = ast::Parser::new(input);

    parser.parse()
}
