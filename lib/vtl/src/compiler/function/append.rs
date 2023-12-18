use value::Value;

use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::context::Context;
use crate::SyntaxError;

pub struct Append;

impl Function for Append {
    fn identifier(&self) -> &'static str {
        "append"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "value",
                kind: Kind::ARRAY,
                required: true,
            },
            Parameter {
                name: "other",
                kind: Kind::ARRAY,
                required: true,
            },
        ]
    }

    fn compile(
        &self,
        cx: FunctionCompileContext,
        mut arguments: ArgumentList,
    ) -> Result<FunctionCall, SyntaxError> {
        let value = arguments.get();
        let others = arguments.get();

        Ok(FunctionCall {
            function: Box::new(AppendFunc { value, others }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct AppendFunc {
    value: Spanned<Expr>,
    others: Spanned<Expr>,
}

impl Expression for AppendFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let mut array = match self.value.resolve(cx)? {
            Value::Array(array) => array,
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::ARRAY,
                    got: value.kind(),
                    span: self.value.span,
                })
            }
        };

        match self.others.resolve(cx)? {
            Value::Array(others) => array.extend(others),
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::ARRAY,
                    got: value.kind(),
                    span: self.others.span,
                })
            }
        }

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
    fn append_to_empty() {
        let value: Vec<Expr> = vec![];
        let others: Vec<Expr> = vec![1.into()];
        compile_and_run(
            vec![value.into(), others.into()],
            Append,
            TypeDef::array(),
            Ok(vec![Value::from(1)].into()),
        )
    }

    #[test]
    fn append_empty() {
        let value: Vec<Expr> = vec![1.into()];
        let others: Vec<Expr> = vec![];
        compile_and_run(
            vec![value.into(), others.into()],
            Append,
            TypeDef::array(),
            Ok(vec![Value::from(1)].into()),
        )
    }

    #[test]
    fn append() {
        let value: Vec<Expr> = vec![1.into()];
        let others: Vec<Expr> = vec![2.into()];
        compile_and_run(
            vec![value.into(), others.into()],
            Append,
            TypeDef::array(),
            Ok(vec![Value::from(1), Value::from(2)].into()),
        )
    }
}
