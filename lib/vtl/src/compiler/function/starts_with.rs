use value::Value;

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef};
use crate::context::Context;

pub struct StartsWith;

impl Function for StartsWith {
    fn identifier(&self) -> &'static str {
        "starts_with"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "value",
                kind: Kind::BYTES,
                required: true,
            },
            Parameter {
                name: "substring",
                kind: Kind::BYTES,
                required: true,
            },
            Parameter {
                name: "case_sensitive",
                kind: Kind::BOOLEAN,
                required: false,
            },
        ]
    }

    fn compile(&self, mut arguments: ArgumentList) -> Result<FunctionCall, SyntaxError> {
        let value = arguments.get();
        let substring = arguments.get();
        let case_sensitive = arguments.get_opt();

        Ok(FunctionCall {
            function: Box::new(StartsWithFunc {
                value,
                substring,
                case_sensitive,
            }),
        })
    }
}

#[derive(Clone)]
struct StartsWithFunc {
    value: Spanned<Expr>,
    substring: Spanned<Expr>,
    case_sensitive: Option<Spanned<Expr>>,
}

impl Expression for StartsWithFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = self.value.resolve(cx)?;
        let value = match &value {
            Value::Bytes(b) => String::from_utf8_lossy(b),
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::BYTES,
                    got: value.kind(),
                    span: self.value.span,
                });
            }
        };

        let substring = self.substring.resolve(cx)?;
        let substring = match &substring {
            Value::Bytes(b) => String::from_utf8_lossy(b),
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::BYTES,
                    got: value.kind(),
                    span: self.value.span,
                });
            }
        };

        let case_sensitive = match &self.case_sensitive {
            Some(expr) => match expr.resolve(cx)? {
                Value::Boolean(b) => b,
                value => {
                    return Err(ExpressionError::UnexpectedType {
                        want: Kind::BOOLEAN,
                        got: value.kind(),
                        span: expr.span,
                    });
                }
            },
            None => false,
        };

        let value = if case_sensitive {
            value.to_string().starts_with(&substring.to_string())
        } else {
            value.starts_with(substring.as_ref())
        };

        Ok(Value::Boolean(value))
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::BOOLEAN,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn find() {
        compile_and_run(
            vec!["foobar".into(), "foo".into()],
            StartsWith,
            TypeDef::boolean(),
            Ok(Value::Boolean(true)),
        )
    }

    #[test]
    fn not() {
        compile_and_run(
            vec!["foobar".into(), "oo".into()],
            StartsWith,
            TypeDef::boolean(),
            Ok(Value::Boolean(false)),
        )
    }

    #[test]
    fn case_sensitive() {
        compile_and_run(
            vec!["foobar".into(), "FOO".into()],
            StartsWith,
            TypeDef::boolean(),
            Ok(Value::Boolean(false)),
        )
    }

    #[test]
    fn case_sensitive_no() {
        compile_and_run(
            vec!["foobar".into(), "FOO".into(), false.into()],
            StartsWith,
            TypeDef::boolean(),
            Ok(Value::Boolean(false)),
        )
    }

    #[test]
    fn case_sensitive_yes() {
        compile_and_run(
            vec!["foobar".into(), "FOO".into(), true.into()],
            StartsWith,
            TypeDef::boolean(),
            Ok(Value::Boolean(false)),
        )
    }
}
