use std::ops::Deref;

use value::Value;

use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::SyntaxError;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::context::Context;

pub struct Contains;

impl Function for Contains {
    fn identifier(&self) -> &'static str {
        "contains"
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
            function: Box::new(ContainsFunc {
                value,
                substring,
                case_sensitive,
            }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct ContainsFunc {
    value: Spanned<Expr>,
    substring: Spanned<Expr>,
    case_sensitive: Option<Spanned<Expr>>,
}

impl Expression for ContainsFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = self.value.resolve(cx)?;
        let value = if let Value::Bytes(b) = &value {
            String::from_utf8_lossy(b)
        } else {
            return Err(ExpressionError::UnexpectedType {
                got: value.kind(),
                want: Kind::BYTES,
                span: self.value.span,
            });
        };

        let substring = self.substring.resolve(cx)?;
        let substring = if let Value::Bytes(b) = &substring {
            String::from_utf8_lossy(b)
        } else {
            return Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: substring.kind(),
                span: self.substring.span,
            });
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
            // default value
            None => false,
        };

        let contains = if case_sensitive {
            value.contains(substring.deref())
        } else {
            value
                .to_lowercase()
                .contains(substring.to_lowercase().as_str())
        };

        Ok(contains.into())
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
    fn no() {
        compile_and_run(
            vec!["foo".into(), "bar".into()],
            Contains,
            TypeDef::boolean(),
            Ok(false.into()),
        )
    }

    #[test]
    fn yes() {
        compile_and_run(
            vec!["foobar".into(), "foo".into()],
            Contains,
            TypeDef::boolean(),
            Ok(true.into()),
        )
    }

    #[test]
    fn entirely() {
        compile_and_run(
            vec!["foobar".into(), "foobar".into()],
            Contains,
            TypeDef::boolean(),
            Ok(true.into()),
        )
    }

    #[test]
    fn middle() {
        compile_and_run(
            vec!["foobar".into(), "oob".into()],
            Contains,
            TypeDef::boolean(),
            Ok(true.into()),
        )
    }

    #[test]
    fn start() {
        compile_and_run(
            vec!["foobar".into(), "foo".into()],
            Contains,
            TypeDef::boolean(),
            Ok(true.into()),
        )
    }

    #[test]
    fn end() {
        compile_and_run(
            vec!["foobar".into(), "bar".into()],
            Contains,
            TypeDef::boolean(),
            Ok(true.into()),
        )
    }

    #[test]
    fn case_sensitive_yes() {
        compile_and_run(
            vec!["fooBAR".into(), "bar".into(), true.into()],
            Contains,
            TypeDef::boolean(),
            Ok(false.into()),
        )
    }

    #[test]
    fn case_sensitive_no_uppercase() {
        compile_and_run(
            vec!["foobar".into(), "BAR".into(), false.into()],
            Contains,
            TypeDef::boolean(),
            Ok(true.into()),
        )
    }
}
