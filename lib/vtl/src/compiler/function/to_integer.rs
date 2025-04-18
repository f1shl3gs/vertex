use value::Value;

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef};
use crate::context::Context;

pub struct ToInteger;

impl Function for ToInteger {
    fn identifier(&self) -> &'static str {
        "to_integer"
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
            function: Box::new(ToIntegerFunc { value }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct ToIntegerFunc {
    value: Spanned<Expr>,
}

impl Expression for ToIntegerFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = match self.value.resolve(cx)? {
            Value::Integer(i) => i,
            Value::Float(f) => f as i64,
            Value::Boolean(b) => i64::from(b),
            Value::Null => 0,
            Value::Timestamp(ts) => ts.timestamp(),
            Value::Bytes(b) => String::from_utf8_lossy(&b).parse::<i64>().map_err(|err| {
                ExpressionError::UnexpectedValue {
                    msg: err.to_string(),
                    span: self.value.span,
                }
            })?,
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::INTEGER | Kind::FLOAT | Kind::BOOLEAN | Kind::NULL | Kind::BYTES,
                    got: value.kind(),
                    span: self.value.span,
                });
            }
        };

        Ok(value.into())
    }

    fn type_def(&self, state: &TypeState) -> TypeDef {
        let def = self.value.type_def(state);
        let fallible = !matches!(
            def.kind,
            Kind::INTEGER | Kind::FLOAT | Kind::BOOLEAN | Kind::NULL
        );

        TypeDef {
            fallible,
            kind: Kind::INTEGER,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;
    use value::parse_target_path;

    #[test]
    fn integer() {
        compile_and_run(vec![1.into()], ToInteger, TypeDef::integer(), Ok(1.into()))
    }

    #[test]
    fn float() {
        compile_and_run(
            vec![1.2.into()],
            ToInteger,
            TypeDef::integer(),
            Ok((1.2 as i64).into()),
        )
    }

    #[test]
    fn boolean() {
        compile_and_run(
            vec![true.into()],
            ToInteger,
            TypeDef::integer(),
            Ok(1.into()),
        );

        compile_and_run(
            vec![false.into()],
            ToInteger,
            TypeDef::integer(),
            Ok(0.into()),
        )
    }

    #[test]
    fn null() {
        compile_and_run(
            vec![Expr::Null],
            ToInteger,
            TypeDef::integer(),
            Ok(0.into()),
        )
    }

    #[test]
    fn bytes() {
        compile_and_run(
            vec!["1".into()],
            ToInteger,
            TypeDef::integer().fallible(),
            Ok(1.into()),
        )
    }

    #[test]
    fn timestamp() {
        compile_and_run(
            vec![parse_target_path(".timestamp").unwrap().into()],
            ToInteger,
            // `.timestamp` is unknown at compile time, so it is fallible when
            // there is no such field
            TypeDef::integer().fallible(),
            Ok(1609459200.into()),
        )
    }
}
