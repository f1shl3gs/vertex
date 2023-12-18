use std::error::Error;
use std::fmt::{Display, Formatter, Write};

use bytes::{BufMut, BytesMut};
use value::Value;

use super::expr::Expr;
use super::span::{Span, Spanned};
use super::state::TypeState;
use super::{Expression, TypeDef};
use super::{ExpressionError, Kind, ValueKind};
use crate::context::Context;
use crate::diagnostic::{DiagnosticMessage, Label};

#[derive(Debug, PartialEq)]
pub enum BinaryError {
    Add(Kind, Kind),
    Subtract(Kind, Kind),
    Multiply(Kind, Kind),
    Divide(Kind, Kind),
    DivideZero,
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
            BinaryError::DivideZero => f.write_str("divide by zero"),
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

impl Error for BinaryError {}

impl From<BinaryError> for ExpressionError {
    fn from(err: BinaryError) -> Self {
        ExpressionError::Binary {
            err,
            span: Span { start: 0, end: 0 },
        }
    }
}

impl DiagnosticMessage for BinaryError {
    fn labels(&self) -> Vec<Label> {
        vec![]
    }
}

#[derive(Clone)]
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

impl Display for BinaryOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryOp::Add => f.write_char('+'),
            BinaryOp::Subtract => f.write_char('-'),
            BinaryOp::Multiply => f.write_char('*'),
            BinaryOp::Divide => f.write_char('/'),
            BinaryOp::Exponent => f.write_char('^'),
            BinaryOp::Equal => f.write_str("=="),
            BinaryOp::NotEqual => f.write_str("!="),
            BinaryOp::LessThan => f.write_char('<'),
            BinaryOp::LessEqual => f.write_str("<="),
            BinaryOp::GreatThan => f.write_char('>'),
            BinaryOp::GreatEqual => f.write_str(">="),
            BinaryOp::And => f.write_str("&&"),
            BinaryOp::Or => f.write_str("||"),
        }
    }
}

#[derive(Clone)]
pub struct Binary {
    pub lhs: Box<Spanned<Expr>>,
    pub rhs: Box<Spanned<Expr>>,
    pub op: BinaryOp,
}

impl Binary {
    pub fn compile(
        lhs: Spanned<Expr>,
        op: BinaryOp,
        rhs: Spanned<Expr>,
    ) -> Result<Expr, BinaryError> {
        // optimize
        let lhs_span = lhs.span;
        let rhs_span = rhs.span;
        let expr = match (lhs.node, op, rhs.node) {
            // float op float
            (Expr::Float(a), BinaryOp::Add, Expr::Float(b)) => Expr::Float(a + b),
            (Expr::Float(a), BinaryOp::Subtract, Expr::Float(b)) => Expr::Float(a - b),
            (Expr::Float(a), BinaryOp::Multiply, Expr::Float(b)) => Expr::Float(a * b),
            (Expr::Float(a), BinaryOp::Divide, Expr::Float(b)) => {
                if b == 0.0 {
                    return Err(BinaryError::DivideZero);
                }

                Expr::Float(a / b)
            }

            // integer op integer
            (Expr::Integer(a), BinaryOp::Add, Expr::Integer(b)) => Expr::Integer(a + b),
            (Expr::Integer(a), BinaryOp::Subtract, Expr::Integer(b)) => Expr::Integer(a - b),
            (Expr::Integer(a), BinaryOp::Multiply, Expr::Integer(b)) => Expr::Integer(a * b),
            (Expr::Integer(a), BinaryOp::Divide, Expr::Integer(b)) => {
                if b == 0 {
                    return Err(BinaryError::DivideZero);
                }

                Expr::Float(a as f64 / b as f64)
            }

            // integer op float
            (Expr::Integer(a), BinaryOp::Add, Expr::Float(b)) => Expr::Float(a as f64 + b),
            (Expr::Integer(a), BinaryOp::Subtract, Expr::Float(b)) => Expr::Float(a as f64 - b),
            (Expr::Integer(a), BinaryOp::Multiply, Expr::Float(b)) => Expr::Float(a as f64 * b),
            (Expr::Integer(a), BinaryOp::Divide, Expr::Float(b)) => {
                if b == 0.0 {
                    return Err(BinaryError::DivideZero);
                }

                Expr::Float(a as f64 / b)
            }

            // float op integer
            (Expr::Float(a), BinaryOp::Add, Expr::Integer(b)) => Expr::Float(a + b as f64),
            (Expr::Float(a), BinaryOp::Subtract, Expr::Integer(b)) => Expr::Float(a - b as f64),
            (Expr::Float(a), BinaryOp::Multiply, Expr::Integer(b)) => Expr::Float(a * b as f64),
            (Expr::Float(a), BinaryOp::Divide, Expr::Integer(b)) => {
                if b == 0 {
                    return Err(BinaryError::DivideZero);
                }

                Expr::Float(a / b as f64)
            }

            // string + string
            (Expr::String(a), BinaryOp::Add, Expr::String(b)) => {
                let mut buf = BytesMut::with_capacity(a.len() + b.len());
                buf.extend_from_slice(&a);
                buf.extend_from_slice(&b);
                Expr::String(buf.freeze())
            }

            (lhs, op, rhs) => Expr::Binary(Binary {
                lhs: Box::new(lhs.with(lhs_span)),
                rhs: Box::new(rhs.with(rhs_span)),
                op,
            }),
        };

        Ok(expr)
    }
}

impl Expression for Binary {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        // optimize for `&&` or `||`
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
                                span: self.lhs.span.merge(self.rhs.span),
                            }),
                        }
                    }
                    _ => {
                        let rhs = self.rhs.resolve(cx)?;
                        Err(ExpressionError::Binary {
                            err: BinaryError::And(lhs.kind(), rhs.kind()),
                            span: self.lhs.span.merge(self.rhs.span),
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
                            span: self.lhs.span.merge(self.rhs.span),
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
                    span: self.lhs.span.merge(self.rhs.span),
                }),
            },
            BinaryOp::Subtract => match (lhs, rhs) {
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a - b)),
                (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(a as f64 - b)),
                (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a - b as f64)),
                (lhs, rhs) => Err(ExpressionError::Binary {
                    err: BinaryError::Subtract(lhs.kind(), rhs.kind()),
                    span: self.lhs.span.merge(self.rhs.span),
                }),
            },
            BinaryOp::Multiply => match (lhs, rhs) {
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a * b)),
                (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(a as f64 * b)),
                (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a * b as f64)),
                (lhs, rhs) => Err(ExpressionError::Binary {
                    err: BinaryError::Multiply(lhs.kind(), rhs.kind()),
                    span: self.lhs.span.merge(self.rhs.span),
                }),
            },
            BinaryOp::Divide => match (lhs, rhs) {
                (Value::Float(a), Value::Float(b)) => {
                    if b == 0.0 {
                        return Err(ExpressionError::Binary {
                            err: BinaryError::DivideZero,
                            span: self.lhs.span.merge(self.rhs.span),
                        });
                    }

                    Ok((a / b).into())
                }
                (Value::Integer(a), Value::Integer(b)) => {
                    if b == 0 {
                        return Err(ExpressionError::Binary {
                            err: BinaryError::DivideZero,
                            span: self.lhs.span.merge(self.rhs.span),
                        });
                    }

                    Ok((a as f64 / b as f64).into())
                }
                (Value::Integer(a), Value::Float(b)) => {
                    if b == 0.0 {
                        return Err(ExpressionError::Binary {
                            err: BinaryError::DivideZero,
                            span: self.lhs.span.merge(self.rhs.span),
                        });
                    }

                    Ok((a as f64 / b).into())
                }
                (Value::Float(a), Value::Integer(b)) => {
                    if b == 0 {
                        return Err(ExpressionError::Binary {
                            err: BinaryError::DivideZero,
                            span: self.lhs.span.merge(self.rhs.span),
                        });
                    }

                    Ok((a / b as f64).into())
                }

                (lhs, rhs) => Err(ExpressionError::Binary {
                    err: BinaryError::Divide(lhs.kind(), rhs.kind()),
                    span: self.lhs.span.merge(self.rhs.span),
                }),
            },
            BinaryOp::Exponent => match (lhs, rhs) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a.pow(b as u32))),
                (Value::Float(a), Value::Integer(b)) => {
                    // todo: safe cast
                    Ok(Value::Float(a.powi(b as i32)))
                }
                (lhs, rhs) => Err(ExpressionError::Binary {
                    err: BinaryError::Exponent(lhs.kind(), rhs.kind()),
                    span: self.lhs.span.merge(self.rhs.span),
                }),
            },

            // Logical operations
            BinaryOp::Equal => Ok(lhs.eq(&rhs).into()),
            BinaryOp::NotEqual => Ok((!lhs.eq(&rhs)).into()),
            BinaryOp::GreatEqual => match (lhs, rhs) {
                (Value::Integer(a), Value::Integer(b)) => Ok((a >= b).into()),
                (Value::Float(a), Value::Float(b)) => Ok((a >= b).into()),
                (Value::Integer(a), Value::Float(b)) => Ok(((a as f64) >= b).into()),
                (Value::Float(a), Value::Integer(b)) => Ok((a >= (b as f64)).into()),
                (Value::Bytes(a), Value::Bytes(b)) => Ok((a >= b).into()),
                (lhs, rhs) => Err(ExpressionError::Binary {
                    err: BinaryError::GreatEqual(lhs.kind(), rhs.kind()),
                    span: self.lhs.span.merge(self.rhs.span),
                }),
            },
            BinaryOp::GreatThan => match (lhs, rhs) {
                (Value::Integer(a), Value::Integer(b)) => Ok((a > b).into()),
                (Value::Float(a), Value::Float(b)) => Ok((a > b).into()),
                (Value::Integer(a), Value::Float(b)) => Ok(((a as f64) > b).into()),
                (Value::Float(a), Value::Integer(b)) => Ok((a > (b as f64)).into()),
                (Value::Bytes(a), Value::Bytes(b)) => Ok((a > b).into()),
                (lhs, rhs) => Err(ExpressionError::Binary {
                    err: BinaryError::GreatThan(lhs.kind(), rhs.kind()),
                    span: self.lhs.span.merge(self.rhs.span),
                }),
            },
            BinaryOp::LessEqual => match (lhs, rhs) {
                (Value::Integer(a), Value::Integer(b)) => Ok((a <= b).into()),
                (Value::Float(a), Value::Float(b)) => Ok((a <= b).into()),
                (Value::Integer(a), Value::Float(b)) => Ok(((a as f64) <= b).into()),
                (Value::Float(a), Value::Integer(b)) => Ok((a <= (b as f64)).into()),
                (Value::Bytes(a), Value::Bytes(b)) => Ok((a <= b).into()),
                (lhs, rhs) => Err(ExpressionError::Binary {
                    err: BinaryError::LessEqual(lhs.kind(), rhs.kind()),
                    span: self.lhs.span.merge(self.rhs.span),
                }),
            },
            BinaryOp::LessThan => match (lhs, rhs) {
                (Value::Integer(a), Value::Integer(b)) => Ok((a < b).into()),
                (Value::Float(a), Value::Float(b)) => Ok((a < b).into()),
                (Value::Integer(a), Value::Float(b)) => Ok(((a as f64) < b).into()),
                (Value::Float(a), Value::Integer(b)) => Ok((a < (b as f64)).into()),
                (Value::Bytes(a), Value::Bytes(b)) => Ok((a < b).into()),
                (lhs, rhs) => Err(ExpressionError::Binary {
                    err: BinaryError::LessThan(lhs.kind(), rhs.kind()),
                    span: self.lhs.span.merge(self.rhs.span),
                }),
            },

            _ => unreachable!("operate should be handled at the start"),
        }
    }

    fn type_def(&self, state: &TypeState) -> TypeDef {
        let lhs_def = self.lhs.type_def(state);

        match self.op {
            BinaryOp::Or => {
                if lhs_def.is_null() || self.lhs.is_bool(false) {
                    // lhs is always "false"
                    self.rhs.type_def(state)
                } else if !(lhs_def.kind.contains(Kind::NULL)
                    || lhs_def.kind.contains(Kind::BOOLEAN))
                    || self.lhs.is_bool(true)
                {
                    // lhs is always "true"
                    lhs_def
                } else {
                    // not sure if lhs is true/false
                    TypeDef {
                        fallible: false, // todo: fix
                        kind: Kind::BOOLEAN,
                    }
                }
            }
            BinaryOp::And => {
                if lhs_def.is_null() || self.lhs.is_bool(false) {
                    // lhs is always "false"
                    TypeDef::boolean()
                } else if self.lhs.is_bool(true) {
                    // lhs is always "true"
                    // keep the fallibility of RHS, but change it to a boolean
                    self.rhs.type_def(state)
                } else {
                    // unknown if lhs is true or false
                    // lhs_def.
                    todo!()
                }
            }

            BinaryOp::Equal | BinaryOp::NotEqual => TypeDef {
                fallible: lhs_def.fallible | self.rhs.type_def(state).fallible,
                kind: Kind::BOOLEAN,
            },

            BinaryOp::GreatThan
            | BinaryOp::GreatEqual
            | BinaryOp::LessThan
            | BinaryOp::LessEqual => {
                if lhs_def.is_bytes() && self.rhs.type_def(state).is_bytes() {
                    TypeDef {
                        fallible: lhs_def.fallible | self.rhs.type_def(state).fallible,
                        kind: Kind::BOOLEAN,
                    }
                } else {
                    TypeDef {
                        fallible: !lhs_def.is_numeric() | !self.rhs.type_def(state).is_numeric(),
                        kind: Kind::BOOLEAN,
                    }
                }
            }

            BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply => {
                // none of these operations short-circuit, so the type of RHS can be applied.
                let rhs_def = self.rhs.type_def(state);

                match self.op {
                    // "foo" + xxxx
                    // xxxx + "bar"
                    BinaryOp::Add if lhs_def.is_bytes() || rhs_def.is_bytes() => TypeDef::bytes(),

                    // ... + 1.0
                    // ... - 1.0
                    // ... * 1.0
                    BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply
                        if lhs_def.is_numeric() || rhs_def.is_numeric() =>
                    {
                        if lhs_def.is_integer() && rhs_def.is_integer() {
                            TypeDef::integer()
                        } else {
                            TypeDef::float()
                        }
                    }

                    // "foo" * 2 == "foofoo"
                    BinaryOp::Multiply if lhs_def.is_bytes() && rhs_def.is_integer() => TypeDef {
                        fallible: lhs_def.fallible | rhs_def.fallible,
                        kind: Kind::BYTES,
                    },

                    // 2 * "bar" = "barbar"
                    BinaryOp::Multiply if lhs_def.is_integer() && rhs_def.is_bytes() => TypeDef {
                        fallible: lhs_def.fallible | rhs_def.fallible,
                        kind: Kind::BYTES,
                    },

                    // foo + bar
                    // foo * bar
                    BinaryOp::Add | BinaryOp::Multiply => TypeDef {
                        fallible: lhs_def.fallible | rhs_def.fallible,
                        kind: Kind::NUMERIC,
                    },

                    // foo - bar
                    BinaryOp::Subtract => TypeDef {
                        fallible: lhs_def.fallible | rhs_def.fallible,
                        kind: Kind::NUMERIC,
                    },

                    _ => unreachable!("Add, Subtract or Multiply operation not handled"),
                }
            }
            BinaryOp::Divide => {
                // division is infallible if the rhs is a literal normal
                // float or integer.

                let fallible = match &self.rhs.node {
                    Expr::Float(f) => *f != 0.0,
                    Expr::Integer(i) => *i != 0,
                    _ => true,
                };

                TypeDef {
                    fallible,
                    kind: Kind::NUMERIC,
                }
            }
            BinaryOp::Exponent => {
                // todo
                TypeDef {
                    fallible: false,
                    kind: Kind::NUMERIC,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::statement::Statement;
    use crate::compiler::Compiler;

    fn assert_optimize(input: &str, want: impl Into<Expr>) {
        let program = Compiler::compile(input).unwrap();
        let statements = program.statements.inner();

        match &statements[0] {
            Statement::Expression(got) => {
                assert_eq!(got.to_string(), want.into().to_string());
            }
            _ => panic!(""),
        }
    }

    #[test]
    fn optimize() {
        assert_optimize("1 + 1", 2);
        assert_optimize("1 + 2 + 3", 6);
        assert_optimize("(1 + 2) / 3", 1.0);
        assert_optimize("1 / 2", 0.5)
    }
}
