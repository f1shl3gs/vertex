use std::net::Ipv4Addr;

use value::Value;

use crate::compiler::expression::Expression;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::Expr;
use crate::compiler::{ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::{Context, SyntaxError};

pub struct IsIpv4;

impl Function for IsIpv4 {
    fn identifier(&self) -> &'static str {
        "is_ipv4"
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
            function: Box::new(IsIpv4Func { value }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct IsIpv4Func {
    value: Spanned<Expr>,
}

impl Expression for IsIpv4Func {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        match self.value.resolve(cx)? {
            Value::Bytes(b) => {
                let text = String::from_utf8_lossy(&b);
                let is = text.parse::<Ipv4Addr>().is_ok();
                Ok(is.into())
            }
            value => Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: value.kind(),
                span: self.value.span,
            }),
        }
    }

    fn type_def(&self) -> TypeDef {
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
    use crate::compiler::Span;
    use value::parse_target_path;

    #[test]
    fn ipv4() {
        compile_and_run(
            vec!["1.1.1.1".into()],
            IsIpv4,
            TypeDef::boolean(),
            Ok(true.into()),
        )
    }

    #[test]
    fn ipv6() {
        compile_and_run(
            vec!["ce:93:20:38:4a:9e".into()],
            IsIpv4,
            TypeDef::boolean(),
            Ok(false.into()),
        )
    }

    #[test]
    fn integer() {
        compile_and_run(
            vec![parse_target_path(".int").unwrap().into()],
            IsIpv4,
            TypeDef::boolean(),
            Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: Kind::INTEGER,
                span: Span::empty(),
            }),
        )
    }
}
