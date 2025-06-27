use value::{Kind, Value};

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Spanned, TypeDef};
use crate::context::Context;

pub struct Assert;

impl Function for Assert {
    fn identifier(&self) -> &'static str {
        "assert"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "condition",
                kind: Kind::BOOLEAN,
                required: true,
            },
            Parameter {
                name: "message",
                kind: Kind::BYTES,
                required: false,
            },
        ]
    }

    fn compile(&self, mut arguments: ArgumentList) -> Result<FunctionCall, SyntaxError> {
        let condition = arguments.get();
        let message = arguments.get_opt();

        Ok(FunctionCall {
            function: Box::new(AssertFunc { condition, message }),
        })
    }
}

#[derive(Clone)]
struct AssertFunc {
    condition: Spanned<Expr>,
    message: Option<Spanned<Expr>>,
}

impl Expression for AssertFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let condition = self.condition.resolve(cx)?;
        let Value::Boolean(b) = condition else {
            return Err(ExpressionError::UnexpectedType {
                want: Kind::BOOLEAN,
                got: condition.kind(),
                span: self.condition.span,
            });
        };

        if b {
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
                message: format!("assertion failed, {condition}"),
                span: self.condition.span,
            }),
        }
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef::null().fallible()
    }
}
