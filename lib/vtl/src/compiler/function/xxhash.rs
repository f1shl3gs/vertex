use value::Value;

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef};
use crate::context::Context;

pub struct XXHash;

impl Function for XXHash {
    fn identifier(&self) -> &'static str {
        "xxhash"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[Parameter {
            name: "value",
            kind: Kind::BYTES,
            required: true,
        }]
    }

    fn compile(&self, mut arguments: ArgumentList) -> Result<FunctionCall, SyntaxError> {
        let value = arguments.get();

        Ok(FunctionCall {
            function: Box::new(XXHashFunc { value }),
        })
    }
}

#[derive(Clone)]
struct XXHashFunc {
    value: Spanned<Expr>,
}

impl Expression for XXHashFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let hash = match self.value.resolve(cx)? {
            Value::Bytes(b) => twox_hash::XxHash64::oneshot(0, &b),
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::BYTES,
                    got: value.kind(),
                    span: self.value.span,
                });
            }
        };

        Ok(hash.into())
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::INTEGER,
        }
    }
}
