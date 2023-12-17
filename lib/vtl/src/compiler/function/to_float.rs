use value::Value;

use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::context::Context;
use crate::SyntaxError;

pub struct ToFloat;

impl Function for ToFloat {
    fn identifier(&self) -> &'static str {
        "to_float"
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
            function: Box::new(ToFloatFunc { value }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct ToFloatFunc {
    value: Spanned<Expr>,
}

impl Expression for ToFloatFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = match self.value.resolve(cx)? {
            Value::Float(f) => f,
            Value::Integer(i) => i as f64,
            Value::Boolean(b) => f64::from(b),
            Value::Null => 0.0,
            Value::Timestamp(ts) => ts.timestamp_nanos_opt().unwrap() as f64 / 1_000_000_000_f64,
            Value::Bytes(b) => String::from_utf8_lossy(&b).parse::<f64>().map_err(|err| {
                ExpressionError::UnexpectedValue {
                    msg: err.to_string(),
                    span: self.value.span,
                }
            })?,
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::FLOAT
                        | Kind::INTEGER
                        | Kind::BOOLEAN
                        | Kind::NULL
                        | Kind::TIMESTAMP
                        | Kind::BYTES,
                    got: value.kind(),
                    span: self.value.span,
                })
            }
        };

        Ok(Value::Float(value))
    }

    fn type_def(&self) -> TypeDef {
        let kind = self.value.type_def().kind;
        let fallible =
            kind.contains(Kind::BYTES) || kind.contains(Kind::ARRAY) || kind.contains(Kind::OBJECT);

        TypeDef {
            fallible,
            kind: Kind::FLOAT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;
    use value::parse_target_path;

    #[test]
    fn float() {
        compile_and_run(vec![1.2.into()], ToFloat, TypeDef::float(), Ok(1.2.into()))
    }

    #[test]
    fn integer() {
        compile_and_run(vec![1.into()], ToFloat, TypeDef::float(), Ok(1.0.into()))
    }

    #[test]
    fn boolean() {
        compile_and_run(vec![true.into()], ToFloat, TypeDef::float(), Ok(1.0.into()));

        compile_and_run(
            vec![false.into()],
            ToFloat,
            TypeDef::float(),
            Ok(0.0.into()),
        )
    }

    #[test]
    fn null() {
        compile_and_run(vec![Expr::Null], ToFloat, TypeDef::float(), Ok(0.0.into()))
    }

    #[test]
    fn timestamp() {
        compile_and_run(
            vec![parse_target_path(".timestamp").unwrap().into()],
            ToFloat,
            TypeDef::float().fallible(), // OwnedTargetPath return Kind::ANY
            Ok(1609459200.0.into()),
        )
    }

    #[test]
    fn bytes() {
        compile_and_run(
            vec!["1".into()],
            ToFloat,
            TypeDef::float().fallible(),
            Ok(1.0.into()),
        );

        compile_and_run(
            vec!["1.2".into()],
            ToFloat,
            TypeDef::float().fallible(),
            Ok(1.2.into()),
        )
    }
}
