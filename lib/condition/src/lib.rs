mod ast;
mod error;
mod serde;

use ast::{CombiningOp, FieldExpr, Parser};
pub use error::Error;
use event::LogRecord;

#[derive(Clone, Debug)]
pub enum Expression {
    Field(FieldExpr),

    Binary {
        op: CombiningOp,
        lhs: Box<Expression>,
        rhs: Box<Expression>,
    },
    // support Unary !?
}

impl PartialEq for Expression {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Expression::Field(a), Expression::Field(b)) => a.eq(b),
            (
                Expression::Binary {
                    lhs: al,
                    op: ao,
                    rhs: ar,
                },
                Expression::Binary {
                    lhs: bl,
                    op: bo,
                    rhs: br,
                },
            ) => ao.eq(bo) && al.eq(bl) && ar.eq(br),
            _ => false,
        }
    }
}

impl Expression {
    fn boxed(self) -> Box<Self> {
        Box::new(self)
    }

    pub fn parse(input: impl AsRef<str>) -> Result<Self, Error> {
        Parser::new(input.as_ref()).parse()
    }

    pub fn eval(&self, log: &LogRecord) -> Result<bool, Error> {
        match self {
            Expression::Field(f) => f.eval(log),
            Expression::Binary { op, lhs, rhs } => match op {
                CombiningOp::And => Ok(lhs.eval(log)? && rhs.eval(log)?),
                CombiningOp::Or => Ok(lhs.eval(log)? || rhs.eval(log)?),
            },
        }
    }
}
