use std::fmt::{Display, Formatter};

use bytes::{BufMut, BytesMut};
use value::Value;

use super::expression::Expression;
use super::parser::Expr;
use super::{ExpressionError, Kind, ValueKind};
use crate::compiler::{Span, TypeDef};
use crate::Context;

#[derive(Debug, PartialEq)]
pub enum BinaryError {
    Add(Kind, Kind),
    Subtract(Kind, Kind),
    Multiply(Kind, Kind),
    Divide(Kind, Kind),
    Exponent(Kind, Kind),

    // ge, gt, le, lt
    GreatEqual(Kind, Kind),
    GreatThan(Kind, Kind),
    LessEqual(Kind, Kind),
    LessThan(Kind, Kind),

    // %
    Rem(Kind, Kind),

    And(Kind, Kind),
    Or(Box<ExpressionError>),
}

impl Display for BinaryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryError::Add(lhs, rhs) => write!(f, "can't add type {rhs} to {lhs}"),
            BinaryError::Subtract(lhs, rhs) => write!(f, "can't subtract type {rhs} from {lhs}"),
            BinaryError::Multiply(lhs, rhs) => write!(f, "can't multiply type {lhs} by {rhs}"),
            BinaryError::Divide(lhs, rhs) => write!(f, "can't divide type {lhs} by {rhs}"),
            BinaryError::Exponent(lhs, rhs) => write!(f, "can't exponent type {lhs} by {rhs}"),
            BinaryError::Rem(lhs, rhs) => {
                write!(f, "can't calculate remainder of type {lhs} and {rhs}")
            }

            BinaryError::GreatEqual(lhs, rhs) => write!(f, "can't compare {lhs} >= {rhs}"),
            BinaryError::GreatThan(lhs, rhs) => write!(f, "can't compare {lhs} > {rhs}"),
            BinaryError::LessEqual(lhs, rhs) => write!(f, "can't compare {lhs} <= {rhs}"),
            BinaryError::LessThan(lhs, rhs) => write!(f, "can't compare {lhs} < {rhs}"),

            BinaryError::And(lhs, rhs) => write!(f, "can't apply an AND to type {lhs} and {rhs}"),
            BinaryError::Or(err) => write!(f, "can't apply an Or to types {err}"),
        }
    }
}

pub enum BinaryOp {
    // Arithmetic
    Add,      // +
    Subtract, // -
    Multiply, // *
    Divide,   // /
    Exponent, // ^

    // Relational
    Equal,
    NotEqual,
    LessThan,
    LessEqual,
    GreatThan,
    GreatEqual,

    // Logical
    And,
    Or,
}

macro_rules! smd {
    ($lhs:expr, $op:tt, $rhs:expr, $err:expr) => {
        match &($lhs, $rhs) {
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a $op b)),
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a $op b)),
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 $op b)),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a $op *b as f64)),
            (lhs, rhs) => {
                Err(ExpressionError::Binary {
                    err: $err(lhs.kind(), rhs.kind()),
                    span: Span {
                        start: 0,
                        end: 0,
                    }
                })
            },
        }
    };
}

macro_rules! compare {
    ($lhs:expr, $op:tt, $rhs:expr, $err:expr) => {
        match &($lhs, $rhs) {
            (Value::Integer(a), Value::Integer(b)) => Ok((a $op b).into()),
            (Value::Float(a), Value::Float(b)) => Ok((a $op b).into()),
            (Value::Integer(a), Value::Float(b)) => Ok(((*a as f64) $op *b).into()),
            (Value::Float(a), Value::Integer(b)) => Ok((*a $op (*b as f64)).into()),
            (Value::Bytes(a), Value::Bytes(b)) => Ok((a $op b).into()),
            (lhs, rhs) => Err(ExpressionError::Binary {
                err: $err(lhs.kind(), rhs.kind()),
                span: Span {
                    start: 0,
                    end: 0,
                },
            })
        }
    };
}

pub struct Binary {
    pub lhs: Box<Expr>,
    pub rhs: Box<Expr>,
    pub op: BinaryOp,
}

impl Expression for Binary {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        // optimize
        match self.op {
            BinaryOp::And => {
                let lhs = self.lhs.resolve(cx)?;
                return match lhs {
                    Value::Null => Ok(false.into()),
                    Value::Boolean(lhv) => {
                        let rhs = self.rhs.resolve(cx)?;
                        match rhs {
                            Value::Null => Ok(false.into()),
                            Value::Boolean(rhv) => Ok((lhv && rhv).into()),
                            _ => Err(ExpressionError::Binary {
                                err: BinaryError::And(lhs.kind(), rhs.kind()),
                                span: Span { start: 0, end: 0 },
                            }),
                        }
                    }
                    _ => {
                        let rhs = self.rhs.resolve(cx)?;
                        Err(ExpressionError::Binary {
                            err: BinaryError::And(lhs.kind(), rhs.kind()),
                            span: Span { start: 0, end: 0 },
                        })
                    }
                };
            }
            BinaryOp::Or => {
                let lhs = self.lhs.resolve(cx)?;
                return match lhs {
                    Value::Null | Value::Boolean(false) => {
                        self.rhs.resolve(cx).map_err(|err| ExpressionError::Binary {
                            err: BinaryError::Or(Box::new(err)),
                            span: Span { start: 0, end: 0 },
                        })
                    }
                    _ => Ok(lhs),
                };
            }
            _ => {
                // handle it blow
            }
        }

        let lhs = self.lhs.resolve(cx)?;
        let rhs = self.rhs.resolve(cx)?;

        match self.op {
            BinaryOp::Add => match (lhs, rhs) {
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a + b)),
                (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(a as f64 + b)),
                (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a + b as f64)),
                (lhs @ Value::Bytes(_), Value::Null) => Ok(lhs),
                (Value::Null, rhs @ Value::Bytes(_)) => Ok(rhs),
                (Value::Bytes(a), Value::Bytes(b)) => {
                    let mut buf = BytesMut::with_capacity(a.len() + b.len());
                    buf.put(a);
                    buf.put(b);
                    Ok(Value::Bytes(buf.freeze()))
                }
                (lhs, rhs) => Err(ExpressionError::Binary {
                    err: BinaryError::Add(lhs.kind(), rhs.kind()),
                    span: Span { start: 0, end: 0 },
                }),
            },
            BinaryOp::Subtract => smd!(lhs, -, rhs, BinaryError::Subtract),
            BinaryOp::Multiply => smd!(lhs, *, rhs, BinaryError::Multiply),
            BinaryOp::Divide => match &(lhs, rhs) {
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Float(*a as f64 / *b as f64)),
                (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(*a as f64 / b)),
                (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a / *b as f64)),
                (lhs, rhs) => Err(ExpressionError::Binary {
                    err: BinaryError::Divide(lhs.kind(), rhs.kind()),
                    span: Span { start: 0, end: 0 },
                }),
            },
            BinaryOp::Exponent => match &(lhs, rhs) {
                (Value::Integer(a), Value::Integer(b)) => {
                    println!("{} {} {}", *a, *b, (*a) ^ (*b));

                    Ok(Value::Integer(*a ^ *b))
                }
                (Value::Float(a), Value::Integer(b)) => {
                    let n = *b as i32; // todo: safe cast
                    let a = *a;
                    Ok(Value::Float(a.powi(n)))
                }
                (lhs, rhs) => Err(ExpressionError::Binary {
                    err: BinaryError::Exponent(lhs.kind(), rhs.kind()),
                    span: Span { start: 0, end: 0 },
                }),
            },

            // Logical operations
            BinaryOp::Equal => Ok(lhs.eq(&rhs).into()),
            BinaryOp::NotEqual => Ok((!lhs.eq(&rhs)).into()),
            BinaryOp::GreatEqual => compare!(lhs, >=, rhs, BinaryError::GreatEqual),
            BinaryOp::GreatThan => compare!(lhs, >, rhs, BinaryError::GreatThan),
            BinaryOp::LessEqual => compare!(lhs, <=, rhs, BinaryError::LessEqual),
            BinaryOp::LessThan => compare!(lhs, <, rhs, BinaryError::LessThan),

            _ => unreachable!("handled at the start"),
        }
    }

    fn type_def(&self) -> TypeDef {
        let lhs = self.lhs.type_def();
        let rhs = self.rhs.type_def();
        let fallible = lhs.fallible & rhs.fallible;

        let kind = match &self.op {
            BinaryOp::Add
            | BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Exponent => Kind::FLOAT | Kind::INTEGER,
            _ => Kind::BOOLEAN,
        };

        TypeDef { fallible, kind }
    }
}
