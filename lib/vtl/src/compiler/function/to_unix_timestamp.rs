use value::Value;

use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::Expr;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::{Context, SyntaxError};

pub struct ToUnixTimestamp;

impl Function for ToUnixTimestamp {
    fn identifier(&self) -> &'static str {
        "to_unix_timestamp"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "timestamp",
                kind: Kind::TIMESTAMP,
                required: true,
            },
            Parameter {
                name: "unit",
                kind: Kind::BYTES,
                required: false,
            },
        ]
    }

    fn compile(
        &self,
        cx: FunctionCompileContext,
        mut arguments: ArgumentList,
    ) -> Result<FunctionCall, SyntaxError> {
        let value = arguments.get();
        let unit = match arguments.get_string_opt()? {
            Some(s) => match s.as_str() {
                "s" | "seconds" => Unit::Seconds,
                "milliseconds" => Unit::Milliseconds,
                "microseconds" => Unit::Microseconds,
                "ns" | "nanoseconds" => Unit::Nanoseconds,
                _ => {
                    return Err(SyntaxError::InvalidValue {
                        err: "".to_string(),
                        want: "seconds, milliseconds, microseconds or nanoseconds".to_string(),
                        got: s.node,
                        span: s.span,
                    })
                }
            },
            None => Unit::Seconds,
        };

        Ok(FunctionCall {
            function: Box::new(ToUnixTimestampFunc { value, unit }),
            span: cx.span,
        })
    }
}

pub enum Unit {
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
}

struct ToUnixTimestampFunc {
    value: Spanned<Expr>,
    unit: Unit,
}

impl Expression for ToUnixTimestampFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let ts = match self.value.resolve(cx)? {
            Value::Timestamp(ts) => ts,
            value => {
                return Err(ExpressionError::UnexpectedType {
                    want: Kind::TIMESTAMP,
                    got: value.kind(),
                    span: self.value.span,
                })
            }
        };

        let ts = match self.unit {
            Unit::Seconds => ts.timestamp(),
            Unit::Milliseconds => ts.timestamp_millis(),
            Unit::Microseconds => ts.timestamp_micros(),
            Unit::Nanoseconds => ts.timestamp_nanos_opt().expect("should always be ok"),
        };

        Ok(Value::Integer(ts))
    }

    fn type_def(&self) -> TypeDef {
        TypeDef {
            fallible: false,
            kind: Kind::INTEGER,
        }
    }
}

#[cfg(test)]
mod tests {
    use value::parse_target_path;

    use super::*;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn seconds() {
        compile_and_run(
            vec![parse_target_path(".timestamp").unwrap().into()],
            ToUnixTimestamp,
            TypeDef::integer(),
            Ok(1_609_459_200_i64.into()),
        );

        compile_and_run(
            vec![parse_target_path(".timestamp").unwrap().into(), "s".into()],
            ToUnixTimestamp,
            TypeDef::integer(),
            Ok(1_609_459_200_i64.into()),
        );

        compile_and_run(
            vec![
                parse_target_path(".timestamp").unwrap().into(),
                "seconds".into(),
            ],
            ToUnixTimestamp,
            TypeDef::integer(),
            Ok(1_609_459_200_i64.into()),
        );
    }

    #[test]
    fn milliseconds() {
        compile_and_run(
            vec![
                parse_target_path(".timestamp").unwrap().into(),
                "milliseconds".into(),
            ],
            ToUnixTimestamp,
            TypeDef::integer(),
            Ok(1_609_459_200_000i64.into()),
        )
    }

    #[test]
    fn microseconds() {
        compile_and_run(
            vec![
                parse_target_path(".timestamp").unwrap().into(),
                "microseconds".into(),
            ],
            ToUnixTimestamp,
            TypeDef::integer(),
            Ok(1_609_459_200_000_000_i64.into()),
        )
    }

    #[test]
    fn nanoseconds() {
        compile_and_run(
            vec![
                parse_target_path(".timestamp").unwrap().into(),
                "nanoseconds".into(),
            ],
            ToUnixTimestamp,
            TypeDef::integer(),
            Ok(1_609_459_200_000_000_000_i64.into()),
        );

        compile_and_run(
            vec![parse_target_path(".timestamp").unwrap().into(), "ns".into()],
            ToUnixTimestamp,
            TypeDef::integer(),
            Ok(1_609_459_200_000_000_000_i64.into()),
        );
    }
}
