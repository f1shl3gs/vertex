use value::Value;

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef};
use crate::context::Context;

pub struct Push;

impl Function for Push {
    fn identifier(&self) -> &'static str {
        "push"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "array",
                kind: Kind::ARRAY,
                required: true,
            },
            Parameter {
                name: "item",
                kind: Kind::ANY,
                required: true,
            },
        ]
    }

    fn compile(
        &self,
        cx: FunctionCompileContext,
        mut arguments: ArgumentList,
    ) -> Result<FunctionCall, SyntaxError> {
        let array = arguments.get();
        let item = arguments.get();

        Ok(FunctionCall {
            function: Box::new(PushFunc { array, item }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct PushFunc {
    array: Spanned<Expr>,
    item: Spanned<Expr>,
}

impl Expression for PushFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let mut array = match self.array.resolve(cx)? {
            Value::Array(array) => array,
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::ARRAY,
                    got: value.kind(),
                    span: self.array.span,
                });
            }
        };

        array.push(self.item.resolve(cx)?);

        Ok(array.into())
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::ARRAY,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn push() {
        let array: Vec<Expr> = vec![1.into()];
        let item = 2.into();
        compile_and_run(
            vec![array.into(), item],
            Push,
            TypeDef::array(),
            Ok(vec![Value::from(1), Value::from(2)].into()),
        )
    }
}
