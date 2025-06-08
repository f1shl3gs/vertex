use std::net::Ipv6Addr;

use value::Value;

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef};
use crate::context::Context;

pub struct IsIpv6;

impl Function for IsIpv6 {
    fn identifier(&self) -> &'static str {
        "is_ipv6"
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
            function: Box::new(IsIpv6Func { value }),
        })
    }
}

#[derive(Clone)]
struct IsIpv6Func {
    value: Spanned<Expr>,
}

impl Expression for IsIpv6Func {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        match self.value.resolve(cx)? {
            Value::Bytes(b) => {
                let text = String::from_utf8_lossy(&b);
                let is = text.parse::<Ipv6Addr>().is_ok();
                Ok(is.into())
            }
            value => Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: value.kind(),
                span: self.value.span,
            }),
        }
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::BOOLEAN,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::Span;
    use crate::compiler::function::compile_and_run;
    use value::parse_target_path;

    #[test]
    fn ipv4() {
        compile_and_run(
            vec!["1.1.1.1".into()],
            IsIpv6,
            TypeDef::boolean(),
            Ok(false.into()),
        )
    }

    #[test]
    fn ipv6() {
        compile_and_run(
            vec!["2001:0db8:85a3:0000:0000:8a2e:0370:7334".into()],
            IsIpv6,
            TypeDef::boolean(),
            Ok(true.into()),
        )
    }

    #[test]
    fn integer() {
        compile_and_run(
            vec![parse_target_path(".int").unwrap().into()],
            IsIpv6,
            TypeDef::boolean(),
            Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: Kind::INTEGER,
                span: Span { start: 0, end: 0 },
            }),
        )
    }
}
