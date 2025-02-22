use value::Value;

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef};
use crate::context::Context;

pub struct ParseJson;

impl Function for ParseJson {
    fn identifier(&self) -> &'static str {
        "parse_json"
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
            function: Box::new(ParseJsonFunc { value }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct ParseJsonFunc {
    value: Spanned<Expr>,
}

impl Expression for ParseJsonFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        match self.value.resolve(cx)? {
            Value::Bytes(b) => serde_json::from_slice(&b).map_err(|err| ExpressionError::Error {
                message: err.to_string(),
                span: self.value.span,
            }),
            value => Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: value.kind(),
                span: self.value.span,
            }),
        }
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef {
            fallible: true,
            kind: Kind::OBJECT,
        }
    }
}

#[cfg(test)]
mod tests {
    use value::value;

    use super::*;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn parse_json() {
        compile_and_run(
            vec![r#"{"key": "value"}"#.into()],
            ParseJson,
            TypeDef::object().fallible(),
            Ok(value!({"key": "value"})),
        )
    }
}
