use std::collections::BTreeMap;

use url::form_urlencoded;
use value::Value;

use crate::compiler::function::{ArgumentList, Function, FunctionCompileContext, Parameter};
use crate::compiler::function_call::FunctionCall;
use crate::compiler::parser::Expr;
use crate::compiler::{Expression, ExpressionError, Kind, Spanned, TypeDef, ValueKind};
use crate::context::Context;
use crate::SyntaxError;

pub struct ParseQuery;

impl Function for ParseQuery {
    fn identifier(&self) -> &'static str {
        "parse_query"
    }

    fn parameters(&self) -> &'static [Parameter] {
        &[Parameter {
            name: "value",
            kind: Kind::BYTES,
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
            function: Box::new(ParseQueryFunc { value }),
            span: cx.span,
        })
    }
}

#[derive(Clone)]
struct ParseQueryFunc {
    value: Spanned<Expr>,
}

impl Expression for ParseQueryFunc {
    fn resolve(&self, cx: &mut Context) -> Result<Value, ExpressionError> {
        match self.value.resolve(cx)? {
            Value::Bytes(b) => {
                let slice = b.as_ref();
                let slice = match slice.first() {
                    Some(b'?') => &slice[1..],
                    Some(_ch) => slice,
                    None => {
                        // empty slice
                        return Ok(Value::Object(BTreeMap::new()));
                    }
                };

                let parsed = form_urlencoded::parse(slice);
                let mut result = BTreeMap::new();
                for (k, v) in parsed {
                    let value = v.as_ref();
                    result
                        .entry(k.into_owned())
                        .and_modify(|v| match v {
                            Value::Array(array) => {
                                array.push(value.into());
                            }
                            v => *v = Value::Array(vec![v.clone(), value.into()]),
                        })
                        .or_insert_with(|| value.into());
                }

                Ok(Value::Object(result))
            }
            value => Err(ExpressionError::UnexpectedType {
                want: Kind::BYTES,
                got: value.kind(),
                span: self.value.span,
            }),
        }
    }

    fn type_def(&self) -> TypeDef {
        TypeDef {
            fallible: true,
            kind: Kind::OBJECT,
        }
    }
}

#[cfg(test)]
mod tests {
    use value::value;

    use super::*;
    use crate::compiler::function::compile_and_run;

    #[test]
    fn complete() {
        compile_and_run(
            vec!["foo=%2B1&bar=2&xyz=&abc".into()],
            ParseQuery,
            TypeDef::object().fallible(),
            Ok(value!({
                "foo": "+1",
                "bar": "2",
                "xyz": "",
                "abc": ""
            })),
        )
    }

    #[test]
    fn multiple_values() {
        compile_and_run(
            vec!["foo=bar&foo=abc".into()],
            ParseQuery,
            TypeDef::object().fallible(),
            Ok(value!({
                "foo": ["bar", "abc"]
            })),
        )
    }

    #[test]
    fn multiple_values_with_question_mark() {
        compile_and_run(
            vec!["?foo%5b%5d=bar&foo%5b%5d=xyz".into()],
            ParseQuery,
            TypeDef::object().fallible(),
            Ok(value!({
                "foo[]": ["bar","xyz"]
            })),
        )
    }
}
