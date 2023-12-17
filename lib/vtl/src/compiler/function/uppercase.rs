use value::Value;

use super::{Function, FunctionCompileContext};
use crate::compiler::function::{ArgumentList, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::{Expr, SyntaxError};
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::context::Context;

pub struct Uppercase;

impl Function for Uppercase {
    fn identifier(&self) -> &'static str {
        "uppercase"
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
        let expr = arguments.get();

        Ok(FunctionCall {
            function: Box::new(UppercaseFunc { expr }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct UppercaseFunc {
    expr: Spanned<Expr>,
}

impl Expression for UppercaseFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        match self.expr.resolve(cx)? {
            Value::Bytes(b) => {
                let s = String::from_utf8_lossy(&b).to_uppercase();
                Ok(Value::Bytes(s.into()))
            }
            value => Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: value.kind(),
                span: self.expr.span,
            }),
        }
    }

    fn type_def(&self) -> TypeDef {
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
    fn uppercase() {
        compile_and_run(
            vec!["foo".into()],
            Uppercase,
            TypeDef::bytes(),
            Ok("FOO".into()),
        )
    }
}
