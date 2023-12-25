use value::path::{PathPrefix, TargetPath};
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

    pub fn apply_state(&self, state: &mut TypeState, kind: Kind) {
        match self {
            AssignmentTarget::Internal(index, path) => {
                let variable = state.variable_mut(*index);
                match path {
                    Some(path) => variable.apply_with_path(kind, path),
                    None => variable.apply(kind),
                }
            }
            AssignmentTarget::External(path) => {
                let value_path = path.value_path();
                match path.prefix() {
                    PathPrefix::Event => state.target.apply_with_path(kind, value_path),
                    PathPrefix::Metadata => state.metadata.apply_with_path(kind, value_path),
                }
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_state() {
        let mut state = TypeState::default();
        state.push("foo"); // a dummy variable, to make suer index 0 did exists
        let target = AssignmentTarget::Internal(0, None);
        target.apply_state(&mut state, Kind::BYTES);

        assert_eq!(state.variable(0).kind(&OwnedValuePath::root()), Kind::BYTES);
    }
}
