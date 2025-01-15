use value::{Kind, Value};

use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Spanned, TypeDef};
use crate::context::Context;
use crate::SyntaxError;

// ICE Sizes, kibis of bits
const BYTE: usize = 1;
const KIBYTE: usize = 1 << 10;
const MIBYTE: usize = 1 << (2 * 10);
const GIBYTE: usize = 1 << (3 * 10);
const TIBYTE: usize = 1 << (4 * 10);
const PIBYTE: usize = 1 << (5 * 10);
const EIBYTE: usize = 1 << (6 * 10);

// SI Sizes
const IBYTE: usize = 1;
const KBYTE: usize = IBYTE * 1000;
const MBYTE: usize = KBYTE * 1000;
const GBYTE: usize = MBYTE * 1000;
const TBYTE: usize = GBYTE * 1000;
const PBYTE: usize = TBYTE * 1000;
const EBYTE: usize = PBYTE * 1000;

pub struct ParseBytes;

impl Function for ParseBytes {
    fn identifier(&self) -> &'static str {
        "parse_bytes"
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
            Some(Spanned { node, span }) => {
                let value = match node.as_str() {
                    "b" | "" => BYTE,
                    "kib" | "ki" => KIBYTE,
                    "kb" | "k" => KBYTE,
                    "mib" | "mi" => MIBYTE,
                    "mb" | "m" => MBYTE,
                    "gib" | "gi" => GIBYTE,
                    "gb" | "g" => GBYTE,
                    "tib" | "ti" => TIBYTE,
                    "tb" | "t" => TBYTE,
                    "pib" | "pi" => PIBYTE,
                    "pb" | "p" => PBYTE,
                    "eib" | "ei" => EIBYTE,
                    "eb" | "e" => EBYTE,
                    unit => {
                        return Err(SyntaxError::InvalidValue {
                            err: format!("invalid unit {}", unit),
                            want: "b, k, kib, m, mib...".to_string(),
                            got: unit.to_string(),
                            span,
                        })
                    }
                };

                Some(value)
            }
            None => Some(1),
        };

        Ok(FunctionCall {
            function: Box::new(ParseBytesFunc { value, unit }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct ParseBytesFunc {
    value: Spanned<Expr>,
    unit: Option<usize>,
}

impl Expression for ParseBytesFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = self.value.resolve(cx)?;
        let Value::Bytes(value) = value else {
            return Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: value.kind(),
                span: self.value.span,
            });
        };

        let bytes = humanize::bytes::parse_bytes(String::from_utf8_lossy(&value).as_ref())
            .map_err(|err| ExpressionError::UnexpectedValue {
                msg: format!("parse bytes failed, {}", err),
                span: self.value.span,
            })?;

        let value = if let Some(unit) = self.unit {
            bytes as f64 / unit as f64
        } else {
            bytes as f64
        };

        Ok(Value::Float(value))
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
    fn mib() {
        compile_and_run(
            vec!["1MiB".into()],
            ParseBytes,
            TypeDef::float().fallible(),
            Ok(Value::Float(1_048_576.0)),
        )
    }

    #[test]
    fn b() {
        compile_and_run(
            vec!["512B".into()],
            ParseBytes,
            TypeDef::float().fallible(),
            Ok(Value::Float(512.0)),
        )
    }

    #[test]
    fn gib() {
        compile_and_run(
            vec!["3.5GiB".into()],
            ParseBytes,
            TypeDef::float().fallible(),
            Ok(Value::Float(3_670_016.0 * 1024.0)),
        )
    }

    #[test]
    fn gib_to_mib() {
        compile_and_run(
            vec!["3.5GiB".into(), "mib".into()],
            ParseBytes,
            TypeDef::float().fallible(),
            Ok(Value::Float(3.5 * 1024.0)),
        )
    }
}
