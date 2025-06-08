use value::Value;

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef};
use crate::context::Context;

pub struct IsInteger;

impl Function for IsInteger {
    fn identifier(&self) -> &'static str {
        "is_integer"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[Parameter {
            name: "value",
            kind: Kind::ANY,
            required: true,
        }]
    }

    fn compile(&self, mut arguments: ArgumentList) -> Result<FunctionCall, SyntaxError> {
        let value = arguments.get();

        Ok(FunctionCall {
            function: Box::new(IsIntegerFunc { value }),
        })
    }
}

#[derive(Clone)]
struct IsIntegerFunc {
    value: Spanned<Expr>,
}

impl Expression for IsIntegerFunc {
    #[allow(clippy::match_like_matches_macro)]
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = match self.value.resolve(cx)? {
            Value::Integer(_i) => true,
            _ => false,
        };

        Ok(value.into())
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
    fn integer() {
        compile_and_run(
            vec![1.into()],
            IsInteger,
            TypeDef::boolean(),
            Ok(true.into()),
        )
    }

    #[test]
    fn not_integer() {
        compile_and_run(
            vec!["1".into()],
            IsInteger,
            TypeDef::boolean(),
            Ok(false.into()),
        )
    }
}
