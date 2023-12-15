use value::{OwnedTargetPath, OwnedValuePath, Value};

use super::expression::Expression;
use super::parser::Expr;
use super::{ExpressionError, Kind};
use crate::compiler::type_def::TypeDef;
use crate::compiler::{Span, Spanned};
use crate::Context;

#[derive(Clone)]
pub enum AssignmentTarget {
    Internal(String, Option<OwnedValuePath>),
    External(OwnedTargetPath),
}

impl AssignmentTarget {
    fn assign(&self, cx: &mut Context, value: Value) -> Result<(), ExpressionError> {
        match self {
            AssignmentTarget::Internal(name, path) => {
                if let Some(target) = cx.variables.get_mut(name) {
                    match path {
                        // foo.bar = "abc"
                        Some(path) => {
                            target.insert(path, value);
                        }
                        // foo = "abc"
                        None => *target = value,
                    }
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

    fn type_def(&self) -> TypeDef {
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
