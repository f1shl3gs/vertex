use value::Value;

use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::context::Context;
use crate::SyntaxError;

pub struct Length;

impl Function for Length {
    fn identifier(&self) -> &'static str {
        "length"
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
            function: Box::new(LengthFunc { value }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct LengthFunc {
    value: Spanned<Expr>,
}

impl Expression for LengthFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let length = match self.value.resolve(cx)? {
            Value::Array(array) => array.len(),
            Value::Object(object) => object.len(),
            Value::Bytes(b) => b.len(),
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::ARRAY_BYTES_OBJECT,
                    got: value.kind(),
                    span: self.value.span,
                })
            }
        };

        Ok(Value::Integer(length as i64))
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::INTEGER,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;
    use value::parse_target_path;

    #[test]
    fn string() {
        compile_and_run(vec!["foo".into()], Length, TypeDef::integer(), Ok(3.into()))
    }

    #[test]
    fn array() {
        compile_and_run(
            vec![parse_target_path(".array").unwrap().into()],
            Length,
            TypeDef::integer(),
            Ok(3.into()),
        )
    }

    #[test]
    fn object() {
        compile_and_run(
            vec![parse_target_path(".map").unwrap().into()],
            Length,
            TypeDef::integer(),
            Ok(1.into()),
        )
    }
}
