use value::Value;

use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::context::Context;
use crate::SyntaxError;

pub struct GetEnv;

impl Function for GetEnv {
    fn identifier(&self) -> &'static str {
        "get_env"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[Parameter {
            name: "name",
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
            function: Box::new(GetEnvFunc { expr }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct GetEnvFunc {
    expr: Spanned<Expr>,
}

impl Expression for GetEnvFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let name = self.expr.resolve(cx)?;

        if let Value::Bytes(b) = name {
            let name = String::from_utf8_lossy(&b);
            let value = std::env::var(name.as_ref()).map_err(|err| ExpressionError::Error {
                message: err.to_string(),
                span: self.expr.span,
            })?;

            Ok(Value::Bytes(value.into()))
        } else {
            Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: name.kind(),
                span: self.expr.span,
            })
        }
    }

    fn type_def(&self) -> TypeDef {
        TypeDef {
            fallible: true,
            kind: Kind::BYTES,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::compiler::function::compile_and_run;
    use crate::compiler::Span;

    #[test]
    fn exists() {
        std::env::set_var("foo", "bar");

        let args = vec!["foo".into()];
        compile_and_run(args, GetEnv, TypeDef::bytes().fallible(), Ok("bar".into()))
    }

    #[test]
    fn not_exists() {
        let args = vec!["bar".into()];
        let want_err = std::env::var("bar").unwrap_err();

        compile_and_run(
            args,
            GetEnv,
            TypeDef::bytes().fallible(),
            Err(ExpressionError::Error {
                message: want_err.to_string(),
                span: Span::empty(),
            }),
        )
    }
}
