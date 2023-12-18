use value::Value;

use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef};
use crate::context::Context;
use crate::SyntaxError;

pub struct IsString;

impl Function for IsString {
    fn identifier(&self) -> &'static str {
        "is_string"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[Parameter {
            name: "value",
            kind: Kind::ANY,
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
            function: Box::new(IsStringFunc { value }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct IsStringFunc {
    value: Spanned<Expr>,
}

impl Expression for IsStringFunc {
    #[allow(clippy::match_like_matches_macro)]
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = match self.value.resolve(cx)? {
            Value::Bytes(_b) => true,
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
    fn string() {
        compile_and_run(
            vec!["foo".into()],
            IsString,
            TypeDef::boolean(),
            Ok(Value::Boolean(true)),
        )
    }

    #[test]
    fn not_string() {
        compile_and_run(
            vec![1.into()],
            IsString,
            TypeDef::boolean(),
            Ok(Value::Boolean(false)),
        )
    }
}
