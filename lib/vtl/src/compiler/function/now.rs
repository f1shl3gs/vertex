use chrono::Utc;
use value::Value;

use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::SyntaxError;
use crate::compiler::{Expression, ExpressionError, Kind, TypeDef};
use crate::context::Context;

#[derive(Debug)]
pub struct Now;

impl Function for Now {
    fn identifier(&self) -> &'static str {
        "now"
    }

    fn compile(
        &self,
        cx: FunctionCompileContext,
        _arguments: ArgumentList,
    ) -> Result<FunctionCall, SyntaxError> {
        Ok(FunctionCall {
            function: Box::new(NowFunc),
            span: cx.span,
        })
    }
}

struct NowFunc;

impl Expression for NowFunc {
    fn resolve(&self, _cx: &mut Context) -> Result<Value, ExpressionError> {
        Ok(Value::Timestamp(Utc::now()))
    }

    fn type_def(&self) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::TIMESTAMP,
        }
    }
}
