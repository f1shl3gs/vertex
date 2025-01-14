use value::{Kind, Value};

use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Spanned, TypeDef};
use crate::context::Context;
use crate::SyntaxError;

pub struct SnakeCase;

impl Function for SnakeCase {
    fn identifier(&self) -> &'static str {
        "snakecase"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[Parameter {
            name: "value",
            kind: Kind::BYTES,
            required: true,
        }]
    }

    fn compile(
        &self,
        cx: FunctionCompileContext,
        mut arguments: ArgumentList,
    ) -> Result<FunctionCall, SyntaxError> {
        let value = arguments.get();

        Ok(FunctionCall {
            function: Box::new(SnakeCaseFunc { value }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct SnakeCaseFunc {
    value: Spanned<Expr>,
}

impl Expression for SnakeCaseFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = self.value.resolve(cx)?;
        let Value::Bytes(value) = value else {
            return Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: value.kind(),
                span: self.value.span,
            });
        };

        let value = String::from_utf8_lossy(&value);
        let mut snake = String::new();
        for (i, ch) in value.char_indices() {
            if i > 0 && ch.is_uppercase() {
                snake.push('_');
            }
            snake.push(ch.to_ascii_lowercase());
        }

        Ok(snake.into())
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef {
            fallible: true,
            kind: Kind::BYTES,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn simple() {
        compile_and_run(
            vec!["camelCase".into()],
            SnakeCase,
            TypeDef::bytes().fallible(),
            Ok(Value::Bytes("camel_case".into())),
        )
    }

    #[test]
    fn no_case() {
        compile_and_run(
            vec!["camel_case".into()],
            SnakeCase,
            TypeDef::bytes().fallible(),
            Ok(Value::Bytes("camel_case".into())),
        )
    }
}
