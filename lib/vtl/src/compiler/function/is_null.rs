use value::{Kind, Value};

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Spanned, TypeDef};
use crate::context::Context;

pub struct IsNull;

impl Function for IsNull {
    fn identifier(&self) -> &'static str {
        "is_null"
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
            function: Box::new(IsNullFunc { value }),
        })
    }
}

#[derive(Clone)]
struct IsNullFunc {
    value: Spanned<Expr>,
}

impl Expression for IsNullFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = self.value.resolve(cx)?;

        match value {
            Value::Null => Ok(true.into()),
            _ => Ok(false.into()),
        }
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef::boolean()
    }
}
