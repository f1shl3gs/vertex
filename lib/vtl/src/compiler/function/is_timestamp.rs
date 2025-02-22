use value::Value;

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef};
use crate::context::Context;

pub struct IsTimestamp;

impl Function for IsTimestamp {
    fn identifier(&self) -> &'static str {
        "is_timestamp"
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
            function: Box::new(IsTimestampFunc { value }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct IsTimestampFunc {
    value: Spanned<Expr>,
}

impl Expression for IsTimestampFunc {
    #[allow(clippy::match_like_matches_macro)]
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = match self.value.resolve(cx)? {
            Value::Timestamp(_ts) => true,
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
    use value::parse_target_path;

    #[test]
    fn timestamp() {
        compile_and_run(
            vec![parse_target_path(".timestamp").unwrap().into()],
            IsTimestamp,
            TypeDef::boolean(),
            Ok(true.into()),
        )
    }

    #[test]
    fn not_timestamp() {
        compile_and_run(
            vec!["foo".into()],
            IsTimestamp,
            TypeDef::boolean(),
            Ok(false.into()),
        )
    }
}
