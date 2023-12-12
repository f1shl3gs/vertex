use std::error::Error;
use std::fmt::{Display, Formatter};

use value::Value;

use super::parser::Expr;
use super::{Expression, ExpressionError, Kind, TypeDef, ValueKind};
use crate::compiler::{Span, Spanned};
use crate::diagnostic::{DiagnosticMessage, Label};
use crate::Context;

#[derive(Debug)]
enum Variant {
    NonNumericNegate,
    NonBoolean,
    // maybe implement this?
    // FallibleOperand,
}

#[derive(Debug)]
pub struct UnaryError {
    variant: Variant,
    span: Span,
}

impl Display for UnaryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use Variant::*;

        match self.variant {
            NonNumericNegate => f.write_str("non numeric negate"),
            NonBoolean => f.write_str("non boolean negate"),
        }
    }
}

impl Error for UnaryError {}

impl DiagnosticMessage for UnaryError {
    fn labels(&self) -> Vec<Label> {
        match self.variant {
            Variant::NonNumericNegate => {
                vec![Label::new("only integer or float can be negate", self.span)]
            }
            Variant::NonBoolean => {
                vec![Label::new("only boolean allowed", self.span)]
            }
        }
    }
}

pub enum UnaryOp {
    // Arithmetic
    Negate, // -
    // Logical
    Not, // not
}

pub struct Unary {
    pub op: UnaryOp,
    pub operand: Box<Spanned<Expr>>,
}

impl Unary {
    pub fn new(op: UnaryOp, operand: Spanned<Expr>) -> Result<Unary, UnaryError> {
        let kind = operand.type_def().kind;
        match op {
            UnaryOp::Negate => {
                if !kind.intersects(Kind::NUMERIC) {
                    return Err(UnaryError {
                        variant: Variant::NonNumericNegate,
                        span: operand.span,
                    });
                }
            }
            UnaryOp::Not => {
                if !kind.intersects(Kind::BOOLEAN) {
                    return Err(UnaryError {
                        variant: Variant::NonBoolean,
                        span: operand.span,
                    });
                }
            }
        }

        Ok(Unary {
            op,
            operand: Box::new(operand),
        })
    }
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
                        span: self.operand.span,
                    })
                }
            }
            UnaryOp::Negate => match value {
                Value::Float(f) => Ok(Value::Float(-f)),
                Value::Integer(i) => Ok(Value::Integer(-i)),
                _ => Err(ExpressionError::UnexpectedType {
                    want: Kind::FLOAT | Kind::INTEGER,
                    got: value.kind(),
                    span: self.operand.span,
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
