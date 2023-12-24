use value::{OwnedTargetPath, OwnedValuePath, Value};

use super::expr::Expr;
use super::state::TypeState;
use super::type_def::TypeDef;
use super::{Expression, Span, Spanned};
use super::{ExpressionError, Kind};
use crate::context::Context;

#[derive(Clone)]
pub enum AssignmentTarget {
    Internal(usize, Option<OwnedValuePath>),
    External(OwnedTargetPath),
}

impl AssignmentTarget {
    fn assign(&self, cx: &mut Context, value: Value) -> Result<(), ExpressionError> {
        match self {
            AssignmentTarget::Internal(index, path) => {
                match path {
                    // foo.bar = "abc"
                    Some(path) => {
                        cx.get_mut(*index).insert(path, value);
                    }
                    // foo = "abc"
                    None => cx.set(*index, value),
                }
            }

            // .bar = "foo"
            AssignmentTarget::External(path) => {
                cx.target
                    .insert(path, value)
                    .map_err(|err| ExpressionError::Error {
                        message: err.to_string(),
                        span: Span { start: 0, end: 0 },
                    })?;
            }
        }

        Ok(())
    }
}

#[derive(Clone)]
pub enum Assignment {
    Single {
        target: AssignmentTarget,
        expr: Spanned<Expr>,
    },
    Infallible {
        ok: AssignmentTarget,
        err: AssignmentTarget,
        expr: Spanned<Expr>,
    },
}

impl Expression for Assignment {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        match self {
            Assignment::Single { target, expr } => {
                let value = expr.resolve(cx)?;
                target.assign(cx, value)?;
                Ok(Value::Null)
            }
            Assignment::Infallible { ok, err, expr } => {
                match expr.resolve(cx) {
                    Ok(value) => {
                        ok.assign(cx, value)?;
                        err.assign(cx, Value::Null)?;
                    }
                    Err(expr_err) => {
                        ok.assign(cx, Value::Null)?;
                        err.assign(cx, expr_err.to_string().into())?;
                    }
                }

                Ok(Value::Null)
            }
        }
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        let fallible = match self {
            Assignment::Single { .. } => true,
            Assignment::Infallible { .. } => false,
        };

        TypeDef {
            fallible,
            kind: Kind::UNDEFINED,
        }
    }
}
