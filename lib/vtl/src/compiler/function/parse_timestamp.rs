use chrono::{DateTime, Utc};
use value::Value;

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef};
use crate::context::Context;

pub struct ParseTimestamp;

impl Function for ParseTimestamp {
    fn identifier(&self) -> &'static str {
        "parse_timestamp"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "value",
                kind: Kind::BYTES,
                required: true,
            },
            Parameter {
                name: "format",
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
        let format = arguments.get_string()?.node;

        Ok(FunctionCall {
            function: Box::new(ParseTimestampFunc { value, format }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct ParseTimestampFunc {
    value: Spanned<Expr>,
    format: String,
}

impl Expression for ParseTimestampFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        match self.value.resolve(cx)? {
            Value::Bytes(b) => {
                let ts = DateTime::parse_from_str(
                    String::from_utf8_lossy(&b).as_ref(),
                    self.format.as_str(),
                )
                .map_err(|err| ExpressionError::UnexpectedValue {
                    msg: err.to_string(),
                    span: self.value.span,
                })?
                .with_timezone(&Utc);

                Ok(Value::Timestamp(ts))
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
    fn parse_timestamp() {
        let format = "%d/%m/%Y:%H:%M:%S %z";
        let date = DateTime::parse_from_rfc2822("Wed, 16 Oct 2019 12:00:00 +0000")
            .unwrap()
            .with_timezone(&Utc);
        let formatted = date.format(format).to_string();
        compile_and_run(
            vec![formatted.into(), format.into()],
            ParseTimestamp,
            TypeDef::timestamp().fallible(),
            Ok(date.into()),
        )
    }

    #[test]
    fn parse_text() {
        let format = "%d/%m/%Y:%H:%M:%S %z";
        let date = DateTime::parse_from_rfc2822("Wed, 16 Oct 2019 12:00:00 +0000")
            .unwrap()
            .with_timezone(&Utc);
        let formatted = date.format(format).to_string();
        compile_and_run(
            vec![formatted.into(), format.into()],
            ParseTimestamp,
            TypeDef::timestamp().fallible(),
            Ok(date.into()),
        )
    }

    // #[test]
    // fn parse_text_with_tz() {
    //     let format = "%d/%m/%Y:%H:%M:%S";
    //     let date =  DateTime::parse_from_rfc2822("Wed, 16 Oct 2019 10:00:00 +0000").unwrap()
    //         .with_timezone(&Utc);
    //     let formatted = date.format(format).to_string();
    //     compile_and_run(
    //         vec![
    //             formatted.into(),
    //             format.into()
    //         ],
    //         ParseTimestamp,
    //         Ok(date.into())
    //     )
    // }
}
