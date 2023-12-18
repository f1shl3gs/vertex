use chrono::{TimeZone, Utc};
use value::Value;

use super::to_unix_timestamp::Unit;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::context::Context;
use crate::SyntaxError;

pub struct FromUnixTimestamp;

impl Function for FromUnixTimestamp {
    fn identifier(&self) -> &'static str {
        "from_unix_timestamp"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "value",
                kind: Kind::INTEGER,
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
            Some(value) => value.try_into()?,
            None => Unit::Seconds,
        };

        Ok(FunctionCall {
            function: Box::new(FromUnixTimestampFunc { value, unit }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct FromUnixTimestampFunc {
    value: Spanned<Expr>,
    unit: Unit,
}

impl Expression for FromUnixTimestampFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        match self.value.resolve(cx)? {
            Value::Integer(i) => {
                let timestamp = match self.unit {
                    Unit::Seconds => match Utc.timestamp_opt(i, 0).single() {
                        Some(ts) => ts,
                        None => {
                            return Err(ExpressionError::UnexpectedValue {
                                msg: "invalid unix timestamp in seconds".to_string(),
                                span: self.value.span,
                            })
                        }
                    },
                    Unit::Milliseconds => match Utc.timestamp_millis_opt(i).single() {
                        Some(ts) => ts,
                        None => {
                            return Err(ExpressionError::UnexpectedValue {
                                msg: "invalid unix timestamp in milliseconds".to_string(),
                                span: self.value.span,
                            })
                        }
                    },
                    Unit::Microseconds => match Utc.timestamp_micros(i).single() {
                        Some(ts) => ts,
                        None => {
                            return Err(ExpressionError::UnexpectedValue {
                                msg: "invalid unix timestamp in microseconds".to_string(),
                                span: self.value.span,
                            })
                        }
                    },
                    Unit::Nanoseconds => Utc.timestamp_nanos(i),
                };

                Ok(timestamp.into())
            }
            value => Err(ExpressionError::UnexpectedType {
                want: Kind::INTEGER,
                got: value.kind(),
                span: self.value.span,
            }),
        }
    }

    #[inline]
    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef {
            fallible: true,
            kind: Kind::TIMESTAMP,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn default_unit() {
        compile_and_run(
            vec![1_431_648_000.into()],
            FromUnixTimestamp,
            TypeDef::timestamp().fallible(),
            Ok(Utc.with_ymd_and_hms(2015, 5, 15, 0, 0, 0).unwrap().into()),
        )
    }

    #[test]
    fn seconds() {
        compile_and_run(
            vec![1_609_459_200_i64.into(), "seconds".into()],
            FromUnixTimestamp,
            TypeDef::timestamp().fallible(),
            Ok(Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap().into()),
        )
    }

    #[test]
    fn milliseconds() {
        compile_and_run(
            vec![1_609_459_200_000_i64.into(), "milliseconds".into()],
            FromUnixTimestamp,
            TypeDef::timestamp().fallible(),
            Ok(Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap().into()),
        )
    }

    #[test]
    fn microseconds() {
        compile_and_run(
            vec![1_609_459_200_000_000_i64.into(), "microseconds".into()],
            FromUnixTimestamp,
            TypeDef::timestamp().fallible(),
            Ok(Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap().into()),
        )
    }

    #[test]
    fn nanoseconds() {
        compile_and_run(
            vec![1_609_459_200_000_000_000_i64.into(), "nanoseconds".into()],
            FromUnixTimestamp,
            TypeDef::timestamp().fallible(),
            Ok(Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap().into()),
        )
    }
}
