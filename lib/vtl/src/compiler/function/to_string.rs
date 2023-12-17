use chrono::SecondsFormat;
use value::Value;

use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::Expr;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::context::Context;
use crate::SyntaxError;

pub struct ToString;

impl Function for ToString {
    fn identifier(&self) -> &'static str {
        "to_string"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[Parameter {
            name: "value",
            kind: Kind::ANY,
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
            function: Box::new(ToStringFunc { value }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct ToStringFunc {
    value: Spanned<Expr>,
}

impl Expression for ToStringFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        use std::string::ToString;

        let value = match self.value.resolve(cx)? {
            v @ Value::Bytes(_) => v,
            Value::Integer(i) => i.to_string().into(),
            Value::Float(f) => f.to_string().into(),
            Value::Boolean(b) => b.to_string().into(),
            Value::Timestamp(ts) => ts.to_rfc3339_opts(SecondsFormat::AutoSi, true).into(),
            Value::Null => "".into(),
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::BYTES
                        | Kind::INTEGER
                        | Kind::FLOAT
                        | Kind::BOOLEAN
                        | Kind::TIMESTAMP
                        | Kind::NULL,
                    got: value.kind(),
                    span: self.value.span,
                })
            }
        };

        Ok(value)
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
    use value::parse_target_path;

    #[test]
    fn bytes() {
        compile_and_run(
            vec!["foo".into()],
            ToString,
            TypeDef::bytes(),
            Ok("foo".into()),
        )
    }

    #[test]
    fn integer() {
        compile_and_run(vec![1.into()], ToString, TypeDef::bytes(), Ok("1".into()))
    }

    #[test]
    fn float() {
        compile_and_run(
            vec![1.2.into()],
            ToString,
            TypeDef::bytes(),
            Ok("1.2".into()),
        )
    }

    #[test]
    fn timestamp() {
        compile_and_run(
            vec![parse_target_path(".timestamp").unwrap().into()],
            ToString,
            TypeDef::bytes(),
            Ok("2021-01-01T00:00:00Z".into()),
        )
    }

    #[test]
    fn null() {
        compile_and_run(vec![Expr::Null], ToString, TypeDef::bytes(), Ok("".into()))
    }
}
