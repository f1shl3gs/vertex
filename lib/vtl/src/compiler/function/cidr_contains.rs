use std::net::IpAddr;
use std::str::FromStr;

use cidr_utils::cidr::IpCidr;
use value::{Kind, Value};

use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Spanned, TypeDef};
use crate::context::Context;
use crate::SyntaxError;

pub struct CidrContains;

impl Function for CidrContains {
    fn identifier(&self) -> &'static str {
        "cidr_contains"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "cidr",
                kind: Kind::BYTES,
                required: true,
            },
            Parameter {
                name: "value",
                kind: Kind::BYTES,
                required: true,
            },
        ]
    }

    fn compile(
        &self,
        cx: FunctionCompileContext,
        mut arguments: ArgumentList,
    ) -> Result<FunctionCall, SyntaxError> {
        let cidr = arguments.get();
        let value = arguments.get();

        Ok(FunctionCall {
            function: Box::new(CidrContainsFunc { cidr, value }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct CidrContainsFunc {
    cidr: Spanned<Expr>,
    value: Spanned<Expr>,
}

impl Expression for CidrContainsFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let cidr = self.cidr.resolve(cx)?;
        let Value::Bytes(value) = cidr else {
            return Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: cidr.kind(),
                span: self.cidr.span,
            });
        };

        let cidr = IpCidr::from_str(String::from_utf8_lossy(&value).as_ref()).map_err(|err| {
            ExpressionError::UnexpectedValue {
                msg: format!("invalid cidr, {}", err),
                span: self.cidr.span,
            }
        })?;

        let value = self.value.resolve(cx)?;
        let Value::Bytes(value) = value else {
            return Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: value.kind(),
                span: self.value.span,
            });
        };
        let ip = String::from_utf8_lossy(&value);
        let ip = IpAddr::from_str(ip.as_ref()).map_err(|err| ExpressionError::UnexpectedValue {
            msg: format!("invalid ip address, {}", err),
            span: self.value.span,
        })?;

        Ok(cidr.contains(&ip).into())
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
    use crate::compiler::function::compile_and_run;

    #[test]
    fn ipv4_yes() {
        compile_and_run(
            vec!["192.168.0.0/16".into(), "192.168.10.32".into()],
            CidrContains,
            TypeDef::boolean(),
            Ok(Value::Boolean(true)),
        )
    }

    #[test]
    fn ipv4_no() {
        compile_and_run(
            vec!["192.168.0.0/24".into(), "192.168.10.32".into()],
            CidrContains,
            TypeDef::boolean(),
            Ok(Value::Boolean(false)),
        )
    }

    #[test]
    fn ipv6_yes() {
        compile_and_run(
            vec![
                "2001:4f8:3:ba::/64".into(),
                "2001:4f8:3:ba:2e0:81ff:fe22:d1f1".into(),
            ],
            CidrContains,
            TypeDef::boolean(),
            Ok(Value::Boolean(true)),
        )
    }

    #[test]
    fn ipv6_no() {
        compile_and_run(
            vec![
                "2001:4f8:4:ba::/64".into(),
                "2001:4f8:3:ba:2e0:81ff:fe22:d1f1".into(),
            ],
            CidrContains,
            TypeDef::boolean(),
            Ok(Value::Boolean(false)),
        )
    }
}
