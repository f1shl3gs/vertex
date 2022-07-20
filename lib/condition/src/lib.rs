#![allow(dead_code)]

mod ast;
mod interpreter;
mod lexer;

#[derive(Debug)]
pub enum Error {
    Empty,
    UnknownOperator { pos: usize, found: String },
    PathExpected { pos: usize },
    RhsExpected { pos: usize },
    EarlyEOF { pos: usize },
    ExpectClosing { pos: usize, found: String },
    ExpectPathOrLeftParentheses { pos: usize, found: String },
}
