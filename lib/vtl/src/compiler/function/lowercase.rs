use value::Value;

use super::{Function, FunctionCompileContext};
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::SyntaxError;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::context::Context;

pub struct Lowercase;

impl Function for Lowercase {
    fn identifier(&self) -> &'static str {
        "lowercase"
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
        // length already checked
        let expr = arguments.get();

        Ok(FunctionCall {
            function: Box::new(LowercaseFunc { expr }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct LowercaseFunc {
    expr: Spanned<Expr>,
}

impl Expression for LowercaseFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        match self.expr.resolve(cx)? {
            Value::Bytes(b) => {
                let s = String::from_utf8_lossy(&b).to_lowercase();
                Ok(Value::Bytes(s.into()))
            }
            value => Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: value.kind(),
                span: self.expr.span,
            }),
        }
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::BYTES,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn lowercase() {
        compile_and_run(
            vec!["FOO".into()],
            Lowercase,
            TypeDef::bytes(),
            Ok("foo".into()),
        )
    }
}
