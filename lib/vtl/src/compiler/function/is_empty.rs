use value::Value;

use crate::compiler::expression::Expression;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::Expr;
use crate::compiler::{ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::context::Context;
use crate::SyntaxError;

pub struct IsEmpty;

impl Function for IsEmpty {
    fn identifier(&self) -> &'static str {
        "is_empty"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[Parameter {
            name: "value",
            kind: Kind::ARRAY_BYTES_OBJECT,
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
            function: Box::new(IsEmptyFunc { value }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct IsEmptyFunc {
    value: Spanned<Expr>,
}

impl Expression for IsEmptyFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let is_empty = match self.value.resolve(cx)? {
            Value::Array(array) => array.is_empty(),
            Value::Object(object) => object.is_empty(),
            Value::Bytes(b) => b.is_empty(),
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::ARRAY | Kind::OBJECT | Kind::BYTES,
                    got: value.kind(),
                    span: self.value.span,
                })
            }
        };

        Ok(is_empty.into())
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
    use std::collections::BTreeMap;

    #[test]
    fn array() {
        compile_and_run(
            vec![vec![Expr::from(1)].into()],
            IsEmpty,
            TypeDef::boolean(),
            Ok(false.into()),
        )
    }

    #[test]
    fn empty_array() {
        compile_and_run(
            vec![vec![].into()],
            IsEmpty,
            TypeDef::boolean(),
            Ok(true.into()),
        )
    }

    #[test]
    fn map() {
        let mut argument = BTreeMap::new();
        argument.insert("foo".into(), 1.into());

        compile_and_run(
            vec![argument.into()],
            IsEmpty,
            TypeDef::boolean(),
            Ok(false.into()),
        )
    }

    #[test]
    fn empty_map() {
        let argument = BTreeMap::new();

        compile_and_run(
            vec![argument.into()],
            IsEmpty,
            TypeDef::boolean(),
            Ok(true.into()),
        )
    }

    #[test]
    fn bytes() {
        compile_and_run(
            vec!["foo".into()],
            IsEmpty,
            TypeDef::boolean(),
            Ok(false.into()),
        )
    }

    #[test]
    fn empty_bytes() {
        compile_and_run(
            vec!["".into()],
            IsEmpty,
            TypeDef::boolean(),
            Ok(true.into()),
        )
    }
}
