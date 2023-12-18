use value::Value;

use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef};
use crate::context::Context;
use crate::SyntaxError;

pub struct AssertEq;

impl Function for AssertEq {
    fn identifier(&self) -> &'static str {
        "assert_eq"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "left",
                kind: Kind::ANY,
                required: true,
            },
            Parameter {
                name: "right",
                kind: Kind::ANY,
                required: true,
            },
            Parameter {
                name: "message",
                kind: Kind::BYTES,
                required: false,
            },
        ]
    }

    fn compile(
        &self,
        cx: FunctionCompileContext,
        mut arguments: ArgumentList,
    ) -> Result<FunctionCall, SyntaxError> {
        let left = arguments.get();
        let right = arguments.get();
        let message = arguments.get_opt();

        Ok(FunctionCall {
            function: Box::new(AssertEqFunc {
                left,
                right,
                message,
            }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct AssertEqFunc {
    left: Spanned<Expr>,
    right: Spanned<Expr>,
    message: Option<Spanned<Expr>>,
}

impl Expression for AssertEqFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let left = self.left.resolve(cx)?;
        let right = self.right.resolve(cx)?;

        if left.eq(&right) {
            return Ok(true.into());
        }

        match &self.message {
            Some(expr) => {
                let message = expr.resolve(cx)?.to_string_lossy().into_owned();
                Err(ExpressionError::Error {
                    message,
                    span: expr.span,
                })
            }
            None => Err(ExpressionError::Error {
                message: format!("assertion failed, {} == {}", left, right),
                span: self.left.span.merge(self.right.span),
            }),
        }
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef {
            fallible: true,
            kind: Kind::BOOLEAN,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;
    use crate::compiler::Span;

    #[test]
    fn pass() {
        compile_and_run(
            vec!["foo".into(), "foo".into()],
            AssertEq,
            TypeDef::boolean().fallible(),
            Ok(true.into()),
        )
    }

    #[test]
    fn fail() {
        compile_and_run(
            vec!["foo".into(), "bar".into()],
            AssertEq,
            TypeDef::boolean().fallible(),
            Err(ExpressionError::Error {
                message: "assertion failed, \"foo\" == \"bar\"".to_string(),
                span: Span::empty(),
            }),
        )
    }

    #[test]
    fn fail_message() {
        compile_and_run(
            vec!["foo".into(), "bar".into(), "msg".into()],
            AssertEq,
            TypeDef::boolean().fallible(),
            Err(ExpressionError::Error {
                message: "msg".to_string(),
                span: Span::empty(),
            }),
        )
    }
}
