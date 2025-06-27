use std::borrow::Cow;
use std::hash::{Hash, Hasher};

use regex::{Captures, Regex};
use value::{Kind, Value};

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Spanned, TypeDef};
use crate::context::Context;

pub struct Redact;

impl Function for Redact {
    fn identifier(&self) -> &'static str {
        "redact"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "value",
                kind: Kind::REDACTABLE,
                required: true,
            },
            Parameter {
                name: "filters",
                kind: Kind::ARRAY,
                required: true,
            },
            Parameter {
                name: "redactor",
                kind: Kind::BYTES,
                required: false,
            },
        ]
    }

    fn compile(&self, mut arguments: ArgumentList) -> Result<FunctionCall, SyntaxError> {
        let value = arguments.get();
        let Spanned { node, span } = arguments.get();
        let Expr::Array(array) = node else {
            return Err(SyntaxError::InvalidFunctionArgumentType {
                function: "redact",
                argument: "filters",
                want: Kind::ARRAY,
                got: Kind::UNDEFINED, // TODO: fix this
                span,
            });
        };

        let filters = array
            .into_iter()
            .map(|Spanned { node, span }| {
                let Expr::String(value) = node else {
                    return Err(SyntaxError::InvalidType {
                        want: "regex pattern".to_string(),
                        got: "".to_string(),
                        span,
                    });
                };

                Regex::new(String::from_utf8_lossy(&value).as_ref()).map_err(|err| {
                    SyntaxError::InvalidValue {
                        err: err.to_string(),
                        want: "valid regex pattern".to_string(),
                        got: "".to_string(),
                        span,
                    }
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let redactor = match arguments.get_string_opt()? {
            Some(Spanned { node, .. }) => match node.as_str() {
                "full" => Redactor::Full,
                "hash" => Redactor::Hash,
                _ => Redactor::Text(node),
            },
            None => Redactor::Full,
        };

        Ok(FunctionCall {
            function: Box::new(RedactFunc {
                value,
                filters,
                redactor,
            }),
        })
    }
}

#[derive(Clone)]
struct RedactFunc {
    value: Spanned<Expr>,
    filters: Vec<Regex>,
    redactor: Redactor,
}

impl Expression for RedactFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = self.value.resolve(cx)?;

        Ok(redact(value, &self.filters, &self.redactor))
    }

    fn type_def(&self, state: &TypeState) -> TypeDef {
        self.value.type_def(state)
    }
}

const REDACTED: &str = "[REDACTED]";

#[derive(Clone)]
enum Redactor {
    Full, // [REDACTED]
    Text(String),
    Hash,
}

impl regex::Replacer for &Redactor {
    fn replace_append(&mut self, caps: &Captures<'_>, dst: &mut String) {
        match self {
            Redactor::Full => {
                dst.push_str(REDACTED);
            }
            Redactor::Text(s) => {
                dst.push_str(s);
            }
            Redactor::Hash => {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                let _ = &caps[0].hash(&mut hasher);
                let hv = hasher.finish();

                dst.push_str(&format!("{hv:x}"));
            }
        }
    }

    fn no_expansion(&mut self) -> Option<Cow<str>> {
        match self {
            Redactor::Full => Some(REDACTED.into()),
            Redactor::Text(s) => Some(s.into()),
            Redactor::Hash => None,
        }
    }
}

fn redact(value: Value, filters: &[Regex], redactor: &Redactor) -> Value {
    match value {
        Value::Bytes(value) => {
            let input = String::from_utf8_lossy(&value);
            let output = filters.iter().fold(input, |input, filter| {
                filter
                    .replace_all(input.as_ref(), redactor)
                    .into_owned()
                    .into()
            });

            Value::Bytes(output.into_owned().into())
        }
        Value::Array(values) => {
            let values = values
                .into_iter()
                .map(|value| redact(value, filters, redactor))
                .collect();

            Value::Array(values)
        }
        Value::Object(map) => {
            let map = map
                .into_iter()
                .map(|(key, value)| (key, redact(value, filters, redactor)))
                .collect();

            Value::Object(map)
        }
        _ => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::function::compile_and_run;

    const PATTERN: &str = r#"[0-9]{6}"#;

    #[test]
    fn replace() {
        let input = "hello 123456 world";
        let re = Regex::new(PATTERN).unwrap();
        let redactor = Redactor::Full;

        let result = re.replace_all(input, &redactor);
        assert_eq!(result, "hello [REDACTED] world");
    }

    #[test]
    fn full() {
        compile_and_run(
            vec![
                "hello 123456 world".into(),
                Expr::Array(vec![PATTERN.into()]),
            ],
            Redact,
            TypeDef::bytes(),
            Ok(Value::Bytes("hello [REDACTED] world".into())),
        )
    }

    #[test]
    fn text() {
        compile_and_run(
            vec![
                "hello 123456 world".into(),
                Expr::Array(vec![PATTERN.into()]),
                "******".into(),
            ],
            Redact,
            TypeDef::bytes(),
            Ok(Value::Bytes("hello ****** world".into())),
        )
    }

    #[test]
    fn hash() {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        "123456".hash(&mut hasher);
        let hv = hasher.finish();
        let redacted = format!("{hv:x}");

        compile_and_run(
            vec![
                "hello 123456 world".into(),
                Expr::Array(vec![PATTERN.into()]),
                "hash".into(),
            ],
            Redact,
            TypeDef::bytes(),
            Ok(Value::Bytes(format!("hello {redacted} world").into())),
        )
    }
}
