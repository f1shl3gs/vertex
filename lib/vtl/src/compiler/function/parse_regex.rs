use std::collections::BTreeMap;

use regex::Regex;
use value::{Kind, Value};

use crate::SyntaxError;
use crate::compiler::expr::Expr;
use crate::compiler::function::{ArgumentList, Function, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::state::TypeState;
use crate::compiler::{Expression, ExpressionError, Spanned, TypeDef};
use crate::context::Context;

pub struct ParseRegex;

impl Function for ParseRegex {
    fn identifier(&self) -> &'static str {
        "parse_regex"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[
            Parameter {
                name: "value",
                kind: Kind::BYTES,
                required: true,
            },
            Parameter {
                name: "pattern",
                kind: Kind::BYTES,
                required: true,
            },
        ]
    }

    fn compile(&self, mut arguments: ArgumentList) -> Result<FunctionCall, SyntaxError> {
        let value = arguments.get();
        let Spanned { node, span } = arguments.get_string()?;
        let pattern = Regex::new(&node).map_err(|err| SyntaxError::InvalidValue {
            err: format!("parse regex pattern failed, {err}"),
            want: "valid regex".to_string(),
            got: node,
            span,
        })?;

        Ok(FunctionCall {
            function: Box::new(ParseRegexFunc {
                value,
                pattern: Spanned::new(pattern, span),
            }),
        })
    }
}

#[derive(Clone)]
struct ParseRegexFunc {
    value: Spanned<Expr>,
    pattern: Spanned<Regex>,
}

impl Expression for ParseRegexFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        let value = self.value.resolve(cx)?;
        let Value::Bytes(data) = value else {
            return Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: value.kind(),
                span: self.value.span,
            });
        };

        let input = String::from_utf8_lossy(&data);
        let captures =
            self.pattern
                .captures(&input)
                .ok_or_else(|| ExpressionError::UnexpectedValue {
                    msg: "value do not match regex".to_string(),
                    span: self.value.span,
                })?;

        let map = self
            .pattern
            .capture_names()
            .flatten()
            .map(|name| {
                let key = name.to_string();
                let value = match captures.name(name).map(|s| s.as_str().to_string()) {
                    Some(v) => Value::Bytes(v.into()),
                    None => Value::Null,
                };

                (key, value)
            })
            .collect::<BTreeMap<_, _>>();

        Ok(map.into())
    }

    fn type_def(&self, _state: &TypeState) -> TypeDef {
        TypeDef::object().fallible()
    }
}

#[cfg(test)]
mod tests {
    use value::value;

    use super::*;
    use crate::compiler::Span;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn single_match() {
        compile_and_run(
            vec![
                "first group and second group".into(),
                "(?P<number>.*?) group".into(),
            ],
            ParseRegex,
            TypeDef::object().fallible(),
            Ok(value!({
                "number": "first",
            })),
        )
    }

    #[test]
    fn no_match() {
        compile_and_run(
            vec![
                "i don't match".into(),
                r##"^(?P<host>[\\w\\.]+) - (?P<user>[\\w]+) (?P<bytes_in>[\\d]+) \\[(?P<timestamp>.*)\\] \"(?P<method>[\\w]+) (?P<path>.*)\" (?P<status>[\\d]+) (?P<bytes_out>[\\d]+)$"##.into()
            ],
            ParseRegex,
            TypeDef::object().fallible(),
            Err(ExpressionError::UnexpectedValue {
                msg: "value do not match regex".to_string(),
                span: Span::empty(),
            })
        )
    }

    #[test]
    fn ok() {
        compile_and_run(
            vec![
                "5.86.210.12 - zieme4647 5667 [19/06/2019:17:20:49 -0400] \"GET /embrace/supply-chains/dynamic/vertical\" 201 20574".into(),
                r##"^(?P<host>[\\w\\.]+) - (?P<user>[\\w]+) (?P<bytes_in>[\\d]+) \\[(?P<timestamp>.*)\\] \"(?P<method>[\\w]+) (?P<path>.*)\" (?P<status>[\\d]+) (?P<bytes_out>[\\d]+)$"##.into()
            ],
            ParseRegex,
            TypeDef::object().fallible(),
            Ok(value!({
                "bytes_in": "5667",
                "host": "5.86.210.12",
                "user": "zieme4647",
                "timestamp": "19/06/2019:17:20:49 -0400",
                "method": "GET",
                "path": "/embrace/supply-chains/dynamic/vertical",
                "status": "201",
                "bytes_out": "20574"
            }))
        )
    }
}
