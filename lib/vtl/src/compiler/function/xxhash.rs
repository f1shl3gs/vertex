use value::Value;

use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::Expr;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::{Context, SyntaxError};

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

    fn compile(
        &self,
        cx: FunctionCompileContext,
        mut arguments: ArgumentList,
    ) -> Result<FunctionCall, SyntaxError> {
        let value = arguments.get();

        Ok(FunctionCall {
            function: Box::new(XXHashFunc { value }),
            span: cx.span,
        })
    }
}

struct XXHashFunc {
    value: Spanned<Expr>,
}

impl Expression for XXHashFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let hash = match self.value.resolve(cx)? {
            Value::Bytes(b) => twox_hash::xxh3::hash64(&b),
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::BYTES,
                    got: value.kind(),
                    span: self.value.span,
                })
            }
        };

        Ok(hash.into())
    }

    fn type_def(&self) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::INTEGER,
        }
    }
}
