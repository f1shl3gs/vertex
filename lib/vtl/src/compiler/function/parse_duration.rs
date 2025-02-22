use std::time::Duration;

use value::{Kind, Value};

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Spanned, TypeDef};
use crate::context::Context;

pub struct ParseDuration;

impl Function for ParseDuration {
    fn identifier(&self) -> &'static str {
        "parse_duration"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "value",
                kind: Kind::BYTES,
                required: true,
            },
            Parameter {
                name: "unit",
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
        let value = arguments.get();
        let Spanned { node, span } = arguments.get_string()?;
        let unit = match node.as_str() {
            "ns" => Duration::from_nanos(1),
            "us" => Duration::from_micros(1),
            "µs" => Duration::from_micros(1),
            "ms" => Duration::from_millis(1),
            "s" => Duration::from_secs(1),
            "m" => Duration::from_secs(60),
            "h" => Duration::from_secs(3600),
            "d" => Duration::from_secs(86_400),
            "w" => Duration::from_secs(86_400 * 7),
            unit => {
                return Err(SyntaxError::InvalidValue {
                    err: "invalid time unit".to_string(),
                    want: "ns, us, µs, ms, s, m, h or d".to_string(),
                    got: unit.to_string(),
                    span,
                });
            }
        };

        Ok(FunctionCall {
            function: Box::new(ParseDurationFunc { value, unit }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct ParseDurationFunc {
    value: Spanned<Expr>,
    unit: Duration,
}

impl Expression for ParseDurationFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = self.value.resolve(cx)?;
        let Value::Bytes(value) = value else {
            return Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: value.kind(),
                span: self.value.span,
            });
        };

        let s = String::from_utf8_lossy(&value);
        let d = humanize::duration::parse_duration(s.as_ref()).map_err(|err| {
            ExpressionError::UnexpectedValue {
                msg: format!("invalid duration {}, {}", s, err),
                span: self.value.span,
            }
        })?;

        Ok(Value::Float(d.div_duration_f64(self.unit)))
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef::float().fallible()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn call() {
        for (input, unit, want) in [
            ("30s", "m", 0.5),
            ("100ms", "ms", 100.0),
            ("1005ms", "s", 1.005),
            ("100ns", "ms", 0.0001),
            ("100us", "ms", 0.1),
            ("100µs", "ms", 0.1),
            ("1d", "s", 86400.0),
            ("1d1s", "s", 86401.0),
            ("1s1ms", "ms", 1001.0),
            ("1ms1us", "ms", 1.001),
            ("1s", "ns", 1_000_000_000.0),
            ("1us", "ms", 0.001),
            ("1w", "ns", 604_800_000_000_000.0),
        ] {
            compile_and_run(
                vec![input.into(), unit.into()],
                ParseDuration,
                TypeDef::float().fallible(),
                Ok(Value::Float(want)),
            )
        }
    }
}
