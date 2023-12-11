use value::Value;

use super::parser::Expr;
use super::{Expression, ExpressionError, Kind, TypeDef, ValueKind};
use crate::compiler::Span;
use crate::Context;

pub enum UnaryOp {
    // Arithmetic
    Negate, // -
    // Logical
    Not, // not
}

pub struct Unary {
    pub op: UnaryOp,
    pub operand: Box<Expr>,
}

impl Expression for Unary {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = self.operand.resolve(cx)?;

        match &self.op {
            UnaryOp::Not => {
                if let Value::Boolean(b) = value {
                    Ok(Value::Boolean(!b))
                } else {
                    Err(ExpressionError::UnexpectedType {
                        want: Kind::BOOLEAN,
                        got: value.kind(),
                        span: Span { start: 0, end: 0 },
                    })
                }
            }
            UnaryOp::Negate => match value {
                Value::Float(f) => Ok(Value::Float(-f)),
                Value::Integer(i) => Ok(Value::Integer(-i)),
                _ => Err(ExpressionError::UnexpectedType {
                    want: Kind::FLOAT | Kind::INTEGER,
                    got: value.kind(),
                    span: Span { start: 0, end: 0 },
                }),
            },
        }
    }

    fn type_def(&self) -> TypeDef {
        let kind = match self.op {
            UnaryOp::Not => Kind::BOOLEAN,
            UnaryOp::Negate => Kind::INTEGER | Kind::FLOAT,
        };

        TypeDef {
            fallible: false,
            kind,
        }
    }
}
