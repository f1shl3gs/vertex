use value::Value;

use crate::compiler::expression::Expression;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::Expr;
use crate::compiler::{ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::context::Context;
use crate::SyntaxError;

pub struct EndsWith;

impl Function for EndsWith {
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

    fn compile(
        &self,
        cx: FunctionCompileContext,
        mut arguments: ArgumentList,
    ) -> Result<FunctionCall, SyntaxError> {
        let value = arguments.get();
        let substring = arguments.get();
        let case_sensitive = arguments.get_opt();

        Ok(FunctionCall {
            function: Box::new(EndsWithFunc {
                value,
                substring,
                case_sensitive,
            }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct EndsWithFunc {
    value: Spanned<Expr>,
    substring: Spanned<Expr>,
    case_sensitive: Option<Spanned<Expr>>,
}

impl Expression for EndsWithFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = self.value.resolve(cx)?;
        let value = match &value {
            Value::Bytes(b) => String::from_utf8_lossy(b),
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::BYTES,
                    got: value.kind(),
                    span: self.value.span,
                })
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
                })
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
                    })
                }
            },
            None => false,
        };

        let value = if case_sensitive {
            value.to_string().ends_with(&substring.to_string())
        } else {
            value.ends_with(substring.as_ref())
        };

        Ok(Value::Boolean(value))
    }

    fn type_def(&self) -> TypeDef {
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
    fn yes() {
        compile_and_run(
            vec!["foobar".into(), "bar".into()],
            EndsWith,
            TypeDef::boolean(),
            Ok(true.into()),
        )
    }

    #[test]
    fn no() {
        compile_and_run(
            vec!["foobar".into(), "ba".into()],
            EndsWith,
            TypeDef::boolean(),
            Ok(false.into()),
        );

        compile_and_run(
            vec!["foobar".into(), "BAR".into()],
            EndsWith,
            TypeDef::boolean(),
            Ok(false.into()),
        );
    }

    #[test]
    fn case_sensitive() {
        compile_and_run(
            vec!["foobar".into(), "BAR".into(), true.into()],
            EndsWith,
            TypeDef::boolean(),
            Ok(false.into()),
        );

        compile_and_run(
            vec!["foobar".into(), "BAR".into(), false.into()],
            EndsWith,
            TypeDef::boolean(),
            Ok(false.into()),
        )
    }
}
