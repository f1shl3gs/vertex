use bytes::Bytes;
use value::Value;

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef};
use crate::context::Context;

pub struct TypeOf;

impl Function for TypeOf {
    fn identifier(&self) -> &'static str {
        "typeof"
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
            function: Box::new(TypeOfFunc { value }),
        })
    }
}

#[derive(Clone)]
struct TypeOfFunc {
    value: Spanned<Expr>,
}

impl Expression for TypeOfFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = match self.value.resolve(cx)? {
            Value::Bytes(_) => "string",
            Value::Float(_) => "float",
            Value::Integer(_) => "integer",
            Value::Boolean(_) => "boolean",
            Value::Timestamp(_) => "timestamp",
            Value::Object(_) => "object",
            Value::Array(_) => "array",
            Value::Null => "null",
        };

        Ok(Value::Bytes(Bytes::from_static(value.as_bytes())))
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::BYTES,
        }
    }
}
